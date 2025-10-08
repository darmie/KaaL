//! Minimal bootable root task for testing Rust elfloader
//!
//! This is a truly minimal example that doesn't depend on complex KaaL infrastructure,
//! making it suitable for testing the bootloader in isolation.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Print banner using seL4_DebugPutChar syscall
    debug_print("\n=================================\n");
    debug_print("  KaaL Minimal Root Task v0.1.0\n");
    debug_print("  Booted with Rust Elfloader!\n");
    debug_print("=================================\n\n");

    debug_print("Root task started successfully!\n");
    debug_print("seL4 microkernel is running.\n");
    debug_print("Rust elfloader handoff complete.\n\n");

    debug_print("Entering idle loop...\n");

    // Idle loop
    loop {
        unsafe {
            core::arch::asm!("wfe");
        }
    }
}

fn debug_print(s: &str) {
    for byte in s.bytes() {
        unsafe {
            core::arch::asm!(
                "mov x0, {ch}",
                "mov x7, #1",  // seL4_DebugPutChar
                "svc #0",
                ch = in(reg) byte as u64,
                out("x0") _,
                out("x7") _,
            );
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    debug_print("\n[PANIC] ");
    if let Some(location) = info.location() {
        debug_print("at ");
        debug_print(location.file());
        debug_print("\n");
    }

    loop {
        unsafe {
            core::arch::asm!("wfe");
        }
    }
}
