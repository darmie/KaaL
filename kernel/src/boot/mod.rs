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
    // Get boot parameters from registers (x19-x23)
    let params = unsafe { get_boot_params() };

    // Initialize UART first so we can print
    crate::arch::aarch64::uart::init();

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
    match dtb::parse(params.dtb_addr) {
        Ok(info) => {
            crate::kprintln!("Device tree parsed successfully:");
            crate::kprintln!("  Model:       {}", info.model);
            crate::kprintln!("  Memory:      {:#x} - {:#x} ({} MB)",
                           info.memory_start,
                           info.memory_end,
                           (info.memory_end - info.memory_start) / (1024 * 1024));
            crate::kprintln!("");
        }
        Err(e) => {
            crate::kprintln!("ERROR: Failed to parse DTB: {:?}", e);
        }
    }

    crate::kprintln!("═══════════════════════════════════════════════════════════");
    crate::kprintln!("  Chapter 1: COMPLETE ✓");
    crate::kprintln!("═══════════════════════════════════════════════════════════");
    crate::kprintln!("");
    crate::kprintln!("Kernel initialization complete!");
    crate::kprintln!("(Halting - more features coming in Chapter 2)");
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

    asm!(
        "mov {0}, x19",
        "mov {1}, x20",
        "mov {2}, x21",
        "mov {3}, x22",
        "mov {4}, x23",
        out(reg) dtb_addr,
        out(reg) root_p_start,
        out(reg) root_p_end,
        out(reg) root_v_entry,
        out(reg) pv_offset,
    );

    BootParams {
        dtb_addr,
        root_p_start,
        root_p_end,
        root_v_entry,
        pv_offset,
    }
}
