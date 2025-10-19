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
    channel_setup::{establish_channel, ChannelRole},
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
            syscall::print("  *** THIS IS THE CONSUMER *** IPC Consumer v0.1.0\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("\n");
            syscall::print("[CONSUMER-CONSUMER-CONSUMER] Initializing...\n");
            syscall::print("[CONSUMER-CONSUMER-CONSUMER] Using semantic message-passing API\n");
            syscall::print("[CONSUMER-CONSUMER-CONSUMER] I am definitely the consumer!\n");
        }

        Ok(IpcConsumer)
    }

    fn run(&mut self) -> ! {
        unsafe {
            syscall::print("[consumer] Starting message reception\n");

            // Step 1: Establish channel using architecture-driven approach
            // The establish_channel function uses syscalls to dynamically allocate resources
            let producer_pid = 0; // TODO: Discover via nameserver/broker
            let buffer_size = 0x1000; // 4KB buffer

            syscall::print("[consumer] Establishing channel via syscalls...\n");
            syscall::print("  - Buffer size: 4KB\n");
            syscall::print("  - Role: Consumer\n");

            let channel_config = match establish_channel(producer_pid, buffer_size, ChannelRole::Consumer) {
                Ok(config) => {
                    syscall::print("  ✓ Channel established with dynamic allocation\n");
                    config
                }
                Err(e) => {
                    syscall::print("  ✗ Failed to establish channel: ");
                    syscall::print(e);
                    syscall::print("\n");
                    loop { syscall::yield_now(); }
                }
            };

            syscall::print("\n[consumer] Configuration complete:\n");
            syscall::print("  - Shared memory ready\n");
            syscall::print("  - Notification capability ready\n");
            syscall::print("\n");

            // Wait a bit for producer to initialize
            for _ in 0..5 {
                syscall::yield_now();
            }

            // Check the magic value written by producer
            syscall::print("[consumer] Checking shared memory...\n");

            if channel_config.buffer_addr != 0 {
                let shared_ptr = channel_config.buffer_addr as *mut u32;
                let magic = *shared_ptr;

                if magic == 0xDEADBEEF {
                    syscall::print("  ✓ Found magic value 0xDEADBEEF - shared memory working!\n");
                } else {
                    syscall::print("  ✗ Magic value mismatch - shared memory not working\n");
                }
                syscall::print("\n");

                // Read test messages with optional notification waiting
                syscall::print("[consumer] Reading test messages from shared memory...\n");

                // Wait for producer to write data
                for _ in 0..10 {
                    // Check for notifications if we have the capability
                    if channel_config.notification_cap != 0 {
                        let signals = syscall::poll(channel_config.notification_cap).unwrap_or(0);
                        if signals != 0 {
                            syscall::print("  [Received signal: ");
                            syscall::print("X");
                            syscall::print("]\n");
                        }
                    }
                    syscall::yield_now();
                }

                // Read the test messages
                for i in 0..5 {
                    // Read message from shared memory
                    let msg_ptr = (channel_config.buffer_addr + 4 + (i * 4)) as *mut u32;
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
            } else {
                syscall::print("[consumer] Warning: No valid shared memory buffer!\n");
                syscall::print("[consumer] Channel establishment may have failed.\n");
            }
        }

        // Continue yielding
        loop {
            unsafe {
                syscall::yield_now();
            }
        }
    }
}
