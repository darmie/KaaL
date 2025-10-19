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
            // Hybrid approach architecture (capability granting TODO):
            // 1. root-task spawns components with autostart=true (works now)
            // 2. system_init WILL spawn spawned_by="system_init" components (future)
            // 3. Runtime on-demand spawning via SDK helper (needs capability grants)
            //
            // NOTE: spawned_by delegation requires capability granting to be implemented
            // Currently, system_init can't spawn because declared capabilities aren't
            // actually granted by the kernel yet. This will be implemented in a future
            // chapter when we add the capability distribution system.
            //
            syscall::print("\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("  System Init: Ready (Developer Playground)\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("\n");
            syscall::print("[system_init] Component spawning delegated to root-task\n");
            syscall::print("[system_init] TODO: Implement capability granting for spawned_by\n");
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
