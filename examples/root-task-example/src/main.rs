//! # KaaL Root Task Example
//!
//! This is a minimal example demonstrating the KaaL root task infrastructure:
//! - VSpace (virtual address space) management
//! - CNode (capability space) management
//! - Component spawning framework
//!
//! ## Usage
//!
//! For Phase 1 (mock seL4):
//! ```bash
//! cargo run --bin root-task-example
//! ```
//!
//! Note: This is a regular Rust binary (not no_std) for Phase 1 testing.
//! Phase 2 will create a true no_std root task binary.

use kaal_root_task::{RootTaskConfig, VSpaceManager, CNodeManager, ComponentSpawner, ComponentInfo, RootTaskError};

fn main() {
    println!("=== KaaL Root Task Example ===\n");

    // Step 1: Configuration
    println!("1. Creating root task configuration...");
    let config = RootTaskConfig {
        heap_size: 8 * 1024 * 1024,      // 8MB heap
        cspace_size: 8192,                // 8K capability slots
        vspace_size: 2 * 1024 * 1024 * 1024, // 2GB virtual address space
    };
    println!("   Heap: {} MB", config.heap_size / (1024 * 1024));
    println!("   CSpace: {} slots", config.cspace_size);
    println!("   VSpace: {} GB\n", config.vspace_size / (1024 * 1024 * 1024));

    // Step 2: VSpace Management
    println!("2. Demonstrating VSpace management...");
    let mut vspace = VSpaceManager::new(
        2,          // VSpace root capability
        0x1000_0000, // Base address
        config.vspace_size,
    );

    match vspace.allocate(4096) {
        Ok(vaddr) => println!("   Allocated 4KB at virtual address: 0x{:x}", vaddr),
        Err(_) => println!("   Failed to allocate virtual memory"),
    }

    match vspace.allocate(1024 * 1024) {
        Ok(vaddr) => println!("   Allocated 1MB at virtual address: 0x{:x}\n", vaddr),
        Err(_) => println!("   Failed to allocate virtual memory\n"),
    }

    // Step 3: CNode Management
    println!("3. Demonstrating CNode management...");
    let mut cnode = CNodeManager::new(
        1,    // CNode root capability
        100,  // Start at slot 100
        1000, // Total 1000 slots
    );

    for i in 0..3 {
        match cnode.allocate() {
            Ok(slot) => println!("   Allocated capability slot: {}", slot),
            Err(_) => println!("   Failed to allocate slot"),
        }
    }
    println!();

    // Step 4: Component Spawning
    println!("4. Demonstrating component spawning framework...");

    let spawner = ComponentSpawner::new(vspace, cnode);
    println!("   Created component spawner");
    println!("   Ready to spawn components with:");
    println!("     - Automatic stack allocation");
    println!("     - TCB (thread) management");
    println!("     - Priority scheduling\n");

    // Example component info (not actually spawned in Phase 1)
    let serial_component = ComponentInfo {
        name: "serial-driver",
        entry_point: 0x2000_0000, // Example address
        stack_size: 16 * 1024,
        priority: 200,
    };

    println!("   Example component configuration:");
    println!("     Name: {}", serial_component.name);
    println!("     Entry point: 0x{:x}", serial_component.entry_point);
    println!("     Stack size: {} KB", serial_component.stack_size / 1024);
    println!("     Priority: {}\n", serial_component.priority);

    // Step 5: Summary
    println!("=== Summary ===");
    println!("✅ Root task configuration created");
    println!("✅ VSpace manager operational");
    println!("✅ CNode manager operational");
    println!("✅ Component spawner framework ready");
    println!("\nPhase 2 TODO:");
    println!("  - Integrate with real seL4 kernel");
    println!("  - Implement actual component spawning");
    println!("  - Add system service initialization");
    println!("  - Create no_std root task binary");

    let _ = spawner; // Prevent unused warning
}
