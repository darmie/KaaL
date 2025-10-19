//! Boot sequence and early initialization
//!
//! This module handles the kernel boot process:
//! 1. Receiving boot parameters from elfloader
//! 2. Initializing early debug output (UART)
//! 3. Parsing device tree
//! 4. Setting up memory regions

use core::arch::asm;
use crate::memory::VirtAddr;

pub mod dtb;
pub mod bootinfo;
pub mod root_task;
pub mod boot_info;     // Userspace boot info (for runtime services)

/// Boot parameters passed from elfloader
#[repr(C)]
pub struct BootParams {
    pub dtb_addr: usize,
    pub root_p_start: usize,
    pub root_p_end: usize,
    pub root_v_entry: usize,
    pub pv_offset: usize,
    pub dtb_size: usize,
}

/// Kernel entry point (called from _start)
///
/// This is the first Rust function that executes.
/// Boot parameters are in callee-saved registers x19-x23 (set by _start).
pub fn kernel_entry() -> ! {
    // CRITICAL: Get boot parameters from registers FIRST before any function calls
    // The registers x19-x23 are callee-saved, but we must read them before
    // any function calls that might use them.
    let params = unsafe { get_boot_params() };

    // Initialize console component (safe to do after reading params)
    crate::config::init_console();

    // Print banner
    crate::kprintln!("KaaL Rust Microkernel v0.1.0");
    crate::kprintln!("");
    crate::kprintln!("[boot] DTB: {:#x} (size: {} bytes)", params.dtb_addr, params.dtb_size);
    crate::kprintln!("[boot] Root task: {:#x} - {:#x}", params.root_p_start, params.root_p_end);
    crate::kprintln!("[boot] Entry: {:#x}", params.root_v_entry);
    crate::kprintln!("[boot] PV offset: {:#x}", params.pv_offset);
    crate::kprintln!("");

    // Initialize global boot info
    let boot_info = bootinfo::BootInfo::new(
        crate::memory::PhysAddr::new(params.root_p_start),
        crate::memory::PhysAddr::new(params.root_p_end),
        params.pv_offset,
        params.root_v_entry,
        crate::memory::PhysAddr::new(params.dtb_addr),
        params.dtb_size,
    );
    unsafe {
        bootinfo::init_boot_info(boot_info);
    }
    crate::kprintln!("[boot] Boot info initialized and stored globally");

    // Parse device tree
    crate::kprintln!("[boot] Parsing device tree...");
    let dtb_info = match dtb::parse(params.dtb_addr) {
        Ok(info) => {
            crate::kprintln!("[boot] Model: {}", info.model);
            crate::kprintln!("[boot] Memory: {:#x} - {:#x} ({} MB)",
                           info.memory_start,
                           info.memory_end,
                           (info.memory_end - info.memory_start) / (1024 * 1024));
            crate::kprintln!("");
            Some(info)
        }
        Err(e) => {
            crate::kprintln!("[boot] ERROR: Failed to parse DTB: {:?}", e);
            None
        }
    };

    crate::kprintln!("");

    // Memory Management - See docs/chapters/CHAPTER_02_STATUS.md
    if let Some(info) = dtb_info {
        crate::kprintln!("[boot] Initializing memory subsystem");
        crate::kprintln!("");

        // Get kernel boundaries (symbols from linker script)
        extern "C" {
            static _kernel_start: u8;
            static _kernel_end: u8;
        }

        let kernel_start = unsafe { &_kernel_start as *const u8 as usize };
        let kernel_end = unsafe { &_kernel_end as *const u8 as usize };

        // Initialize memory subsystem
        unsafe {
            crate::memory::init(
                crate::memory::PhysAddr::new(kernel_start),
                crate::memory::PhysAddr::new(kernel_end),
                crate::memory::PhysAddr::new(info.memory_start),
                info.memory_end - info.memory_start,
            );
        }

        // Frame allocator test - See docs/chapters/CHAPTER_02_STATUS.md
        // Commented out to reduce boot verbosity. Test verified functional.
        /*
        crate::kprintln!("");
        crate::kprintln!("[test] Testing frame allocator...");
        if let Some(frame1) = crate::memory::alloc_frame() {
            crate::kprintln!("  Allocated frame: {:?}", frame1);
            if let Some(frame2) = crate::memory::alloc_frame() {
                crate::kprintln!("  Allocated frame: {:?}", frame2);

                // Deallocate
                unsafe {
                    crate::memory::dealloc_frame(frame1);
                    crate::memory::dealloc_frame(frame2);
                }
                crate::kprintln!("  Deallocated both frames");
            }
        }

        if let Some((free, total)) = crate::memory::memory_stats() {
            crate::kprintln!("  Final stats: {}/{} frames free", free, total);
        }
        */

        // Page tables and MMU setup - See docs/chapters/CHAPTER_02_STATUS.md
        crate::kprintln!("[boot] Setting up page tables and MMU...");

        // Allocate root page table for kernel space (TTBR1)
        let root_frame = crate::memory::alloc_frame().expect("Failed to allocate root page table");
        let root_phys = root_frame.phys_addr();
        let root_table = unsafe { &mut *(root_phys.as_usize() as *mut crate::arch::aarch64::page_table::PageTable) };
        root_table.zero();

        // Create page mapper
        let mut mapper = unsafe { crate::memory::PageMapper::new(root_table) };

        // Map necessary memory regions for kernel operation
        use crate::arch::aarch64::page_table::PageTableFlags;

        // 1. Map DTB + elfloader + root task region (everything before kernel)
        crate::memory::paging::identity_map_region(
            &mut mapper,
            info.memory_start,
            kernel_start - info.memory_start,
            PageTableFlags::KERNEL_DATA,
        ).expect("Failed to map DTB region");

        // 2. Map kernel code and data (use KERNEL_RWX for bootstrapping)
        let kernel_size = kernel_end - kernel_start;
        crate::memory::paging::identity_map_region(
            &mut mapper,
            kernel_start,
            kernel_size,
            PageTableFlags::KERNEL_RWX,  // RWX for bootstrapping (TODO: split code/data)
        ).expect("Failed to map kernel");

        // 3. Map stack region (grows down from top of RAM)
        let stack_region_start = kernel_end;
        let stack_region_size = info.memory_end - kernel_end;
        crate::memory::paging::identity_map_region(
            &mut mapper,
            stack_region_start,
            stack_region_size,
            PageTableFlags::KERNEL_DATA,
        ).expect("Failed to map stack region");

        // 4. Map UART for console output
        crate::memory::paging::identity_map_region(
            &mut mapper,
            crate::generated::memory_config::UART0_BASE as usize,
            4096,
            PageTableFlags::KERNEL_DEVICE,
        ).expect("Failed to map UART");

        // CRITICAL: Install exception handlers BEFORE MMU enable!
        // MMU enable might trigger exceptions, so handlers must be ready
        crate::arch::aarch64::exception::init();

        crate::kprintln!("[boot] Enabling MMU...");

        let mmu_config = crate::arch::aarch64::mmu::MmuConfig {
            ttbr1: root_phys,
            ttbr0: Some(root_phys), // Use same table for identity mapping
        };

        unsafe {
            crate::arch::aarch64::mmu::init_mmu(mmu_config);
        }

        let mmu_enabled = crate::arch::aarch64::mmu::is_mmu_enabled();
        if !mmu_enabled {
            panic!("MMU failed to enable!");
        }

        crate::kprintln!("[boot] MMU enabled successfully");
        crate::kprintln!("");
    }

    // Exception Handling & Syscalls - See docs/chapters/CHAPTER_03_STATUS.md
    // Exception vector table already installed before MMU enable
    // Exception handling verified (trap frame, ESR/FAR decoding, syscalls)

    crate::kprintln!("[boot] Kernel initialization complete");
    crate::kprintln!("");

    // Initialize scheduler before creating root task - See docs/chapters/CHAPTER_08_STATUS.md
    crate::kprintln!("[boot] Initializing scheduler");

    // Create idle thread TCB
    let idle_tcb_frame = crate::memory::alloc_frame()
        .expect("Failed to allocate idle thread TCB");
    let idle_tcb_ptr = idle_tcb_frame.phys_addr().as_usize() as *mut crate::objects::TCB;
    unsafe {
        use crate::memory::VirtAddr;
        // Create idle thread (just spins in WFI loop)
        let idle_tcb = crate::objects::TCB::new(
            0, // Idle thread ID
            core::ptr::null_mut(), // No CSpace
            0, // No page table (uses kernel's)
            VirtAddr::new(0), // No IPC buffer
            0, // No entry point (will never execute user code)
            0, // No stack
            0, // No capabilities (idle thread can't call syscalls)
        );
        core::ptr::write(idle_tcb_ptr, idle_tcb);

        // Initialize scheduler with idle thread
        crate::scheduler::init(idle_tcb_ptr);

        // Initialize timer for preemption
        crate::scheduler::timer::init();

        // Enable IRQs for timer interrupts
        core::arch::asm!("msr daifclr, #2"); // Clear IRQ mask (bit 1)
    }
    crate::kprintln!("");

    // Create and start root task - See docs/chapters/CHAPTER_07_STATUS.md
    crate::kprintln!("[boot] Starting root task");
    crate::kprintln!("");

    unsafe {
        // Create and start root task in EL0
        // This function never returns - it transitions to EL0
        root_task::create_and_start_root_task();
    }

    // Idle loop
    loop {
        unsafe {
            asm!("wfi"); // Wait for interrupt
        }
    }
}

/// Get boot parameters from saved registers
///
/// The _start function saves x0-x5 into x19-x24
/// We retrieve them here
#[inline(always)]
unsafe fn get_boot_params() -> BootParams {
    let dtb_addr: usize;
    let root_p_start: usize;
    let root_p_end: usize;
    let root_v_entry: usize;
    let pv_offset: usize;
    let dtb_size: usize;

    // Use specific registers to avoid clobbering x19-x24
    asm!(
        "mov {dtb}, x19",
        "mov {root_start}, x20",
        "mov {root_end}, x21",
        "mov {entry}, x22",
        "mov {offset}, x23",
        "mov {dtb_size}, x24",
        dtb = out(reg) dtb_addr,
        root_start = out(reg) root_p_start,
        root_end = out(reg) root_p_end,
        entry = out(reg) root_v_entry,
        offset = out(reg) pv_offset,
        dtb_size = out(reg) dtb_size,
        options(nomem, nostack),
    );

    BootParams {
        dtb_addr,
        root_p_start,
        root_p_end,
        root_v_entry,
        pv_offset,
        dtb_size,
    }
}
