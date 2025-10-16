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
            syscall::print("[consumer] NOTE: Waiting for Phase 5 implementation:\n");
            syscall::print("  - shared memory mapping\n");
            syscall::print("  - capability passing via spawn\n");
            syscall::print("  - entry point arguments\n");
            syscall::print("\n");

            // Phase 5 implementation will look like:
            //
            // // Get configuration from entry point arguments
            // let config = ChannelConfig {
            //     shared_memory: shared_mem_virt,  // From spawn args
            //     receiver_notify: 102,             // Notification cap slot
            //     sender_notify: 103,               // Notification cap slot
            // };
            //
            // // Create receiver endpoint
            // let channel = Channel::<u32>::receiver(config);
            //
            // syscall::print("[consumer] Receiving 10 messages...\n");
            //
            // // Receive messages - blocking automatically handles empty channel
            // for i in 0..10 {
            //     match channel.receive() {
            //         Ok(message) => {
            //             syscall::print("[consumer] Received message: ");
            //             // TODO: Add print_number to syscall
            //             syscall::print("\n");
            //         }
            //         Err(_) => {
            //             syscall::print("[consumer] Error receiving message\n");
            //             break;
            //         }
            //     }
            // }
            //
            // syscall::print("[consumer] All messages received!\n");
            // syscall::print("[consumer] Channel automatically signaled sender\n");

            syscall::print("[consumer] Entering yield loop\n");
            syscall::print("\n");
        }

        // Cooperative multitasking - yield to other components
        loop {
            unsafe {
                syscall::yield_now();
            }
        }
    }
}
