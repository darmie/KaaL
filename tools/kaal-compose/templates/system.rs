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

use cap_broker::{BootInfo, ComponentConfig, ComponentSpawner, DefaultCapBroker, DeviceId, DEFAULT_STACK_SIZE};
use core::panic::PanicInfo;

// Simple bump allocator for demonstration
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;

struct BumpAllocator {
    heap: UnsafeCell<[u8; 8 * 1024 * 1024]>, // 8MB heap
    next: UnsafeCell<usize>,
}

unsafe impl Sync for BumpAllocator {}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut next = *self.next.get();
        let align = layout.align();
        let size = layout.size();

        // Align up
        next = (next + align - 1) & !(align - 1);

        let alloc_start = next;
        next += size;

        if next > 8 * 1024 * 1024 {
            return core::ptr::null_mut();
        }

        *self.next.get() = next;
        self.heap.get().cast::<u8>().add(alloc_start)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't deallocate
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator {
    heap: UnsafeCell::new([0; 8 * 1024 * 1024]),
    next: UnsafeCell::new(0),
};

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
        // ============================================================
        // STEP 1: Parse Bootinfo from seL4 Kernel
        // ============================================================
        let bootinfo = match BootInfo::get() {
            Ok(bi) => bi,
            Err(_) => halt(),
        };

        // ============================================================
        // STEP 2: Initialize Default Capability Broker
        // ============================================================
        let mut broker = match DefaultCapBroker::init() {
            Ok(b) => b,
            Err(_) => halt(),
        };

        // ============================================================
        // STEP 3: Create Component Spawner
        // ============================================================
        let mut spawner = ComponentSpawner::new(
            bootinfo.cspace_root,
            bootinfo.vspace_root,
            0x4000_0000,          // VSpace base address (1GB)
            256 * 1024 * 1024,    // VSpace size (256MB)
        );

        // ============================================================
        // STEP 4: Spawn Stub Driver Component
        // ============================================================
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
            Ok(component) => component,
            Err(_) => halt(),
        };

        let _ = spawner.start_component(&driver);

        // ============================================================
        // STEP 5: Root Task Main Loop
        // ============================================================
        loop {
            // Wait for IPC messages from components
            core::arch::asm!("wfi");
        }
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
