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
    channel_setup::{establish_channel, ChannelRole},
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
pub struct IpcProducer {
    instance_id: u32,
}

// Static counter to track instances
static mut INSTANCE_COUNTER: u32 = 0;

impl Component for IpcProducer {
    fn init() -> kaal_sdk::Result<Self> {
        let instance_id = unsafe {
            INSTANCE_COUNTER += 1;
            INSTANCE_COUNTER
        };

        unsafe {
            syscall::print("\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("  IPC Producer v0.1.0 [INSTANCE #");
            // Print instance number
            let mut n = instance_id;
            if n == 0 {
                syscall::print("0");
            } else {
                let mut digits = [0u8; 10];
                let mut i = 0;
                while n > 0 {
                    digits[i] = b'0' + (n % 10) as u8;
                    n /= 10;
                    i += 1;
                }
                while i > 0 {
                    i -= 1;
                    let digit_str = core::str::from_utf8_unchecked(&digits[i..i+1]);
                    syscall::print(digit_str);
                }
            }
            syscall::print("]\n");
            syscall::print("═══════════════════════════════════════════════════════════\n");
            syscall::print("\n");
            syscall::print("[producer] Initializing...\n");
            syscall::print("[producer] Using semantic message-passing API\n");
        }

        Ok(IpcProducer { instance_id })
    }

    fn run(&mut self) -> ! {
        unsafe {
            syscall::print("[producer] Starting message production\n");

            // Step 1: Establish channel using architecture-driven approach
            // The establish_channel function uses syscalls to dynamically allocate resources
            let channel_name = "kaal.ipc.producer_consumer";
            let buffer_size = 0x1000; // 4KB buffer

            syscall::print("[producer] Establishing channel via syscalls...\n");
            syscall::print("  - Channel: ");
            syscall::print(channel_name);
            syscall::print("\n");
            syscall::print("  - Buffer size: 4KB\n");
            syscall::print("  - Role: Producer\n");

            let channel_config = match establish_channel(channel_name, buffer_size, ChannelRole::Producer) {
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

            syscall::print("\n[producer] Configuration complete:\n");
            syscall::print("  - Shared memory ready\n");
            syscall::print("  - Notification capability ready\n");
            syscall::print("\n");

            // Initialize the shared ring buffer
            syscall::print("[producer] Initializing SharedRing buffer...\n");

            // Write a magic value to shared memory to test it's working
            let shared_ptr = channel_config.buffer_addr as *mut u32;
            if shared_ptr as usize != 0 {
                *shared_ptr = 0xDEADBEEF;
                syscall::print("[producer] Wrote magic value 0xDEADBEEF to shared memory\n");

                // Write test data to shared memory
                syscall::print("[producer] Writing test data to shared memory...\n");
                for i in 0..5 {
                    // Write message to shared memory (simplified - no ring buffer yet)
                    let msg_ptr = (channel_config.buffer_addr + 4 + (i * 4)) as *mut u32;
                    *msg_ptr = 0x1000 + i as u32;

                    syscall::print("  → Wrote test message ");
                    syscall::print("X\n");

                    // Signal consumer if we have notification capability
                    if channel_config.notification_cap != 0 {
                        let _ = syscall::signal(channel_config.notification_cap, 1 << i);
                    }

                    // Yield to let consumer see the data
                    for _ in 0..5 {
                        syscall::yield_now();
                    }
                }

                syscall::print("[producer] All test data written!\n");
                syscall::print("\n");
            } else {
                syscall::print("[producer] Warning: No valid shared memory buffer!\n");
                syscall::print("[producer] Channel establishment may have failed.\n");
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
