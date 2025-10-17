//! Minimal test component to debug entry point issues
//!
//! This component does absolutely nothing except loop with wfi.
//! No SDK, no syscalls, just the bare minimum.

#![no_std]
#![no_main]

// Entry point - the simplest possible with yield
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Print a simple message first to verify we're running
    const MSG: &[u8] = b"[test-minimal] ALIVE!\n";

    unsafe {
        // Call debug print syscall with the message
        core::arch::asm!(
            "mov x8, #0x1001",      // SYS_DEBUG_PRINT
            "svc #0",               // Make syscall
            in("x0") MSG.as_ptr(),
            in("x1") MSG.len(),
            out("x8") _,
        );
    }

    // Just yield forever to test basic context switching
    loop {
        unsafe {
            // SYS_YIELD = 0x01
            // Call yield syscall directly without SDK
            core::arch::asm!(
                "mov x8, #0x01",  // SYS_YIELD
                "svc #0",
                out("x8") _,
                out("x0") _,
            );
        }
    }
}

// Panic handler - required for no_std
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}