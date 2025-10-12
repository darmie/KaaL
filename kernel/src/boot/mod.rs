//! Boot sequence and early initialization
//!
//! This module handles the kernel boot process:
//! 1. Receiving boot parameters from elfloader
//! 2. Initializing early debug output (UART)
//! 3. Parsing device tree
//! 4. Setting up memory regions

use core::arch::asm;

pub mod dtb;

/// Boot parameters passed from elfloader
#[repr(C)]
pub struct BootParams {
    pub dtb_addr: usize,
    pub root_p_start: usize,
    pub root_p_end: usize,
    pub root_v_entry: usize,
    pub pv_offset: usize,
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
    crate::kprintln!("═══════════════════════════════════════════════════════════");
    crate::kprintln!("  KaaL Rust Microkernel v0.1.0");
    crate::kprintln!("  Chapter 1: Bare Metal Boot & Early Init");
    crate::kprintln!("═══════════════════════════════════════════════════════════");
    crate::kprintln!("");
    crate::kprintln!("Boot parameters:");
    crate::kprintln!("  DTB:         {:#x}", params.dtb_addr);
    crate::kprintln!("  Root task:   {:#x} - {:#x}", params.root_p_start, params.root_p_end);
    crate::kprintln!("  Entry:       {:#x}", params.root_v_entry);
    crate::kprintln!("  PV offset:   {:#x}", params.pv_offset);
    crate::kprintln!("");

    // Parse device tree
    crate::kprintln!("Parsing device tree...");
    let dtb_info = match dtb::parse(params.dtb_addr) {
        Ok(info) => {
            crate::kprintln!("Device tree parsed successfully:");
            crate::kprintln!("  Model:       {}", info.model);
            crate::kprintln!("  Memory:      {:#x} - {:#x} ({} MB)",
                           info.memory_start,
                           info.memory_end,
                           (info.memory_end - info.memory_start) / (1024 * 1024));
            crate::kprintln!("");
            Some(info)
        }
        Err(e) => {
            crate::kprintln!("ERROR: Failed to parse DTB: {:?}", e);
            None
        }
    };

    crate::kprintln!("═══════════════════════════════════════════════════════════");
    crate::kprintln!("  Chapter 1: COMPLETE ✓");
    crate::kprintln!("═══════════════════════════════════════════════════════════");
    crate::kprintln!("");

    // Chapter 2: Memory Management
    if let Some(info) = dtb_info {
        crate::kprintln!("═══════════════════════════════════════════════════════════");
        crate::kprintln!("  Chapter 2: Memory Management");
        crate::kprintln!("═══════════════════════════════════════════════════════════");
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

        // Test frame allocation
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

        crate::kprintln!("");
        crate::kprintln!("═══════════════════════════════════════════════════════════");
        crate::kprintln!("  Chapter 2: Phase 1 & 2 COMPLETE ✓");
        crate::kprintln!("═══════════════════════════════════════════════════════════");
        crate::kprintln!("");
    }

    crate::kprintln!("Kernel initialization complete!");
    crate::kprintln!("(Halting - MMU and page tables coming in Phase 3)");
    crate::kprintln!("");

    // Halt (later chapters will jump to scheduler)
    loop {
        unsafe {
            asm!("wfi"); // Wait for interrupt
        }
    }
}

/// Get boot parameters from saved registers
///
/// The _start function saves x0-x4 into x19-x23
/// We retrieve them here
#[inline(always)]
unsafe fn get_boot_params() -> BootParams {
    let dtb_addr: usize;
    let root_p_start: usize;
    let root_p_end: usize;
    let root_v_entry: usize;
    let pv_offset: usize;

    // Use specific registers to avoid clobbering x19-x23
    asm!(
        "mov {dtb}, x19",
        "mov {root_start}, x20",
        "mov {root_end}, x21",
        "mov {entry}, x22",
        "mov {offset}, x23",
        dtb = out(reg) dtb_addr,
        root_start = out(reg) root_p_start,
        root_end = out(reg) root_p_end,
        entry = out(reg) root_v_entry,
        offset = out(reg) pv_offset,
        options(nomem, nostack),
    );

    BootParams {
        dtb_addr,
        root_p_start,
        root_p_end,
        root_v_entry,
        pv_offset,
    }
}
