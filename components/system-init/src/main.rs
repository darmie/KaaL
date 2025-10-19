//! System Init Component
//!
//! The first component spawned by the root-task. Responsible for:
//! - Initializing core system services
//! - Spawning other components based on priority
//! - Managing system-wide initialization

#![no_std]
#![no_main]

use kaal_sdk::{
    component::Component,
    syscall,
};

// Declare this as a system service - generates metadata, _start, and panic handler
kaal_sdk::component! {
    name: "system_init",
    type: Service,
    version: "0.1.0",
    capabilities: ["process:create", "process:destroy", "memory:allocate", "ipc:*"],
    impl: SystemInit
}

/// System initialization service
pub struct SystemInit;

impl Component for SystemInit {
    fn init() -> kaal_sdk::Result<Self> {
        unsafe {
            syscall::print("\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("  System Init Component v0.1.0\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("\n");
            syscall::print("[system_init] Initializing...\n");
            syscall::print("[system_init] Component spawned successfully!\n");
            syscall::print("[system_init] Running in userspace (EL0)\n");
            syscall::print("\n");
        }
        Ok(SystemInit)
    }

    fn run(&mut self) -> ! {
        unsafe {
            syscall::print("[system_init] Starting initialization\n");

            // Create a notification object for event-driven operation
            syscall::print("[system_init] Creating notification for event handling...\n");
            let notification_cap = match syscall::notification_create() {
                Ok(cap) => {
                    syscall::print("[system_init] Notification created successfully\n");
                    cap
                }
                Err(_) => {
                    syscall::print("[system_init] ERROR: Failed to create notification!\n");
                    // Can't proceed without notification, just yield forever
                    loop {
                        syscall::yield_now();
                    }
                }
            };

            // ═══════════════════════════════════════════════════════════
            // Developer Playground - Hybrid Component Spawning
            // ═══════════════════════════════════════════════════════════
            //
            // This demonstrates the hybrid approach:
            // 1. Registry-based spawning for autostart components (from generated registry)
            // 2. SDK helper available for runtime spawning (spawn_from_elf still works!)
            //
            syscall::print("\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("  System Init: Developer Playground\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("\n");

            // Import generated registry
            mod generated {
                include!("generated/registry.rs");
            }

            // Spawn autostart components from registry
            syscall::print("[system_init] Spawning autostart components from registry...\n");
            for component in generated::COMPONENT_REGISTRY {
                if component.autostart {
                    syscall::print("[system_init]   → Spawning ");
                    syscall::print(component.name);
                    syscall::print("...\n");

                    match kaal_sdk::component::spawn_from_elf(component.binary_data, component.priority) {
                        Ok(_result) => {
                            syscall::print("[system_init]   ✓ ");
                            syscall::print(component.name);
                            syscall::print(" spawned successfully!\n");
                        }
                        Err(_) => {
                            syscall::print("[system_init]   ERROR: Failed to spawn ");
                            syscall::print(component.name);
                            syscall::print("\n");
                        }
                    }
                }
            }

            syscall::print("\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("  System Init: Component Spawning Demo Complete!\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("\n");

            // Event loop - block waiting for notifications instead of busy-yielding
            syscall::print("[system_init] Entering event loop (waiting for signals)\n");
            loop {
                // Block waiting for notification events
                // This removes us from the scheduler's ready queue
                match syscall::wait(notification_cap) {
                    Ok(signals) => {
                        if signals != 0 {
                            syscall::print("[system_init] Received notification signal\n");
                            // Handle events here
                        }
                    }
                    Err(_) => {
                        syscall::print("[system_init] Wait error\n");
                    }
                }
            }
        }
    }
}
