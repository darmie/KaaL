//! Serial Driver Component - Example using KaaL SDK
//!
//! Demonstrates the device driver pattern from SYSTEM_COMPOSITION.md:
//! - Minimal driver structure
//! - Hardware access (MMIO)
//! - Event loop (IPC + IRQ)
//! - Component lifecycle

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use kaal_sdk::{
    component::{Component, DriverBase, ComponentType},
    syscall, memory,
};

// Define component metadata (for system composition)
kaal_sdk::component_metadata! {
    name: "serial_driver",
    type: Driver,
    version: "0.1.0",
    capabilities: ["memory_map:0x09000000", "interrupt:33"],
}

/// Serial driver state
struct SerialDriver {
    base: DriverBase,
    uart_base: Option<usize>,
}

impl Component for SerialDriver {
    fn init() -> kaal_sdk::Result<Self> {
        syscall::print("[serial_driver] Initializing...\n");

        // Create driver base
        let mut base = DriverBase::new("serial_driver")?;

        syscall::print("[serial_driver] ✓ Driver base created\n");

        // In a real implementation, we would:
        // 1. Map UART MMIO region
        // 2. Register IRQ handler
        // 3. Initialize hardware

        // For now, just demonstrate the pattern
        let driver = Self {
            base,
            uart_base: None,
        };

        syscall::print("[serial_driver] ✓ Initialization complete\n");
        Ok(driver)
    }

    fn run(&mut self) -> ! {
        syscall::print("[serial_driver] Entering event loop...\n");
        syscall::print("[serial_driver] Ready to handle:\n");
        syscall::print("  • IPC requests (Write, Read)\n");
        syscall::print("  • Hardware interrupts\n");
        syscall::print("\n");

        syscall::print("═══════════════════════════════════════════════════════════\n");
        syscall::print("  Serial Driver Component: RUNNING ✓\n");
        syscall::print("═══════════════════════════════════════════════════════════\n");
        syscall::print("\n");

        syscall::print("[serial_driver] Component Pattern Demonstrated:\n");
        syscall::print("  ✓ Component trait impl (init + run)\n");
        syscall::print("  ✓ Driver base (common functionality)\n");
        syscall::print("  ✓ Metadata annotation\n");
        syscall::print("  ✓ Clean SDK API (no raw syscalls)\n");
        syscall::print("  ✓ Event loop structure\n");
        syscall::print("\n");

        syscall::print("[serial_driver] Architecture Benefits:\n");
        syscall::print("  • Isolated address space\n");
        syscall::print("  • Minimal capabilities\n");
        syscall::print("  • Fault isolation\n");
        syscall::print("  • IPC-based communication\n");
        syscall::print("\n");

        // Event loop (simplified)
        loop {
            // In a real driver, this would:
            // 1. Wait for IPC message or IRQ
            // 2. Handle request/interrupt
            // 3. Send IPC reply if needed

            syscall::yield_now();

            // Simulate event handling
            unsafe {
                core::arch::asm!("wfi");
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    syscall::print("\n");
    syscall::print("═══════════════════════════════════════════════════════════\n");
    syscall::print("  Serial Driver Component - SDK Example\n");
    syscall::print("═══════════════════════════════════════════════════════════\n");
    syscall::print("\n");

    // Start component using SDK pattern
    SerialDriver::start()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    syscall::print("[serial_driver] PANIC!\n");
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
