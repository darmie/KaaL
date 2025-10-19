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
            // Developer Playground - Add your experiments here!
            // ═══════════════════════════════════════════════════════════
            //
            // This is the entry point for application-level development.
            // The kernel and root-task handle low-level initialization.
            // Use this component to:
            //
            // - Test new IPC patterns
            // - Spawn additional components
            // - Experiment with syscalls
            // - Build application logic
            // - Demo features
            //
            // Example: IPC Demo (currently runs from root-task)
            //
            // TODO: Move component spawning to SDK (see docs/COMPONENT_SPAWN_DESIGN.md)
            //
            // Planned approach (Option B - SDK helper):
            //   let registry = generated::component_registry::get_registry();
            //   let producer = kaal_sdk::component::spawn(registry, "ipc_producer")?;
            //   let consumer = kaal_sdk::component::spawn(registry, "ipc_consumer")?;
            //
            // This reuses existing syscalls (no kernel changes needed!):
            //   - SYS_MEMORY_ALLOCATE (for process memory, stack, page table)
            //   - SYS_MEMORY_MAP/UNMAP (to copy ELF segments)
            //   - SYS_PROCESS_CREATE (to create TCB)
            //   - SYS_CAP_INSERT_SELF (to get TCB capability)
            //
            // See docs/COMPONENT_SPAWN_DESIGN.md for full design.
            //
            syscall::print("\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("  System Init: Developer Playground\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("\n");
            syscall::print("[system_init] This is your development entry point\n");
            syscall::print("[system_init] Add experiments, tests, and demos here\n");
            syscall::print("[system_init] Keep kernel and root-task minimal\n");
            syscall::print("\n");
            syscall::print("Available infrastructure:\n");
            syscall::print("  ✓ IPC: Shared memory + notifications\n");
            syscall::print("  ✓ Memory: Allocate, map, unmap\n");
            syscall::print("  ✓ Capabilities: Create, transfer, manage\n");
            syscall::print("  ✓ Components: Producer/consumer patterns\n");
            syscall::print("\n");
            syscall::print("Next steps:\n");
            syscall::print("  → Implement SYS_COMPONENT_SPAWN for userspace\n");
            syscall::print("  → Move IPC demo from root-task to here\n");
            syscall::print("  → Add your own experiments!\n");
            syscall::print("\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("  System Init: Ready ✓\n");
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
