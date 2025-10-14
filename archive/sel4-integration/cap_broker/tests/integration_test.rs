//! Integration tests for the complete capability broker system
//!
//! These tests demonstrate end-to-end workflows combining:
//! - Bootinfo parsing
//! - Component spawning
//! - TCB management
//! - VSpace management
//! - Device allocation
//! - IPC setup

use cap_broker::*;

/// Test complete system initialization from bootinfo
#[test]
fn test_full_system_initialization() {
    unsafe {
        // 1. Get bootinfo (Phase 1: mock)
        let bootinfo = BootInfo::get().expect("Failed to get bootinfo");

        // Verify bootinfo has critical capabilities
        assert_eq!(bootinfo.cspace_root, 1);
        assert_eq!(bootinfo.vspace_root, 2);
        assert_eq!(bootinfo.tcb, 3);
        assert_eq!(bootinfo.irq_control, 4);

        // Verify untyped regions available
        assert!(!bootinfo.untyped.is_empty());
    }
}

/// Test spawning a simple component with all subsystems
#[test]
fn test_spawn_component_full_workflow() {
    // Create component spawner
    let mut spawner = ComponentSpawner::new(
        1,  // cspace_root from bootinfo
        2,  // vspace_root from bootinfo
        0x4000_0000,  // Virtual address base
        256 * 1024 * 1024,  // 256MB address space
    );

    // Mock capability allocator
    let mut next_slot = 100;
    let mut allocator = || {
        let slot = next_slot;
        next_slot += 1;
        Ok(slot)
    };

    // Configure component
    let config = ComponentConfig {
        name: "serial_driver",
        entry_point: 0x400000,  // Hypothetical driver entry point
        stack_size: 64 * 1024,
        priority: 150,
        device: None,  // TODO: Test with device allocation
        fault_ep: None,
    };

    // Spawn component
    let component = spawner
        .spawn_component(config, &mut allocator, 10)
        .expect("Failed to spawn component");

    // Verify component creation
    assert_eq!(component.name(), "serial_driver");
    assert!(component.tcb_cap() >= 100);
    assert!(component.endpoint() >= 100);
    assert!(component.notification() >= 100);

    // Verify spawner state
    assert_eq!(spawner.component_count(), 1);
    assert_eq!(spawner.running_component_count(), 0);

    // Start component
    spawner
        .start_component(&component)
        .expect("Failed to start component");

    assert_eq!(spawner.running_component_count(), 1);

    // Stop component
    spawner
        .stop_component(&component)
        .expect("Failed to stop component");

    assert_eq!(spawner.running_component_count(), 0);
}

/// Test spawning multiple components with resource isolation
#[test]
fn test_multiple_component_isolation() {
    let mut spawner = ComponentSpawner::new(1, 2, 0x4000_0000, 512 * 1024 * 1024);

    let mut next_slot = 100;
    let mut allocator = || {
        let slot = next_slot;
        next_slot += 1;
        Ok(slot)
    };

    // Spawn serial driver
    let serial_config = ComponentConfig {
        name: "serial_driver",
        entry_point: 0x400000,
        stack_size: 64 * 1024,
        priority: 150,
        device: None,
        fault_ep: None,
    };

    let serial = spawner
        .spawn_component(serial_config, &mut allocator, 10)
        .expect("Failed to spawn serial driver");

    // Spawn network driver
    let network_config = ComponentConfig {
        name: "network_driver",
        entry_point: 0x500000,
        stack_size: 128 * 1024,
        priority: 140,
        device: None,
        fault_ep: None,
    };

    let network = spawner
        .spawn_component(network_config, &mut allocator, 11)
        .expect("Failed to spawn network driver");

    // Spawn filesystem component
    let fs_config = ComponentConfig {
        name: "filesystem",
        entry_point: 0x600000,
        stack_size: 256 * 1024,
        priority: 100,
        device: None,
        fault_ep: None,
    };

    let filesystem = spawner
        .spawn_component(fs_config, &mut allocator, 12)
        .expect("Failed to spawn filesystem");

    // Verify all components created
    assert_eq!(spawner.component_count(), 3);

    // Start all components
    spawner.start_component(&serial).unwrap();
    spawner.start_component(&network).unwrap();
    spawner.start_component(&filesystem).unwrap();

    assert_eq!(spawner.running_component_count(), 3);

    // Stop network driver
    spawner.stop_component(&network).unwrap();
    assert_eq!(spawner.running_component_count(), 2);

    // Verify different components have different capabilities (isolation)
    assert_ne!(serial.tcb_cap(), network.tcb_cap());
    assert_ne!(serial.endpoint(), network.endpoint());
    assert_ne!(network.tcb_cap(), filesystem.tcb_cap());
}

/// Test VSpace allocation across components
#[test]
fn test_vspace_allocation_multiple_components() {
    let mut spawner = ComponentSpawner::new(1, 2, 0x4000_0000, 64 * 1024 * 1024);

    let initial_vspace = spawner.available_vspace();
    assert_eq!(initial_vspace, 64 * 1024 * 1024);

    let mut next_slot = 100;
    let mut allocator = || {
        let slot = next_slot;
        next_slot += 1;
        Ok(slot)
    };

    // Spawn component with 64KB stack
    let config = ComponentConfig {
        name: "comp1",
        entry_point: 0x400000,
        stack_size: 64 * 1024,
        priority: 100,
        device: None,
        fault_ep: None,
    };

    spawner
        .spawn_component(config, &mut allocator, 10)
        .expect("Failed to spawn component");

    // VSpace should decrease by stack + IPC buffer
    let expected_used = 64 * 1024 + IPC_BUFFER_SIZE;
    let remaining = spawner.available_vspace();
    assert!(remaining < initial_vspace);
    assert_eq!(remaining, initial_vspace - expected_used);
}

/// Test component configuration variations
#[test]
fn test_component_configuration_variations() {
    let mut spawner = ComponentSpawner::new(1, 2, 0x4000_0000, 256 * 1024 * 1024);

    let mut next_slot = 100;
    let mut allocator = || {
        let slot = next_slot;
        next_slot += 1;
        Ok(slot)
    };

    // High priority component
    let high_prio = ComponentConfig {
        name: "critical_service",
        entry_point: 0x400000,
        stack_size: DEFAULT_STACK_SIZE,
        priority: MAX_PRIORITY,
        device: None,
        fault_ep: None,
    };

    let comp1 = spawner
        .spawn_component(high_prio, &mut allocator, 10)
        .unwrap();

    // Low priority component with large stack
    let low_prio = ComponentConfig {
        name: "background_worker",
        entry_point: 0x500000,
        stack_size: 512 * 1024,  // 512KB stack
        priority: 50,
        device: None,
        fault_ep: None,
    };

    let comp2 = spawner
        .spawn_component(low_prio, &mut allocator, 11)
        .unwrap();

    // Default configuration
    let default_config = ComponentConfig {
        name: "default_component",
        entry_point: 0x600000,
        ..Default::default()
    };

    let comp3 = spawner
        .spawn_component(default_config, &mut allocator, 12)
        .unwrap();

    assert_eq!(spawner.component_count(), 3);
}

/// Test TCB management integration
#[test]
fn test_tcb_lifecycle_via_spawner() {
    let mut spawner = ComponentSpawner::new(1, 2, 0x4000_0000, 256 * 1024 * 1024);

    let mut next_slot = 100;
    let mut allocator = || {
        let slot = next_slot;
        next_slot += 1;
        Ok(slot)
    };

    let config = ComponentConfig {
        name: "test_tcb",
        entry_point: 0x400000,
        stack_size: 64 * 1024,
        priority: 100,
        device: None,
        fault_ep: None,
    };

    // Spawn creates and configures TCB
    let component = spawner
        .spawn_component(config, &mut allocator, 10)
        .unwrap();

    // Component not running initially
    assert_eq!(spawner.running_component_count(), 0);

    // Start resumes TCB
    spawner.start_component(&component).unwrap();
    assert_eq!(spawner.running_component_count(), 1);

    // Stop suspends TCB
    spawner.stop_component(&component).unwrap();
    assert_eq!(spawner.running_component_count(), 0);

    // Can restart
    spawner.start_component(&component).unwrap();
    assert_eq!(spawner.running_component_count(), 1);
}

/// Demonstrate complete system initialization workflow
#[test]
fn test_complete_system_workflow() {
    // This test demonstrates how a real root task would initialize the system

    // 1. Get bootinfo from kernel
    let bootinfo = unsafe { BootInfo::get().expect("Failed to get bootinfo") };

    // 2. Extract critical capabilities
    let cspace_root = bootinfo.cspace_root;
    let vspace_root = bootinfo.vspace_root;
    let irq_control = bootinfo.irq_control;

    // 3. Create component spawner
    let mut spawner = ComponentSpawner::new(
        cspace_root,
        vspace_root,
        0x4000_0000,
        256 * 1024 * 1024,
    );

    // 4. Mock capability allocator (in real system, this would be CSpace allocator)
    let mut next_slot = 100;
    let mut allocator = || {
        let slot = next_slot;
        next_slot += 1;
        Ok(slot)
    };

    // 5. Spawn system components
    let components = vec![
        ComponentConfig {
            name: "serial_driver",
            entry_point: 0x400000,
            stack_size: 64 * 1024,
            priority: 150,
            device: None,
            fault_ep: None,
        },
        ComponentConfig {
            name: "timer_driver",
            entry_point: 0x410000,
            stack_size: 64 * 1024,
            priority: 150,
            device: None,
            fault_ep: None,
        },
        ComponentConfig {
            name: "filesystem",
            entry_point: 0x500000,
            stack_size: 128 * 1024,
            priority: 100,
            device: None,
            fault_ep: None,
        },
    ];

    let mut spawned = Vec::new();
    for config in components {
        let comp = spawner
            .spawn_component(config, &mut allocator, 10)
            .expect("Failed to spawn component");
        spawned.push(comp);
    }

    // 6. Start all components
    for comp in &spawned {
        spawner
            .start_component(comp)
            .expect("Failed to start component");
    }

    // 7. Verify system state
    assert_eq!(spawner.component_count(), 3);
    assert_eq!(spawner.running_component_count(), 3);

    // System is now running with isolated components!
}

/// Test spawning a component with device access
#[test]
fn test_spawn_component_with_device() {
    // Initialize capability broker
    let mut broker = unsafe {
        DefaultCapBroker::init().expect("Failed to init broker")
    };

    // Create component spawner
    let mut spawner = ComponentSpawner::new(
        1,  // cspace_root
        2,  // vspace_root
        0x4000_0000,
        256 * 1024 * 1024,
    );

    // Configure component with device
    let config = ComponentConfig {
        name: "serial_driver",
        entry_point: 0x400000,
        stack_size: 64 * 1024,
        priority: 150,
        device: Some(DeviceId::Serial { port: 0 }),
        fault_ep: None,
    };

    // Allocator
    let mut next_slot = 100;
    let mut allocator = || {
        let slot = next_slot;
        next_slot += 1;
        Ok(slot)
    };

    // Spawn component with device
    let component = spawner
        .spawn_component_with_device(config, allocator, 10, &mut broker)
        .expect("Failed to spawn component with device");

    // Verify component has device bundle
    assert!(component.device_bundle().is_some());

    let device_bundle = component.device_bundle().unwrap();

    // Verify device resources
    // Serial port has IRQ 4
    assert_eq!(device_bundle.irq.irq_num(), 4);

    // Start the component
    spawner
        .start_component(&component)
        .expect("Failed to start component");

    assert_eq!(spawner.running_component_count(), 1);
}

/// Test full driver workflow: spawn with device, start, communicate
#[test]
fn test_full_driver_workflow() {
    // This demonstrates the complete lifecycle of a hardware driver component

    // 1. Initialize system
    let mut broker = unsafe {
        DefaultCapBroker::init().expect("Failed to init broker")
    };

    let mut spawner = ComponentSpawner::new(1, 2, 0x4000_0000, 512 * 1024 * 1024);

    let mut next_slot = 100;
    let mut allocator = || {
        let slot = next_slot;
        next_slot += 1;
        Ok(slot)
    };

    // 2. Spawn serial driver
    let serial_config = ComponentConfig {
        name: "serial_driver",
        entry_point: 0x400000,
        stack_size: 64 * 1024,
        priority: 150,
        device: Some(DeviceId::Serial { port: 0 }),
        fault_ep: None,
    };

    let serial_driver = spawner
        .spawn_component_with_device(serial_config, &mut allocator, 10, &mut broker)
        .expect("Failed to spawn serial driver");

    // 3. Spawn network driver
    let network_config = ComponentConfig {
        name: "network_driver",
        entry_point: 0x500000,
        stack_size: 128 * 1024,
        priority: 140,
        device: Some(DeviceId::Pci {
            vendor: 0x8086,
            device: 0x100E,
        }),
        fault_ep: None,
    };

    let network_driver = spawner
        .spawn_component_with_device(network_config, &mut allocator, 11, &mut broker)
        .expect("Failed to spawn network driver");

    // 4. Spawn filesystem (no device)
    let fs_config = ComponentConfig {
        name: "filesystem",
        entry_point: 0x600000,
        stack_size: 256 * 1024,
        priority: 100,
        device: None,
        fault_ep: None,
    };

    let filesystem = spawner
        .spawn_component(fs_config, &mut allocator, 12)
        .expect("Failed to spawn filesystem");

    // 5. Verify components
    assert_eq!(spawner.component_count(), 3);

    // Serial driver has device
    assert!(serial_driver.device_bundle().is_some());
    assert_eq!(serial_driver.device_bundle().unwrap().irq.irq_num(), 4);

    // Network driver has device
    assert!(network_driver.device_bundle().is_some());
    assert_eq!(network_driver.device_bundle().unwrap().irq.irq_num(), 11);

    // Filesystem has no device
    assert!(filesystem.device_bundle().is_none());

    // 6. Start all drivers
    spawner.start_component(&serial_driver).unwrap();
    spawner.start_component(&network_driver).unwrap();
    spawner.start_component(&filesystem).unwrap();

    assert_eq!(spawner.running_component_count(), 3);

    // 7. IPC endpoints available for communication
    let serial_ep = serial_driver.endpoint();
    let network_ep = network_driver.endpoint();
    let fs_ep = filesystem.endpoint();

    // All components have unique endpoints
    assert_ne!(serial_ep, network_ep);
    assert_ne!(network_ep, fs_ep);
    assert_ne!(serial_ep, fs_ep);

    // System is fully operational with:
    // - Isolated components
    // - Device access
    // - IPC endpoints
    // - Running drivers
}
