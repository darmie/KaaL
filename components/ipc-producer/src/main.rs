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
            syscall::print("[producer] NOTE: Waiting for Phase 5 implementation:\n");
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
            // // Create sender endpoint
            // let channel = Channel::<u32>::sender(config);
            //
            // syscall::print("[producer] Sending 10 messages...\n");
            //
            // // Send messages - blocking automatically handles full channel
            // for i in 0..10 {
            //     match channel.send(i) {
            //         Ok(()) => {
            //             syscall::print("[producer] Sent message: ");
            //             // TODO: Add print_number to syscall
            //             syscall::print("\n");
            //         }
            //         Err(_) => {
            //             syscall::print("[producer] Error sending message\n");
            //             break;
            //         }
            //     }
            // }
            //
            // syscall::print("[producer] All messages sent!\n");
            // syscall::print("[producer] Channel automatically signaled receiver\n");

            syscall::print("[producer] Entering yield loop\n");
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
