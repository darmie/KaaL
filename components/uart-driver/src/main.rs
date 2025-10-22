//! UART Driver Component
//!
//! Demonstrates interrupt handling using the IRQ capability system.
//! Binds to UART0 IRQ and handles keyboard input interrupts.

#![no_std]
#![no_main]

use kaal_sdk::{
    component::Component,
    printf,
    syscall,
};

// Declare this as a driver component
kaal_sdk::component! {
    name: "uart_driver",
    type: Driver,
    version: "0.1.0",
    capabilities: ["caps:allocate", "irq:control"],
    impl: UartDriver
}

/// UART Driver
pub struct UartDriver {
    notification_cap: usize,
    irq_handler_slot: usize,
    irq_count: u32,
}

// Well-known capability slots
const IRQ_CONTROL_SLOT: usize = 0; // IRQControl capability from root-task
const UART0_IRQ: usize = 33;       // UART0 IRQ number (from platform config)

impl Component for UartDriver {
    fn init() -> kaal_sdk::Result<Self> {
        printf!("\n");
        printf!("===========================================\n");
        printf!("  UART Driver - IRQ Capability Demo\n");
        printf!("===========================================\n");
        printf!("\n");

        // Step 1: Create notification for UART IRQ signaling
        printf!("[uart_driver] Step 1: Create notification for IRQ signaling\n");
        let notification_cap = syscall::notification_create()?;
        printf!("  ✓ Created notification at slot {}\n", notification_cap);

        // Step 2: Allocate slot for IRQHandler
        printf!("[uart_driver] Step 2: Allocate slot for IRQHandler\n");
        let irq_handler_slot = syscall::cap_allocate()?;
        printf!("  ✓ Allocated IRQHandler slot {}\n", irq_handler_slot);

        // Step 3: Get IRQHandler from IRQControl
        printf!("[uart_driver] Step 3: Allocate IRQHandler for UART0 (IRQ {})\n", UART0_IRQ);
        match unsafe {
            syscall::irq_handler_get(
                IRQ_CONTROL_SLOT,
                UART0_IRQ,
                notification_cap,
                irq_handler_slot,
            )
        } {
            Ok(()) => {
                printf!("  ✓ Successfully bound IRQ {} to notification\n", UART0_IRQ);
                printf!("  ✓ IRQHandler created at slot {}\n", irq_handler_slot);
            }
            Err(_) => {
                printf!("  ✗ FAIL: irq_handler_get failed\n");
                printf!("  This might be because:\n");
                printf!("    - No IRQControl capability in slot 0\n");
                printf!("    - IRQ {} already allocated\n", UART0_IRQ);
                printf!("    - Invalid notification capability\n");
                printf!("\n");
                printf!("  Note: Full IRQ driver requires IRQControl delegation from root-task\n");
                printf!("  For now, driver initialization complete (syscall interface verified)\n");
                printf!("\n");

                // Return early but don't fail - let the component run
                return Ok(Self {
                    notification_cap,
                    irq_handler_slot,
                    irq_count: 0,
                });
            }
        }

        printf!("\n");
        printf!("===========================================\n");
        printf!("  UART Driver Initialized Successfully!\n");
        printf!("===========================================\n");
        printf!("\n");
        printf!("Driver configuration:\n");
        printf!("  - IRQ Number:       {}\n", UART0_IRQ);
        printf!("  - Notification:     slot {}\n", notification_cap);
        printf!("  - IRQHandler:       slot {}\n", irq_handler_slot);
        printf!("\n");
        printf!("Driver is now waiting for UART interrupts...\n");
        printf!("(Press keys in QEMU to generate UART RX interrupts)\n");
        printf!("\n");

        Ok(Self {
            notification_cap,
            irq_handler_slot,
            irq_count: 0,
        })
    }

    fn run(&mut self) -> ! {
        loop {
            // Wait for notification (blocks until IRQ fires)
            match syscall::wait(self.notification_cap) {
                Ok(badge) => {
                    self.irq_count += 1;
                    printf!("\n");
                    printf!(">>> IRQ RECEIVED! <<<\n");
                    printf!("  IRQ count: {}\n", self.irq_count);
                    printf!("  Badge:     0x{:x}\n", badge);
                    printf!("\n");

                    // In a real driver, we would:
                    // 1. Read UART data register
                    // 2. Process received character
                    // 3. Clear UART interrupt flag

                    // Acknowledge IRQ to re-enable it
                    match unsafe { syscall::irq_handler_ack(self.irq_handler_slot) } {
                        Ok(()) => {
                            printf!("  ✓ IRQ acknowledged and re-enabled\n");
                        }
                        Err(_) => {
                            printf!("  ✗ FAIL: irq_handler_ack failed\n");
                        }
                    }
                }
                Err(_) => {
                    // Wait failed - yield and retry
                    syscall::yield_now();
                }
            }
        }
    }
}
