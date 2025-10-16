//! System Init Component
//!
//! The first component spawned by the root-task. Responsible for:
//! - Initializing core system services
//! - Spawning other components based on priority
//! - Managing system-wide initialization

#![no_std]
#![no_main]

use kaal_sdk::{
    component::{Component, ServiceBase},
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
            syscall::print("[system_init] Starting main loop\n");
            syscall::print("[system_init] TODO: Spawn other components\n");
            syscall::print("[system_init] TODO: Initialize system services\n");
            syscall::print("\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("  System Init: Ready ✓\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("\n");
        }

        // Main loop - yield to allow other tasks to run
        loop {
            unsafe {
                syscall::yield_now();
            }
        }
    }
}
