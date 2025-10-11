//! KaaL Rust Microkernel
//!
//! A pure-Rust seL4-compatible microkernel for ARM64.
//!
//! # Architecture
//!
//! The kernel is organized into the following modules:
//! - `boot`: Boot sequence and initialization
//! - `arch`: Architecture-specific code (ARM64)
//! - `debug`: Debug output and logging
//!
//! # Chapter 1: Bare Metal Boot & Early Init
//!
//! This is the initial implementation focusing on:
//! - Booting on QEMU ARM64 virt platform
//! - Initializing serial UART output
//! - Parsing device tree (DTB)
//! - Printing "Hello from KaaL Kernel!"

#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(asm_const)]

use core::panic::PanicInfo;

// Module declarations
pub mod arch;
pub mod boot;
pub mod debug;

/// Kernel entry point - called by elfloader
///
/// This is the first Rust code that executes when the kernel starts.
/// The elfloader has already:
/// - Set up a basic stack
/// - Passed boot parameters in registers
/// - Loaded the kernel into memory
///
/// Boot parameters (ARM64 convention):
/// - x0: DTB physical address
/// - x1: Root task physical region start
/// - x2: Root task physical region end
/// - x3: Root task virtual entry point
/// - x4: Physical-virtual offset
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // This will be implemented in boot/mod.rs
    boot::kernel_entry()
}

/// Panic handler - called when the kernel panics
///
/// For now, this just loops forever. In later chapters, we'll:
/// - Print panic information to UART
/// - Dump register state
/// - Halt the system gracefully
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
