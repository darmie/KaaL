//! Root task creation and EL0 transition
//!
//! Creates the first userspace task (root task) and transitions to EL0.

use crate::arch::aarch64::page_table::PageTableFlags;
use crate::boot::bootinfo;
use crate::memory::{PhysAddr, VirtAddr, PAGE_SIZE};
use core::arch::naked_asm;

/// Root task creation error
#[derive(Debug)]
pub enum RootTaskError {
    /// Boot info not available
    BootInfoNotAvailable,
    /// Invalid boot info (zero addresses or sizes)
    InvalidBootInfo,
    /// Failed to allocate page table
    PageTableAllocation,
    /// Failed to map memory
    MemoryMapping,
}

/// Create and start the root task in EL0
///
/// This function:
/// 1. Creates a user page table with identity mapping
/// 2. Maps root task code/data into user address space
/// 3. Transitions to EL0 and jumps to root task entry point
///
/// # Safety
/// Must be called after kernel initialization with valid boot info.
pub unsafe fn create_and_start_root_task() -> Result<(), RootTaskError> {
    // Get boot info
    let boot_info = bootinfo::get_boot_info()
        .ok_or(RootTaskError::BootInfoNotAvailable)?;

    // Validate boot info
    if !boot_info.is_valid() {
        return Err(RootTaskError::InvalidBootInfo);
    }

    crate::kprintln!("[root_task] Creating root task:");
    crate::kprintln!("  Root task image: {:#x} - {:#x} ({} KB)",
                   boot_info.root_task_start.as_usize(),
                   boot_info.root_task_end.as_usize(),
                   boot_info.root_task_size() / 1024);
    crate::kprintln!("  Entry point:     {:#x}", boot_info.root_task_entry);

    // Step 1: Create user page table
    let user_page_table_frame = crate::memory::alloc_frame()
        .ok_or(RootTaskError::PageTableAllocation)?;

    let user_page_table_phys = user_page_table_frame.phys_addr();
    let user_page_table = &mut *(user_page_table_phys.as_usize()
        as *mut crate::arch::aarch64::page_table::PageTable);
    user_page_table.zero();

    crate::kprintln!("  User page table: {:#x}", user_page_table_phys.as_usize());

    // Create page mapper for user page table
    let mut mapper = crate::memory::PageMapper::new(user_page_table);

    // Step 1.5: Map kernel into user page table for syscall handling
    // This is necessary because syscall handlers run in kernel mode but with
    // user's page table (TTBR0). Kernel code/data must be accessible.
    extern "C" {
        static _kernel_start: u8;
        static _kernel_end: u8;
    }
    let kernel_start_addr = &_kernel_start as *const u8 as usize;
    let kernel_end_addr = &_kernel_end as *const u8 as usize;

    crate::kprintln!("  Mapping kernel in user PT: {:#x} - {:#x}",
        kernel_start_addr, kernel_end_addr);

    // SECURITY NOTE: Mapping kernel into user page table creates security risks:
    // - Meltdown-style attacks could leak kernel data
    // - Kernel addresses become predictable (KASLR bypass)
    // TODO Chapter 9 Phase 2: Implement proper page table switching in exception handler
    crate::memory::paging::identity_map_region(
        &mut mapper,
        kernel_start_addr,
        kernel_end_addr - kernel_start_addr,
        PageTableFlags::KERNEL_RWX,
    ).map_err(|_| RootTaskError::MemoryMapping)?;

    // Also map UART for kprintln! in syscalls
    crate::memory::paging::identity_map_region(
        &mut mapper,
        0x9000000,
        PAGE_SIZE,
        PageTableFlags::KERNEL_DEVICE,
    ).map_err(|_| RootTaskError::MemoryMapping)?;

    // Step 2: Map root task memory (code + data + rodata)
    // IMPORTANT: The elfloader embeds the entire ELF file (including headers),
    // but the entry point expects to start at the first LOAD segment.
    // ELF headers are typically 0x1000 bytes, so we need to skip them.

    let phys_file_start = boot_info.root_task_start.as_usize();
    let phys_end = boot_info.root_task_end.as_usize();
    let binary_size = phys_end - phys_file_start;

    // Virtual address: where the linker script expects it
    let entry_addr = boot_info.root_task_entry;
    let virt_start = entry_addr & !(PAGE_SIZE - 1); // Align down to page boundary

    // Parse the ELF header to find the file offset of the first LOAD segment
    // ELF64 header is 64 bytes, followed by program headers
    let elf_base = phys_file_start as *const u8;
    let e_phoff = unsafe { *(elf_base.add(32) as *const u64) }; // Program header offset
    let e_phnum = unsafe { *(elf_base.add(56) as *const u16) }; // Number of program headers

    // Find the first LOAD segment (PT_LOAD = 1)
    let phdr_size = 56; // Size of ELF64 program header
    let mut code_file_offset = 0;
    for i in 0..e_phnum {
        let phdr_addr = (elf_base as usize + e_phoff as usize + (i as usize * phdr_size)) as *const u32;
        let p_type = unsafe { *phdr_addr };
        if p_type == 1 { // PT_LOAD
            code_file_offset = unsafe { *((phdr_addr as usize + 8) as *const u64) };
            break;
        }
    }

    let phys_start = phys_file_start + code_file_offset as usize;

    // Map enough pages to cover the entire binary
    let code_size = ((binary_size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)) + PAGE_SIZE * 4;

    // Map virtual address → physical address (skip ELF headers)
    let mut current_virt = virt_start;
    let mut current_phys = phys_start;
    let end_virt = virt_start + code_size;

    while current_virt < end_virt && current_phys < phys_end + PAGE_SIZE * 4 {
        mapper.map(
            VirtAddr::new(current_virt),
            PhysAddr::new(current_phys),
            PageTableFlags::USER_RWX,
            crate::memory::PageSize::Size4KB,
        ).map_err(|_| RootTaskError::MemoryMapping)?;

        current_virt += PAGE_SIZE;
        current_phys += PAGE_SIZE;
    }

    // Map stack (1MB below entry point, 256KB size)
    let stack_top = entry_addr - PAGE_SIZE;
    let stack_size = 256 * 1024; // 256KB stack
    let stack_bottom = stack_top - stack_size;

    crate::memory::paging::identity_map_region(
        &mut mapper,
        stack_bottom,
        stack_size,
        PageTableFlags::USER_DATA,
    ).map_err(|_| RootTaskError::MemoryMapping)?;

    // Map UART for syscalls (device memory)
    let uart_base = 0x09000000;
    crate::memory::paging::identity_map_region(
        &mut mapper,
        uart_base,
        4096,
        PageTableFlags::USER_DEVICE,
    ).map_err(|_| RootTaskError::MemoryMapping)?;

    crate::kprintln!("  Entry point:     {:#x}", entry_addr);
    crate::kprintln!("  Stack:           {:#x} - {:#x} (256 KB)", stack_bottom, stack_top);
    crate::kprintln!("  Code segment:    {} KB at virt {:#x}", code_size / 1024, virt_start);
    crate::kprintln!("  ✓ Root task ready for EL0 transition");

    // Step 3: Transition to EL0
    crate::kprintln!("");
    crate::kprintln!("[root_task] Transitioning to EL0...");
    crate::kprintln!("");

    transition_to_el0(entry_addr, stack_top, user_page_table_phys.as_usize());

    // This should never be reached
    Ok(())
}

/// Transition to EL0 and jump to user code
///
/// # Safety
/// Must be called with valid entry point, stack, and page table.
#[unsafe(naked)]
unsafe extern "C" fn transition_to_el0(entry: usize, sp: usize, ttbr0: usize) -> ! {
    naked_asm!(
        // Set up TTBR0_EL1 (user page table)
        "msr ttbr0_el1, x2",
        "isb",

        // Set up ELR_EL1 (return address = user entry point)
        "msr elr_el1, x0",

        // Set up SP_EL0 (user stack pointer)
        "msr sp_el0, x1",

        // Set up SPSR_EL1 for EL0
        // - Mode: EL0t (0b0000)
        // - All interrupt bits cleared (D=0, A=0, I=0, F=0) - interrupts enabled
        "mov x3, #0",
        "msr spsr_el1, x3",

        // Perform exception return to EL0
        "eret",
    )
}
