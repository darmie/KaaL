//! IPC Consumer Component
//!
//! Demonstrates inter-component IPC using SharedRing from kaal-ipc.
//! Consumes u32 values produced by ipc-producer.
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
        }

        Ok(IpcConsumer)
    }

    fn run(&mut self) -> ! {
        unsafe {
            syscall::print("[consumer] Starting consumption loop\n");
            syscall::print("[consumer] NOTE: Waiting for Phase 5 implementation:\n");
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
            // // Consume items
            // let mut count = 0;
            // loop {
            //     // Wait for data
            //     let _ = ring.wait_consumer();
            //
            //     // Pop all available items
            //     while let Ok(item) = ring.pop() {
            //         syscall::print("[consumer] Consumed: ");
            //         // print number
            //         syscall::print("\n");
            //         count += 1;
            //
            //         if count >= 10 {
            //             syscall::print("[consumer] All items consumed!\n");
            //             break;
            //         }
            //     }
            //
            //     if count >= 10 {
            //         break;
            //     }
            // }

            syscall::print("[consumer] Entering idle loop (yielding)\n");
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
