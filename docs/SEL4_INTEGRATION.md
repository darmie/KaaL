# seL4 Integration Guide

This guide explains how KaaL integrates with the seL4 microkernel and how to set up a development environment for seL4-based development.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     User Applications                        │
├─────────────────────────────────────────────────────────────┤
│                  POSIX Compatibility Layer                   │
├─────────────────────────────────────────────────────────────┤
│          System Services (VFS, Network, etc.)                │
├─────────────────────────────────────────────────────────────┤
│                    Device Drivers (DDDK)                     │
├─────────────────────────────────────────────────────────────┤
│                    Capability Broker                         │
│              (Manages seL4 Capabilities)                     │
├─────────────────────────────────────────────────────────────┤
│                    seL4 Rust Bindings                        │
│                  (sel4, sel4-sys crates)                     │
├─────────────────────────────────────────────────────────────┤
│                    seL4 Microkernel                          │
│                     (10K LOC C)                              │
└─────────────────────────────────────────────────────────────┘
```

## seL4 Kernel Capabilities

KaaL leverages seL4's capability-based security model:

### Capability Types Used

1. **Untyped Memory** - Raw memory that can be retyped
   - Used by Capability Broker to create other capabilities
   - Provided in bootinfo at system startup

2. **Frame Capabilities** - Physical memory pages
   - Created from Untyped memory via `seL4_Untyped_Retype`
   - Mapped into VSpace for MMIO access

3. **IRQ Handler Capabilities** - Interrupt handlers
   - Obtained via `seL4_IRQControl_Get`
   - Bound to notification objects for signaling

4. **Notification Capabilities** - Async signaling
   - Used by IPC layer for producer/consumer notifications
   - Bound to IRQ handlers for interrupt delivery

5. **Endpoint Capabilities** - Synchronous IPC
   - Used for RPC-style communication
   - Created from Untyped memory

## Capability Broker Integration

The Capability Broker is KaaL's interface to seL4 capabilities:

### Initialization Flow

```rust
// 1. Root task starts with initial capabilities in bootinfo
let bootinfo = sel4::get_bootinfo();

// 2. Parse bootinfo to get untyped memory and device regions
let untyped_regions = bootinfo.untyped_list();
let device_regions = parse_device_tree();

// 3. Initialize Capability Broker with bootinfo
let mut broker = unsafe {
    DefaultCapBroker::from_bootinfo(bootinfo)?
};

// 4. Drivers request device bundles
let serial_bundle = broker.request_device(DeviceId::Serial { port: 0 })?;

// 5. Bundle contains all necessary resources
// - MMIO regions (mapped frames)
// - IRQ handler (with notification)
// - DMA pool (allocated from untyped)
```

### Resource Allocation Flow

```
Driver Request
      ↓
Capability Broker
      ↓
  ┌───┴────┬──────────┬──────────┐
  ↓        ↓          ↓          ↓
MMIO    IRQ      DMA Pool    I/O Ports
  ↓        ↓          ↓          ↓
Frames   IRQHandler  Frames    (x86 only)
  ↓        ↓          ↓
seL4_Untyped_Retype
  ↓
Untyped Memory (from bootinfo)
```

## Boot Process

### 1. Kernel Boot

```
GRUB/UEFI → seL4 Kernel → Root Task (KaaL)
```

The seL4 kernel:
- Sets up initial address space
- Creates root task with initial capabilities
- Passes bootinfo structure to root task
- Transfers control to root task entry point

### 2. Root Task Initialization

**File:** `src/root_task.rs` (to be created in Phase 2)

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Get bootinfo from seL4
    let bootinfo = unsafe { sel4::get_bootinfo() };

    // Initialize capability broker
    let mut broker = unsafe {
        DefaultCapBroker::from_bootinfo(bootinfo)
            .expect("Failed to initialize capability broker")
    };

    // Initialize IPC system
    let ipc_manager = IpcManager::new(&mut broker)
        .expect("Failed to initialize IPC");

    // Spawn driver components
    spawn_serial_driver(&mut broker, &ipc_manager);
    spawn_network_driver(&mut broker, &ipc_manager);

    // Spawn system services
    spawn_vfs_service(&ipc_manager);
    spawn_network_stack(&ipc_manager);

    // Spawn POSIX server
    spawn_posix_server(&ipc_manager);

    // Main event loop
    event_loop();
}
```

### 3. Component Spawning

Each component runs in its own seL4 thread with:
- Private VSpace (address space)
- CSpace (capability space)
- TCB (Thread Control Block)
- Stack and IPC buffer

```rust
fn spawn_component(
    broker: &mut dyn CapabilityBroker,
    name: &str,
    entry_point: fn() -> !,
) -> Result<ComponentHandle> {
    // Allocate capabilities
    let tcb_cap = broker.allocate_tcb()?;
    let vspace_cap = broker.allocate_vspace()?;
    let cspace_cap = broker.allocate_cspace()?;

    // Configure TCB
    unsafe {
        sel4_sys::seL4_TCB_Configure(
            tcb_cap,
            cspace_cap,
            vspace_cap,
            0, // fault_ep
            IPC_BUFFER_VADDR,
            ipc_buffer_cap,
        )?;

        // Set registers (entry point, stack pointer)
        sel4_sys::seL4_TCB_WriteRegisters(
            tcb_cap,
            false, // resume
            0,     // arch_flags
            2,     // count
            &mut [
                entry_point as usize, // rip
                STACK_TOP,            // rsp
            ],
        )?;

        // Resume thread
        sel4_sys::seL4_TCB_Resume(tcb_cap)?;
    }

    Ok(ComponentHandle { tcb_cap, name })
}
```

## Memory Management

### Virtual Memory Layout

```
0xFFFF_FFFF_FFFF_FFFF ┌────────────────────┐
                      │   Kernel Space     │
                      │    (seL4 kernel)   │
0xFFFF_8000_0000_0000 ├────────────────────┤
                      │   Component VSpace │
                      │                    │
                      │  ┌──────────────┐  │
                      │  │ Stack        │  │
0x0000_7FFF_FFFF_F000 │  ├──────────────┤  │
                      │  │ Heap         │  │
0x0000_0000_4000_0000 │  ├──────────────┤  │
                      │  │ MMIO Devices │  │
0x0000_0000_2000_0000 │  ├──────────────┤  │
                      │  │ IPC Buffers  │  │
0x0000_0000_1000_0000 │  ├──────────────┤  │
                      │  │ Code + Data  │  │
0x0000_0000_0040_0000 │  └──────────────┘  │
                      │                    │
0x0000_0000_0000_0000 └────────────────────┘
```

### DMA Memory

DMA regions require:
1. **Physical contiguity** - Allocated from large untyped regions
2. **Identity mapping** - Vaddr == Paddr for device access
3. **Cache attributes** - Uncached or write-combining

```rust
impl DmaPool {
    pub fn allocate_dma(&mut self, size: usize) -> Result<DmaRegion> {
        // Find contiguous untyped region
        let untyped = self.find_contiguous_untyped(size)?;

        // Retype to frames
        let num_frames = (size + 4095) / 4096;
        let frames = self.retype_to_frames(untyped, num_frames)?;

        // Map with identity mapping (vaddr == paddr)
        let paddr = untyped.base_paddr;
        for (i, frame) in frames.iter().enumerate() {
            unsafe {
                sel4_sys::seL4_ARCH_Page_Map(
                    *frame,
                    self.vspace_root,
                    paddr + i * 4096, // vaddr == paddr
                    sel4_sys::seL4_CanRead | sel4_sys::seL4_CanWrite,
                    sel4_sys::seL4_ARCH_Uncached, // Important for DMA!
                )?;
            }
        }

        Ok(DmaRegion {
            vaddr: paddr,
            paddr,
            size,
        })
    }
}
```

## Interrupt Handling

### IRQ Registration Flow

```rust
// 1. Driver requests IRQ
let irq_handler = broker.request_irq(4)?; // COM1 IRQ

// 2. Broker creates notification and binds to IRQ
unsafe {
    // Get IRQ handler capability
    sel4_sys::seL4_IRQControl_Get(
        sel4_sys::seL4_CapIRQControl,
        4, // IRQ number
        cspace_root,
        irq_cap,
        depth,
    )?;

    // Create notification
    let notif_cap = create_notification()?;

    // Bind IRQ to notification
    sel4_sys::seL4_IRQHandler_SetNotification(
        irq_cap,
        notif_cap,
    )?;
}

// 3. Driver waits for interrupts
loop {
    unsafe {
        sel4_sys::seL4_Wait(notif_cap, &mut badge);
    }

    // Handle interrupt
    handle_serial_interrupt();

    // Acknowledge IRQ
    unsafe {
        sel4_sys::seL4_IRQHandler_Ack(irq_cap)?;
    }
}
```

### Interrupt Thread Model

Each device driver runs in its own thread and blocks on its IRQ notification:

```
┌──────────┐        ┌──────────┐        ┌──────────┐
│ Serial   │        │ Network  │        │  Timer   │
│ Driver   │        │ Driver   │        │ Driver   │
│          │        │          │        │          │
│ Wait on  │        │ Wait on  │        │ Wait on  │
│ IRQ 4    │        │ IRQ 11   │        │ IRQ 0    │
└────┬─────┘        └────┬─────┘        └────┬─────┘
     │                   │                   │
     │ seL4_Wait()       │ seL4_Wait()       │ seL4_Wait()
     │                   │                   │
     ↓                   ↓                   ↓
┌────────────────────────────────────────────────┐
│            seL4 Notification Objects            │
└────────────────────────────────────────────────┘
     ↑                   ↑                   ↑
     │                   │                   │
   IRQ 4               IRQ 11              IRQ 0
     │                   │                   │
┌────────────────────────────────────────────────┐
│               Hardware Interrupts               │
└────────────────────────────────────────────────┘
```

## IPC Mechanisms

KaaL uses two IPC mechanisms:

### 1. Shared Memory IPC (High Throughput)

Used for bulk data transfer (network packets, file I/O):

```rust
// Create shared ring buffer in mapped memory
let shared_mem = broker.allocate_memory(4096)?;
let ring: &mut SharedRing<Packet, 256> = unsafe {
    &mut *(shared_mem.vaddr as *mut SharedRing<Packet, 256>)
};

// Create notification pair for signaling
let (producer_notify, consumer_notify) = broker.create_notification_pair()?;

// Initialize ring with notifications
*ring = SharedRing::with_notifications(consumer_notify, producer_notify);

// Producer sends data
ring.push(packet)?; // Automatically signals consumer

// Consumer receives data
let packet = ring.pop()?; // Can block waiting for signal
```

### 2. seL4 Endpoints (RPC)

Used for synchronous request/response:

```rust
// Create endpoint
let endpoint = broker.create_endpoint()?;

// Server blocks waiting for calls
loop {
    let (sender, msg) = endpoint.recv()?;

    // Process request
    let response = handle_request(msg);

    // Reply to sender
    endpoint.reply(sender, response)?;
}

// Client makes synchronous call
let response = endpoint.call(server_ep, request)?;
```

## Platform Support

### x86_64 PC99

**Platforms:** QEMU, PC hardware, VMs

**Features:**
- IOMMU support (VT-d)
- APIC/x2APIC interrupts
- PCI device enumeration
- VGA/Serial console

**Boot:** Multiboot via GRUB

### ARM64 (AArch64)

**Platforms:** Raspberry Pi 4, QEMU virt

**Features:**
- GICv2/GICv3 interrupts
- Device tree parsing
- UART PL011
- SMMU support

**Boot:** U-Boot or UEFI

## Development Workflow

### 1. Build seL4 Kernel

```bash
# Clone seL4
git clone https://github.com/seL4/seL4.git

# Configure for x86_64
cd seL4
mkdir build && cd build
cmake -DPLATFORM=x86_64 -DSIMULATION=TRUE ..
make

# Kernel binary: images/kernel-x86_64-pc99
```

### 2. Build KaaL with seL4

```bash
# Set environment
export SEL4_DIR=/path/to/seL4
export SEL4_PLATFORM=x86_64

# Build KaaL
cmake -B build
cmake --build build

# System image: build/images/kaal-image-x86_64-pc99
```

### 3. Run in QEMU

```bash
qemu-system-x86_64 \
    -kernel build/images/kernel-x86_64-pc99 \
    -initrd build/images/kaal-image-x86_64-pc99 \
    -serial stdio \
    -nographic \
    -m 512M
```

### 4. Debug with GDB

```bash
# Terminal 1: Run QEMU with GDB server
qemu-system-x86_64 \
    -kernel build/images/kernel-x86_64-pc99 \
    -initrd build/images/kaal-image-x86_64-pc99 \
    -serial stdio \
    -s -S # GDB server on :1234, wait for debugger

# Terminal 2: Connect GDB
gdb build/kaal-root
(gdb) target remote :1234
(gdb) break _start
(gdb) continue
```

## Performance Characteristics

### seL4 Kernel Operations

| Operation | Cycles (x86_64) | Time @ 3GHz |
|-----------|-----------------|-------------|
| IPC (call) | ~150 | ~50 ns |
| Notification Signal | ~50 | ~16 ns |
| Notification Wait | ~50 | ~16 ns |
| Context Switch | ~150 | ~50 ns |
| Page Fault | ~800 | ~266 ns |

### KaaL Overhead

| Operation | Additional Cycles | Total Time |
|-----------|-------------------|------------|
| Ring Buffer Push | ~20 | ~70 ns |
| Ring Buffer Pop | ~20 | ~70 ns |
| Device Request (first) | ~2000 | ~700 ns |
| Device Request (cached) | ~100 | ~33 ns |

## Security Guarantees

seL4's formal verification provides:

1. **Functional Correctness** - Kernel behaves according to specification
2. **Integrity** - No unauthorized access to memory or capabilities
3. **Confidentiality** - Information flow only through IPC
4. **Availability** - No kernel-level deadlocks

KaaL preserves these guarantees by:
- All device access mediated through Capability Broker
- No direct hardware access by drivers
- IPC-only communication between components
- Capability-based access control

## Resources

- **seL4 Manual:** https://sel4.systems/Info/Docs/seL4-manual.pdf
- **Rust Bindings:** https://github.com/seL4/rust-sel4
- **Tutorials:** https://docs.sel4.systems/Tutorials/
- **Proofs:** https://sel4.systems/Info/FAQ/proof.pml

---

**Next:** See `PHASE2_MIGRATION.md` for migration steps
