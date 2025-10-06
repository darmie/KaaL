//! # KaaL System Template
//!
//! Complete no_std seL4 system demonstrating the full workflow:
//! 1. Parse bootinfo from seL4 kernel
//! 2. Initialize default Capability Broker
//! 3. Set up Component Spawner
//! 4. Spawn stub driver component
//! 5. Root task main loop initialization
//!
//! This follows the KaaL convention for system composition.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use cap_broker::{BootInfo, ComponentConfig, ComponentSpawner, DefaultCapBroker, DeviceId, DEFAULT_STACK_SIZE};
use core::panic::PanicInfo;

// ============================================================
// Component Entry Points
// ============================================================

/// Stub driver component entry point
///
/// Replace this with your actual driver logic.
/// In a real system, this would:
/// - Initialize hardware registers
/// - Set up interrupt handlers
/// - Enter service loop waiting for IPC requests
pub extern "C" fn stub_driver_main() -> ! {
    unsafe {
        debug_print(b"[Driver] Starting...\n");
        debug_print(b"[Driver] Initialized\n");
    }

    // Driver main loop
    loop {
        unsafe {
            // Wait for interrupts or IPC messages
            // In real seL4: seL4_Wait() or seL4_Poll()
            core::arch::asm!("wfi");
        }
    }
}

// ============================================================
// Root Task Entry Point
// ============================================================

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        debug_print(b"\n================================================\n");
        debug_print(b"  KaaL System Starting\n");
        debug_print(b"================================================\n\n");

        // ============================================================
        // STEP 1: Parse Bootinfo from seL4 Kernel
        // ============================================================
        debug_print(b"[1/5] Parsing bootinfo...\n");

        let bootinfo = match BootInfo::get() {
            Ok(bi) => {
                debug_print(b"  Bootinfo parsed\n");
                bi
            }
            Err(_) => {
                debug_print(b"ERROR: Failed to parse bootinfo\n");
                halt();
            }
        };

        // ============================================================
        // STEP 2: Initialize Default Capability Broker
        // ============================================================
        debug_print(b"\n[2/5] Initializing Capability Broker...\n");

        let mut broker = match DefaultCapBroker::init() {
            Ok(b) => {
                debug_print(b"  Capability Broker ready\n");
                b
            }
            Err(_) => {
                debug_print(b"ERROR: Failed to initialize broker\n");
                halt();
            }
        };

        // ============================================================
        // STEP 3: Create Component Spawner
        // ============================================================
        debug_print(b"\n[3/5] Setting up Component Spawner...\n");

        let mut spawner = ComponentSpawner::new(
            bootinfo.cspace_root,
            bootinfo.vspace_root,
            0x4000_0000,          // VSpace base address (1GB)
            256 * 1024 * 1024,    // VSpace size (256MB)
        );
        debug_print(b"  Component Spawner initialized\n");

        // ============================================================
        // STEP 4: Spawn Stub Driver Component
        // ============================================================
        debug_print(b"\n[4/5] Spawning stub driver...\n");

        // Capability slot allocator
        let mut next_slot = bootinfo.empty.start;
        let mut slot_allocator = || {
            let slot = next_slot;
            next_slot += 1;
            Ok(slot)
        };

        let driver_config = ComponentConfig {
            name: "stub_driver",
            entry_point: stub_driver_main as usize,
            stack_size: DEFAULT_STACK_SIZE,
            priority: 100,
            device: Some(DeviceId::Serial { port: 0 }),
            fault_ep: None,
        };

        let driver = match spawner.spawn_component_with_device(
            driver_config,
            &mut slot_allocator,
            10, // untyped_cap
            &mut broker,
        ) {
            Ok(component) => {
                debug_print(b"  Driver component spawned\n");
                component
            }
            Err(_) => {
                debug_print(b"ERROR: Failed to spawn driver\n");
                halt();
            }
        };

        match spawner.start_component(&driver) {
            Ok(_) => debug_print(b"  Driver started\n"),
            Err(_) => debug_print(b"ERROR: Failed to start driver\n"),
        }

        // ============================================================
        // STEP 5: Root Task Initialization Complete
        // ============================================================
        debug_print(b"\n[5/5] Initialization complete\n");
        debug_print(b"\n================================================\n");
        debug_print(b"  System Ready\n");
        debug_print(b"================================================\n\n");

        debug_print(b"Components: stub_driver (priority 100)\n");
        debug_print(b"Root task entering main loop...\n\n");

        // ============================================================
        // Root Task Main Loop
        // ============================================================
        loop {
            // Wait for IPC messages from components
            core::arch::asm!("wfi");
        }
    }
}

// ============================================================
// Helper Functions
// ============================================================

/// Print debug message via seL4 debug console
unsafe fn debug_print(msg: &[u8]) {
    for &byte in msg {
        sel4_platform::adapter::seL4_DebugPutChar(byte);
    }
}

/// Halt system on critical error
fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    halt()
}
