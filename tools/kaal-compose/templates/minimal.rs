#![no_std]
#![no_main]

use core::panic::PanicInfo;

/// Minimal KaaL root task
///
/// This is the entry point for your seL4 system.
/// Customize this to implement your system logic.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Your system initialization here

    // Example: Print to seL4 debug console
    unsafe {
        sel4_platform::adapter::seL4_DebugPutChar(b'K');
        sel4_platform::adapter::seL4_DebugPutChar(b'a');
        sel4_platform::adapter::seL4_DebugPutChar(b'a');
        sel4_platform::adapter::seL4_DebugPutChar(b'L');
        sel4_platform::adapter::seL4_DebugPutChar(b'\n');
    }

    // Main loop
    loop {
        unsafe {
            core::arch::asm!("wfi"); // Wait For Interrupt
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
