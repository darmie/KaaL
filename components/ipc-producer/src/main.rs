//! IPC Producer Component
//!
//! Demonstrates inter-component message passing using the semantic Channel API.
//! Sends u32 messages to the consumer component.
//!
//! # Arguments (Phase 5 - to be implemented)
//! - shared_mem_virt: Virtual address of shared memory for channel
//! - receiver_notify_cap: Notification capability for receiver
//! - sender_notify_cap: Notification capability for sender (this component)

#![no_std]
#![no_main]

use kaal_sdk::{
    component::{Component, ServiceBase},
    syscall,
    message::{Channel, ChannelConfig},
};

// Declare this as a service component
kaal_sdk::component! {
    name: "ipc_producer",
    type: Service,
    version: "0.1.0",
    capabilities: ["memory:map", "notification:signal", "notification:wait"],
    impl: IpcProducer
}

/// IPC Producer Service
pub struct IpcProducer;

impl Component for IpcProducer {
    fn init() -> kaal_sdk::Result<Self> {
        unsafe {
            syscall::print("\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("  IPC Producer v0.1.0\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("\n");
            syscall::print("[producer] Initializing...\n");
            syscall::print("[producer] Using semantic message-passing API\n");
        }

        Ok(IpcProducer)
    }

    fn run(&mut self) -> ! {
        unsafe {
            syscall::print("[producer] Starting message production\n");

            // Create our own notification for signaling
            syscall::print("[producer] Creating notification capability...\n");
            let producer_notify = match syscall::notification_create() {
                Ok(slot) => {
                    syscall::print("  ✓ Created notification at slot: ");
                    // Can't print number easily, just indicate success
                    syscall::print("X\n");
                    slot
                }
                Err(_) => {
                    syscall::print("  ✗ Failed to create notification\n");
                    loop { syscall::yield_now(); }
                }
            };

            // For demo: we'll use hardcoded shared memory location
            // In real implementation, this would be discovered or passed
            let shared_mem_virt = 0x80000000u64; // Hardcoded for demo

            syscall::print("[producer] Configuration:\n");
            syscall::print("  - Shared memory at: 0x80000000 (hardcoded)\n");
            syscall::print("  - Producer notification created\n");
            syscall::print("\n");

            // Initialize the shared ring buffer
            syscall::print("[producer] Initializing SharedRing buffer...\n");

            // Write a magic value to shared memory to test it's working
            let shared_ptr = shared_mem_virt as *mut u32;
            *shared_ptr = 0xDEADBEEF;

            syscall::print("[producer] Wrote magic value 0xDEADBEEF to shared memory\n");

            // For this demo, we'll just write test data to shared memory
            // In real implementation, we'd exchange capabilities with consumer first
            syscall::print("[producer] Writing test data to shared memory...\n");
            for i in 0..5 {
                // Write message to shared memory (simplified - no ring buffer yet)
                let msg_ptr = (shared_mem_virt + 4 + (i * 4)) as *mut u32;
                *msg_ptr = 0x1000 + i as u32;

                syscall::print("  → Wrote test message ");
                syscall::print("X\n");

                // Yield to let consumer see the data
                for _ in 0..5 {
                    syscall::yield_now();
                }
            }

            syscall::print("[producer] All test data written!\n");
            syscall::print("\n");
        }

        // Continue yielding
        loop {
            unsafe {
                syscall::yield_now();
            }
        }
    }
}
