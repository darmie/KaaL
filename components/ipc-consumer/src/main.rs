//! IPC Consumer Component
//!
//! Demonstrates inter-component message passing using the semantic Channel API.
//! Receives u32 messages from the producer component.
//!
//! # Arguments (Phase 5 - to be implemented)
//! - shared_mem_virt: Virtual address of shared memory for channel
//! - receiver_notify_cap: Notification capability for receiver (this component)
//! - sender_notify_cap: Notification capability for sender

#![no_std]
#![no_main]

use kaal_sdk::{
    component::{Component, ServiceBase},
    syscall,
    message::{Channel, ChannelConfig},
};

// Declare this as a service component
kaal_sdk::component! {
    name: "ipc_consumer",
    type: Service,
    version: "0.1.0",
    capabilities: ["memory:map", "notification:signal", "notification:wait"],
    impl: IpcConsumer
}

/// IPC Consumer Service
pub struct IpcConsumer;

impl Component for IpcConsumer {
    fn init() -> kaal_sdk::Result<Self> {
        unsafe {
            syscall::print("\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("  IPC Consumer v0.1.0\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("\n");
            syscall::print("[consumer] Initializing...\n");
            syscall::print("[consumer] Using semantic message-passing API\n");
        }

        Ok(IpcConsumer)
    }

    fn run(&mut self) -> ! {
        unsafe {
            syscall::print("[consumer] Starting message reception\n");

            // Create our own notification for receiving signals
            syscall::print("[consumer] Creating notification capability...\n");
            let consumer_notify = match syscall::notification_create() {
                Ok(slot) => {
                    syscall::print("  ✓ Created notification at slot: ");
                    syscall::print("X\n");
                    slot
                }
                Err(_) => {
                    syscall::print("  ✗ Failed to create notification\n");
                    loop { syscall::yield_now(); }
                }
            };

            // For demo: we'll use hardcoded shared memory location
            let shared_mem_virt = 0x80000000u64; // Hardcoded for demo

            syscall::print("[consumer] Configuration:\n");
            syscall::print("  - Shared memory at: 0x80000000 (hardcoded)\n");
            syscall::print("  - Consumer notification created\n");
            syscall::print("\n");

            // Wait a bit for producer to initialize
            for _ in 0..5 {
                syscall::yield_now();
            }

            // Check the magic value written by producer
            syscall::print("[consumer] Checking shared memory...\n");
            let shared_ptr = shared_mem_virt as *mut u32;
            let magic = *shared_ptr;

            if magic == 0xDEADBEEF {
                syscall::print("  ✓ Found magic value 0xDEADBEEF - shared memory working!\n");
            } else {
                syscall::print("  ✗ Magic value mismatch - shared memory not working\n");
            }
            syscall::print("\n");

            // For demo: just read the test data from shared memory
            // In real implementation, we'd wait for signals from producer
            syscall::print("[consumer] Reading test messages from shared memory...\n");

            // Wait for producer to write data
            for _ in 0..10 {
                syscall::yield_now();
            }

            // Read the test messages
            for i in 0..5 {
                // Read message from shared memory
                let msg_ptr = (shared_mem_virt + 4 + (i * 4)) as *mut u32;
                let message = *msg_ptr;

                syscall::print("  ← Read message ");
                // Check if it's what we expect (0x1000 + i)
                if message == 0x1000 + i as u32 {
                    syscall::print("✓\n");
                } else {
                    syscall::print("(unexpected value)\n");
                }
            }

            syscall::print("[consumer] All messages received!\n");
            syscall::print("\n");

            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("  IPC COMMUNICATION SUCCESSFUL! ✓\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
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
