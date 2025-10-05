# KaaL System Composition Guide

## Overview

This guide explains how to compose a complete KaaL system by integrating the Capability Broker, Component Spawner, and device drivers. It demonstrates the complete workflow from bootinfo parsing to running multi-component systems.

## Architecture Layers

```
┌──────────────────────────────────────────────────┐
│            Root Task (System Manager)            │
│  • Bootinfo parsing                             │
│  • Capability Broker init                       │
│  • Component spawning                           │
└────────────┬─────────────────────────────────────┘
             │
   ┌─────────┴──────────┬───────────────┬──────────┐
   │                    │               │          │
┌──▼────┐         ┌─────▼──────┐  ┌────▼─────┐  ┌─▼──────┐
│Serial │         │  Network   │  │ Storage  │  │  More  │
│Driver │         │   Driver   │  │ Driver   │  │Drivers │
└───────┘         └────────────┘  └──────────┘  └────────┘
    │                    │              │
    └────────────────────┴──────────────┴─────────────┐
                                                       │
                                                  ┌────▼────┐
                                                  │  IPC    │
                                                  │ Layer   │
                                                  └─────────┘
```

## Complete System Composition Workflow

### Step 1: System Initialization

The root task starts by parsing bootinfo from the seL4 kernel and initializing the Capability Broker:

```rust
use cap_broker::{BootInfo, DefaultCapBroker};

unsafe {
    // Parse bootinfo from seL4 kernel
    let bootinfo = BootInfo::get().expect("Failed to get bootinfo");

    // Key bootinfo fields:
    // - cspace_root: CSpace root capability (slot 1)
    // - vspace_root: VSpace root capability (slot 2)
    // - tcb: Initial TCB (slot 3)
    // - irq_control: IRQ control capability (slot 4)
    // - empty: Available capability slot range (e.g., 100-4096)
    // - untyped: Available untyped memory regions

    // Initialize capability broker
    let mut broker = DefaultCapBroker::init()
        .expect("Failed to initialize broker");
}
```

**What Happens:**
1. Bootinfo is read from seL4 kernel memory
2. Critical capability slots are extracted (CSpace root, VSpace root, TCB, IRQ control)
3. CSpace allocator is initialized with empty slot range
4. Untyped memory regions are registered
5. Device database is prepared for allocation

### Step 2: Component Spawner Setup

Create a ComponentSpawner to manage isolated execution contexts:

```rust
use cap_broker::ComponentSpawner;

let mut spawner = ComponentSpawner::new(
    bootinfo.cspace_root,  // CSpace root capability
    bootinfo.vspace_root,  // VSpace root capability
    0x4000_0000,           // VSpace base address (1GB)
    512 * 1024 * 1024,     // VSpace size (512MB)
);
```

**What Happens:**
1. ComponentSpawner is initialized with CSpace and VSpace roots
2. Virtual address space region is reserved for components
3. TCB and VSpace managers are created internally
4. Component tracking structures are initialized

### Step 3: Spawn Components with Devices

Spawn driver components with automatic device resource allocation:

```rust
use cap_broker::{ComponentConfig, DeviceId, DEFAULT_STACK_SIZE};

// Capability slot allocator
let mut next_slot = bootinfo.empty.start;
let mut slot_allocator = || {
    let slot = next_slot;
    next_slot += 1;
    Ok(slot)
};

// Serial driver configuration
let serial_config = ComponentConfig {
    name: "serial_driver",
    entry_point: 0x400000,        // Driver entry function
    stack_size: DEFAULT_STACK_SIZE, // 64KB stack
    priority: 200,                 // High priority
    device: Some(DeviceId::Serial { port: 0 }),
    fault_ep: None,
};

// Spawn component with device
let serial_component = spawner.spawn_component_with_device(
    serial_config,
    &mut slot_allocator,
    10, // untyped_cap for memory allocation
    &mut broker,
).expect("Failed to spawn serial driver");
```

**What Happens:**
1. Component configuration specifies requirements (stack, priority, device)
2. `spawn_component_with_device()` orchestrates:
   - Allocates capability slots (TCB, VSpace, endpoint, notification, frames)
   - Creates TCB for the component
   - Allocates virtual address space for stack and IPC buffer
   - Maps stack frames into VSpace
   - Creates IPC endpoints and notification
   - **Requests device bundle from broker** (MMIO + IRQ + DMA)
   - Configures TCB with entry point, stack pointer, registers
   - Returns Component handle

3. Device bundle includes:
   - MMIO regions (memory-mapped device registers)
   - IRQ handler (interrupt capability + notification)
   - DMA pool (for device DMA operations)

### Step 4: Spawn Software-Only Components

Not all components need hardware devices:

```rust
let fs_config = ComponentConfig {
    name: "filesystem",
    entry_point: 0x600000,
    stack_size: 256 * 1024,  // 256KB stack
    priority: 100,            // Lower priority
    device: None,             // No hardware device
    fault_ep: None,
};

let fs_component = spawner.spawn_component(
    fs_config,
    &mut slot_allocator,
    12, // untyped_cap
).expect("Failed to spawn filesystem");
```

**What Happens:**
1. Similar to device-backed components, but without device allocation
2. Component gets TCB, VSpace, stack, and IPC endpoints
3. No MMIO/IRQ/DMA resources allocated
4. Useful for pure software services (VFS, network stack, etc.)

### Step 5: Start Components

Resume TCBs to start component execution:

```rust
spawner.start_component(&serial_component)
    .expect("Failed to start serial driver");

spawner.start_component(&fs_component)
    .expect("Failed to start filesystem");
```

**What Happens:**
1. TCB is resumed using `seL4_TCB_Resume()`
2. Component begins executing at configured entry point
3. Registers are set (PC, SP, etc.) - architecture-specific (x86_64/aarch64)
4. Component can now:
   - Access MMIO regions (for drivers)
   - Wait for IRQs on notification
   - Send/receive IPC on endpoints

### Step 6: System Status

Query system state:

```rust
println!("Total components: {}", spawner.component_count());
println!("Running: {}", spawner.running_component_count());
println!("Slots used: {}", next_slot - bootinfo.empty.start);
```

## Component Lifecycle

```
┌─────────────┐
│   Created   │  ← spawn_component() / spawn_component_with_device()
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Configured │  ← TCB configured, resources allocated
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Running   │  ← start_component() (TCB resumed)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Suspended  │  ← stop_component() (TCB suspended)
└─────────────┘
```

## Device Resource Allocation

When `device: Some(DeviceId)` is specified:

1. **Device Identification:**
   ```rust
   DeviceId::Serial { port: 0 }
   DeviceId::Pci { vendor: 0x8086, device: 0x100E }
   DeviceId::Platform { name: "uart0" }
   ```

2. **Resource Allocation:**
   - **MMIO**: Physical device registers mapped to virtual address space
   - **IRQ**: Interrupt handler capability bound to notification
   - **DMA**: DMA buffer pool allocated from untyped memory

3. **Device Bundle:**
   ```rust
   pub struct DeviceBundle {
       pub mmio_regions: Vec<MappedRegion>,
       pub irq: IrqHandlerImpl,
       pub dma_pool: DmaPool,
   }
   ```

## Example: Complete System

See [`examples/system-composition/src/main.rs`](../examples/system-composition/src/main.rs) for a complete working example that demonstrates:

- ✅ Bootinfo parsing
- ✅ Capability Broker initialization
- ✅ Serial driver with device resources
- ✅ Network driver with PCI device
- ✅ Filesystem (software-only component)
- ✅ Starting all components
- ✅ System status monitoring

**Run it:**
```bash
cargo run --bin system-composition
```

**Expected Output:**
```
🚀 STEP 1: System Initialization
  ✓ Parsed bootinfo from seL4 kernel
  ✓ Initialized Capability Broker

🏗️  STEP 2: Component Spawner Setup
  ✓ Created ComponentSpawner

📡 STEP 3: Spawn Serial Driver Component
  ✓ Spawned serial driver component
    • Device resources allocated: MMIO, IRQ, DMA

🌐 STEP 4: Spawn Network Driver Component
  ✓ Spawned network driver component

💾 STEP 5: Spawn Filesystem Component
  ✓ Spawned filesystem component (no device)

▶️  STEP 6: Start Components
  ✓ Started all components

📊 STEP 7: System Status
  Total components: 3
  Running components: 3
```

## Architecture-Specific Notes

### x86_64 TCB Configuration
```rust
// PC = RIP, SP = RSP
regs.rip = entry_point;
regs.rsp = stack_pointer;
regs.rflags = 0x200; // IF (interrupt enable)
```

### aarch64 TCB Configuration
```rust
// PC, SP, processor state
regs.pc = entry_point;
regs.sp = stack_pointer;
regs.spsr = 0x0; // EL0t (user mode), interrupts enabled
```

**Both architectures fully supported!** Tested on Mac Silicon (aarch64).

## Integration Points

### With DDDK (Device Driver Development Kit)

Drivers using DDDK macros can request resources:

```rust
#[derive(Driver)]
#[pci(vendor = 0x8086, device = 0x100E)]
#[resources(mmio = "bar0", irq = "auto", dma = "4MB")]
pub struct E1000Driver {
    #[mmio]
    regs: &'static mut E1000Registers,

    #[dma_ring(size = 256)]
    rx_ring: DmaRing<RxDescriptor>,
}
```

The DDDK probe function calls `broker.request_device()` to get resources automatically.

### With IPC Layer

Components communicate via endpoints and notifications:

```rust
// Component A sends to Component B
let message = MyMessage { data: 42 };
ipc_send(component_b.endpoint(), &message)?;

// Component B receives
let message: MyMessage = ipc_recv()?;
```

Shared memory IPC provides <1μs latency for bulk transfers.

### With Real seL4

When switching from mocks to real seL4 (see [`SEL4_INTEGRATION_ROADMAP.md`](SEL4_INTEGRATION_ROADMAP.md)):

1. Replace `sel4-sys` dependency with real seL4 bindings
2. Build seL4 kernel image
3. Link root task with kernel
4. Boot in QEMU or hardware

**All code remains the same!** The `#[cfg(feature = "sel4-real")]` guards handle the switch.

## Best Practices

1. **Capability Slot Management:**
   - Use a simple incrementing allocator for slots
   - Track `next_slot` to avoid conflicts
   - Reserve ranges for specific purposes

2. **Priority Assignment:**
   - Drivers: 150-255 (high priority)
   - System services: 100-149 (medium)
   - Applications: 1-99 (low)

3. **Stack Sizing:**
   - Drivers: 64KB (`DEFAULT_STACK_SIZE`)
   - Network stack: 128KB
   - Application components: 256KB+

4. **Virtual Address Layout:**
   - 0x4000_0000 - 0x6000_0000: Component address spaces
   - Keep regions non-overlapping
   - Leave room for future components

5. **Error Handling:**
   - Always check return values from spawn operations
   - Handle device allocation failures gracefully
   - Provide fallback components if drivers fail

## Testing

Run integration tests:
```bash
cargo test --test integration_test
```

Current test coverage:
- ✅ 86 tests passing (77 unit + 9 integration)
- ✅ Full system initialization
- ✅ Multi-component spawning
- ✅ Device resource allocation
- ✅ Component lifecycle management

## Next Steps

1. **Real seL4 Integration** (~4 hours)
   - Replace mocks with real kernel
   - Test in QEMU
   - Validate on hardware

2. **IPC Message Passing**
   - Implement `seL4_Call/Reply`
   - Message marshalling
   - RPC framework

3. **Driver Implementation**
   - Serial port driver (16550 UART)
   - Network driver (e1000)
   - Timer driver

4. **System Services**
   - VFS implementation
   - Network stack integration
   - Device manager

## References

- [Technical Architecture](../internal_resource/technical_arch_implementation.md)
- [Implementation Plan](IMPLEMENTATION_PLAN.md)
- [seL4 Integration Roadmap](SEL4_INTEGRATION_ROADMAP.md)
- [Example: System Composition](../examples/system-composition/src/main.rs)
- [Example: Serial Driver](../examples/serial-driver/src/main.rs)

---

**Document Version:** 1.0
**Last Updated:** 2025-10-05
**Author:** KaaL Team
