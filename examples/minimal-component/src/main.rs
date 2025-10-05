//! # Minimal KaaL Component Example
//!
//! This is the **absolute minimum** needed to build a KaaL system.
//! Perfect for hobbyists getting started!
//!
//! ## What This Does
//!
//! 1. Initializes the root task
//! 2. Spawns ONE simple component (hello_world)
//! 3. Component prints a message
//! 4. System enters idle loop
//!
//! ## How to Run
//!
//! ```bash
//! cargo run --bin minimal-component
//! ```
//!
//! ## How to Extend
//!
//! 1. **Add More Components**: Copy the `spawn_hello_component` pattern
//! 2. **Add Drivers**: Use DDDK to add hardware drivers
//! 3. **Add Services**: Implement VFS, network, etc.
//!
//! The sky's the limit - start small, build incrementally!

use kaal_root_task::{RootTask, RootTaskConfig};
use cap_broker::{ComponentConfig, ComponentSpawner, DEFAULT_STACK_SIZE};

/// Simple "Hello World" component entry point
///
/// This is what runs when the component starts.
/// In a real system, this would be in a separate binary.
pub extern "C" fn hello_world_component() -> ! {
    // In a real system, you'd have actual work here:
    // - Read from IPC
    // - Process data
    // - Send results

    // For now, just demonstrate the component is running
    #[cfg(not(feature = "sel4-real"))]
    {
        println!("👋 Hello from KaaL component!");
        println!("   • Component is isolated");
        println!("   • Has private address space");
        println!("   • Can communicate via IPC");
    }

    // Component main loop
    loop {
        #[cfg(feature = "sel4-real")]
        unsafe {
            // In real seL4:
            // - Wait for IPC messages
            // - Process requests
            // - Send replies
            sel4_sys::seL4_Yield();
        }

        #[cfg(not(feature = "sel4-real"))]
        {
            // Mock mode: just loop
            core::hint::spin_loop();
        }
    }
}

/// Extend the root task with your own components
///
/// This shows how hobbyists can add their own functionality
/// to the KaaL system.
fn spawn_my_components(broker: &mut cap_broker::DefaultCapBroker) {
    println!("\n🚀 Spawning Components...");
    println!("─────────────────────────────────────────");

    // Get bootinfo for capability slots
    let bootinfo = unsafe {
        cap_broker::BootInfo::get().expect("Failed to get bootinfo")
    };

    // Create component spawner
    let mut spawner = ComponentSpawner::new(
        bootinfo.cspace_root,
        bootinfo.vspace_root,
        0x4000_0000,          // VSpace base
        256 * 1024 * 1024,    // 256MB VSpace
    );

    // Capability slot allocator
    let mut next_slot = bootinfo.empty.start;
    let mut slot_allocator = || {
        let slot = next_slot;
        next_slot += 1;
        Ok(slot)
    };

    // Spawn hello world component
    let hello_config = ComponentConfig {
        name: "hello_world",
        entry_point: hello_world_component as usize,
        stack_size: DEFAULT_STACK_SIZE,
        priority: 100,
        device: None, // No hardware needed
        fault_ep: None,
    };

    match spawner.spawn_component(hello_config, &mut slot_allocator, 10) {
        Ok(component) => {
            println!("  ✓ Spawned: {}", component.name());

            // Start the component
            if let Err(e) = spawner.start_component(&component) {
                eprintln!("  ✗ Failed to start: {:?}", e);
            } else {
                println!("  ✓ Started: {} (running!)", component.name());
            }
        }
        Err(e) => {
            eprintln!("  ✗ Failed to spawn: {:?}", e);
        }
    }

    println!("\n✅ System Ready!");
    println!("   • {} component(s) running", spawner.running_component_count());
}

fn main() {
    println!("\n╔═══════════════════════════════════════════════╗");
    println!("║        Minimal KaaL System Example          ║");
    println!("║     Perfect for Hobbyists Getting Started   ║");
    println!("╚═══════════════════════════════════════════════╝\n");

    unsafe {
        // Step 1: Initialize root task
        println!("📋 Step 1: Initialize Root Task");
        println!("─────────────────────────────────────────");

        let config = RootTaskConfig::default();
        println!("  • Heap: {} MB", config.heap_size / (1024 * 1024));
        println!("  • CSpace: {} slots", config.cspace_size);
        println!("  • VSpace: {} GB", config.vspace_size / (1024 * 1024 * 1024));

        let mut root = match RootTask::init(config) {
            Ok(r) => {
                println!("  ✓ Root task initialized");
                r
            }
            Err(e) => {
                eprintln!("  ✗ Failed: {:?}", e);
                return;
            }
        };

        // Step 2 & 3: Run with composable pattern
        println!("\n📋 Step 2 & 3: Run System (Composable Pattern)");
        println!("─────────────────────────────────────────");
        println!("  • Using run_with() for composability");
        println!("  • Spawning components via closure");
        println!("\n💡 To add more:");
        println!("   1. Create component entry function");
        println!("   2. Add to the closure in run_with()");
        println!("   3. Rebuild and run!");
        println!("\n🎉 Starting KaaL system (composable)!\n");

        // This never returns - composable pattern!
        root.run_with(|broker| {
            // Your components spawn here - clean and composable!
            println!("\n🚀 Spawning Components (via callback)...");
            println!("─────────────────────────────────────────");
            spawn_my_components(broker);
        });
    }
}
