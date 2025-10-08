//! KaaL Entry Point - Rust FFI Bridge
//!
//! This module provides the C-callable entry point that receives
//! seL4 boot info and initializes the KaaL runtime.

#![no_std]

use kaal_root_task::{RootTask, RootTaskConfig};

/// C-callable entry point from wrapper
///
/// SAFETY: This function receives a pointer to seL4_BootInfo from the C wrapper.
/// The pointer must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn kaal_main(bootinfo: *const u8) -> ! {
    // Initialize KaaL with default configuration
    let config = RootTaskConfig::default();
    let mut root = match RootTask::init(config) {
        Ok(r) => r,
        Err(_) => halt("Failed to initialize RootTask"),
    };

    // Run KaaL system with component spawning
    root.run_with(|broker| {
        // TODO: Spawn your components here
        // See examples/my-kaal-system/src/main.rs for patterns

        // For now, just entering idle loop
        let _ = broker;
    });
}

/// Halt system on critical error
fn halt(msg: &str) -> ! {
    // In a real system, you'd log this error
    let _ = msg;
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
