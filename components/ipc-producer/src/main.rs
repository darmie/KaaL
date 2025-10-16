//! IPC Producer Component
//!
//! Demonstrates inter-component IPC using SharedRing from kaal-ipc.
//! Produces u32 values and signals the consumer via notifications.
//!
//! # Arguments (passed via entry point - Phase 5 TODO)
//! - shared_mem_virt: Virtual address of shared memory containing SharedRing
//! - consumer_notify_cap: Capability slot for consumer notification
//! - producer_notify_cap: Capability slot for producer notification

#![no_std]
#![no_main]

use kaal_sdk::{
    component::{Component, ServiceBase},
    syscall,
    ipc::SharedRing,
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
        }

        Ok(IpcProducer)
    }

    fn run(&mut self) -> ! {
        unsafe {
            syscall::print("[producer] Starting production loop\n");
            syscall::print("[producer] NOTE: Waiting for Phase 5 implementation:\n");
            syscall::print("  - shared memory mapping\n");
            syscall::print("  - capability passing\n");
            syscall::print("  - entry point arguments\n");
            syscall::print("\n");

            // Phase 5 implementation will look like:
            //
            // // Get arguments from entry point (passed by root-task)
            // let shared_mem_virt = /* from args */;
            // let consumer_cap = 102;  // Capability slot
            // let producer_cap = 103;  // Capability slot
            //
            // // Access SharedRing in mapped memory
            // let ring = &*(shared_mem_virt as *const SharedRing<u32, 256>);
            //
            // // Produce 10 items
            // for i in 0..10 {
            //     loop {
            //         match ring.push(i) {
            //             Ok(()) => {
            //                 syscall::print("[producer] Produced: ");
            //                 // print number
            //                 syscall::print("\n");
            //                 break;
            //             }
            //             Err(kaal_sdk::ipc::IpcError::BufferFull { .. }) => {
            //                 // Wait for consumer to make space
            //                 let _ = ring.wait_producer();
            //             }
            //             Err(_) => break,
            //         }
            //     }
            // }
            //
            // syscall::print("[producer] Production complete!\n");

            syscall::print("[producer] Entering idle loop (yielding)\n");
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
