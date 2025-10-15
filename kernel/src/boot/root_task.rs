//! Root task creation and EL0 transition
//!
//! Creates the first userspace task (root task) and transitions to EL0.

use crate::arch::aarch64::page_table::PageTableFlags;
use crate::boot::{bootinfo, boot_info};
use crate::memory::{PhysAddr, VirtAddr, PAGE_SIZE};
use crate::generated::memory_config;
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
    /// Failed to create userspace boot info
    BootInfoCreation,
}

/// Populate the boot info structure for userspace runtime services
///
/// This function creates and populates a BootInfo structure with:
/// - Untyped memory regions (available physical memory)
/// - Device MMIO regions (UART, RTC, Timer, etc.)
/// - Initial capability slots
/// - System configuration
unsafe fn populate_boot_info() -> Result<boot_info::BootInfo, RootTaskError> {
    let mut info = boot_info::BootInfo::new();

    // Set system configuration
    info.ram_size = memory_config::RAM_SIZE;
    info.kernel_virt_base = memory_config::KERNEL_BASE as u64;
    info.user_virt_start = memory_config::USER_VIRT_START;
    info.ipc_buffer_vaddr = 0x8000_0000; // Fixed IPC buffer location

    // TODO: Set capability slots when CSpace is implemented
    info.cspace_root_slot = 0; // Placeholder
    info.vspace_root_slot = 0; // Placeholder

    // Add device regions from platform configuration
    info.add_device_region(boot_info::DeviceRegion {
        paddr: memory_config::UART0_BASE,
        size: 0x1000,
        device_type: memory_config::DEVICE_UART0 as u32,
        irq: 33, // QEMU virt UART0 IRQ
    }).map_err(|_| RootTaskError::BootInfoCreation)?;

    info.add_device_region(boot_info::DeviceRegion {
        paddr: memory_config::UART1_BASE,
        size: 0x1000,
        device_type: memory_config::DEVICE_UART1 as u32,
        irq: 34, // QEMU virt UART1 IRQ
    }).map_err(|_| RootTaskError::BootInfoCreation)?;

    info.add_device_region(boot_info::DeviceRegion {
        paddr: memory_config::RTC_BASE,
        size: 0x1000,
        device_type: memory_config::DEVICE_RTC as u32,
        irq: 0xFFFFFFFF, // No IRQ
    }).map_err(|_| RootTaskError::BootInfoCreation)?;

    info.add_device_region(boot_info::DeviceRegion {
        paddr: memory_config::TIMER_BASE,
        size: 0x1000,
        device_type: memory_config::DEVICE_TIMER as u32,
        irq: 27, // QEMU virt timer IRQ
    }).map_err(|_| RootTaskError::BootInfoCreation)?;

    // Add untyped memory regions
    // TODO: Enumerate actual free memory regions from frame allocator
    // For now, add a large untyped region representing available RAM
    // This is a placeholder - proper implementation will query frame allocator

    // Example: 64MB of RAM starting at 64MB offset (after kernel)
    // Size bits: 26 = 64MB (2^26 = 67108864)
    info.add_untyped_region(boot_info::UntypedRegion::new(
        memory_config::RAM_BASE + 0x04000000, // 64MB offset
        26, // 64MB (2^26 bytes)
        false, // Not device memory
    )).map_err(|_| RootTaskError::BootInfoCreation)?;

    crate::kprintln!("[boot_info] Created userspace boot info:");
    crate::kprintln!("  Devices:  {} regions", info.num_device_regions);
    crate::kprintln!("  Untyped:  {} regions", info.num_untyped_regions);
    crate::kprintln!("  RAM size: {} MB", info.ram_size / (1024 * 1024));

    Ok(info)
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

    // Chapter 9: Page table switching implemented in exception handler!
    // The exception handler (handle_lower_el_aarch64_sync) now switches from user's
    // TTBR0 to kernel's TTBR1 when entering EL1 for syscall handling, then restores
    // TTBR0 before returning to EL0. This provides proper isolation and security.
    // No need to map kernel into user page table anymore!

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
    crate::memory::paging::identity_map_region(
        &mut mapper,
        memory_config::UART0_BASE as usize,
        4096,
        PageTableFlags::USER_DEVICE,
    ).map_err(|_| RootTaskError::MemoryMapping)?;

    crate::kprintln!("  Entry point:     {:#x}", entry_addr);
    crate::kprintln!("  Stack:           {:#x} - {:#x} (256 KB)", stack_bottom, stack_top);
    crate::kprintln!("  Code segment:    {} KB at virt {:#x}", code_size / 1024, virt_start);
    crate::kprintln!("  ✓ Root task ready for EL0 transition");

    // Step 2b: Create and map boot info for userspace runtime services
    crate::kprintln!("");
    crate::kprintln!("[root_task] Creating boot info for runtime services...");

    // Create boot info structure
    let userspace_boot_info = populate_boot_info()?;

    // Allocate physical frame for boot info
    let boot_info_frame = crate::memory::alloc_frame()
        .ok_or(RootTaskError::PageTableAllocation)?;
    let boot_info_phys = boot_info_frame.phys_addr();

    // Write boot info to physical frame
    let boot_info_ptr = boot_info_phys.as_usize() as *mut boot_info::BootInfo;
    core::ptr::write(boot_info_ptr, userspace_boot_info);

    // Map boot info at fixed virtual address (0x7FFF_F000 - just below IPC buffer)
    const BOOT_INFO_VADDR: usize = 0x7FFF_F000;
    mapper.map(
        VirtAddr::new(BOOT_INFO_VADDR),
        boot_info_phys,
        PageTableFlags::USER_DATA, // Read-only would be better, but use RW for now
        crate::memory::PageSize::Size4KB,
    ).map_err(|_| RootTaskError::MemoryMapping)?;

    crate::kprintln!("  Boot info phys:  {:#x}", boot_info_phys.as_usize());
    crate::kprintln!("  Boot info virt:  {:#x}", BOOT_INFO_VADDR);
    crate::kprintln!("  Boot info size:  {} bytes", boot_info::BootInfo::size());
    crate::kprintln!("  ✓ Boot info mapped for userspace");

    // Step 3: Create TCB for root task
    crate::kprintln!("  Creating root TCB...");
    let root_tcb_frame = crate::memory::alloc_frame()
        .ok_or(RootTaskError::PageTableAllocation)?;
    let root_tcb_ptr = root_tcb_frame.phys_addr().as_usize() as *mut crate::objects::TCB;
    crate::kprintln!("  Root TCB frame:  {:#x}", root_tcb_ptr as usize);

    // Create TCB with TID 1 (0 is idle thread)
    crate::kprintln!("  Initializing TCB...");
    let root_tcb = crate::objects::TCB::new(
        1,                                     // TID = 1 for root-task
        core::ptr::null_mut(),                 // No CSpace yet (will be set up later)
        user_page_table_phys.as_usize(),       // VSpace root (page table)
        VirtAddr::new(0x8000_0000),            // IPC buffer (not used yet)
        entry_addr as u64,                     // Entry point
        stack_top as u64,                      // Stack pointer
    );
    crate::kprintln!("  Writing TCB...");
    core::ptr::write(root_tcb_ptr, root_tcb);

    crate::kprintln!("  Setting state to Running...");
    // Set state to Running (root-task will start executing immediately)
    (*root_tcb_ptr).set_state(crate::objects::ThreadState::Running);

    crate::kprintln!("  Setting saved_ttbr0...");
    // Set saved_ttbr0 for context switching
    (*root_tcb_ptr).context_mut().saved_ttbr0 = user_page_table_phys.as_usize() as u64;

    crate::kprintln!("  Registering with scheduler...");
    // Register with scheduler as current thread
    crate::scheduler::test_set_current_thread(root_tcb_ptr);

    crate::kprintln!("  Root TCB:        {:#x} ✓", root_tcb_ptr as usize);

    // Step 4: Transition to EL0
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
