# KaaL Architecture Overview

A high-level overview of the KaaL microkernel and framework architecture.

---

## Design Philosophy

### Core Principles

1. **Capability-Based Security** - Unforgeable tokens for all access control
2. **Composition Over Configuration** - Link only what you need
3. **Type Safety** - Rust's safety guarantees throughout the stack
4. **Performance Through Architecture** - Zero-copy, efficient IPC, minimal overhead

### Design Goals

| Goal | Target | Status |
|------|--------|--------|
| **Security** | Capability-based, memory-safe | ✅ By Design |
| **Performance** | Low latency IPC, fast context switching | 🚧 In Progress |
| **Portability** | Easy to port to new ARM64 boards | ✅ Config-driven |
| **Developer Experience** | Modern tooling, clear abstractions | 🚧 In Progress |

---

## System Layers

### Layer 0: KaaL Microkernel (Rust)
- **Language:** Pure Rust (no_std)
- **Architecture:** ARM64 (AArch64)
- **Responsibilities:**
  - Capability management
  - IPC and message passing
  - Memory management (MMU, page tables)
  - Thread scheduling
  - Exception handling
- **Status:** Boot and early initialization complete

### Layer 1: Runtime Services
- **Size:** ~8K LOC
- **Components:**
  - Capability Broker (5K LOC)
  - Memory Manager (3K LOC)
- **Responsibilities:**
  - Hide microkernel complexity
  - Device resource allocation
  - Untyped memory management
  - IPC endpoint creation
- **Development:** Stable foundation, rarely changes

### Layer 2: Driver & Device Layer
- **Size:** Variable (~5-50K per driver)
- **Components:**
  - DDDK (Device Driver Development Kit)
  - DDE-Linux (compatibility layer)
  - Native drivers
- **Responsibilities:**
  - Hardware abstraction
  - Interrupt handling
  - DMA management
  - Driver registration
- **Development:** Add drivers as needed

### Layer 3: System Services
- **Size:** ~75K LOC total
- **Components:**
  - VFS (10K LOC)
  - Network Stack (30K LOC)
  - Display Manager (20K LOC)
  - Audio Subsystem (15K LOC)
- **Responsibilities:**
  - Core OS functionality
  - Component communication
  - Resource management
- **Development:** Composable, replaceable

### Layer 4: Compatibility Shims
- **Size:** ~20K LOC
- **Components:**
  - LibC implementation
  - POSIX server
  - Standard library facades
- **Responsibilities:**
  - Syscall translation
  - Process management
  - Signal delivery
  - File descriptor mapping
- **Development:** Expand coverage over time

### Layer 5: Applications
- **Size:** Unlimited
- **Examples:**
  - POSIX programs (bash, coreutils)
  - Native Rust/C applications
  - Python, Node.js (with patches)
- **Responsibilities:**
  - User functionality
- **Development:** Standard toolchains

---

## Key Abstractions

### 1. Capability Broker

**Purpose:** Hide seL4 capability complexity

**API:**
```rust
pub trait CapabilityBroker {
    fn request_device(&mut self, device: DeviceId) -> Result<DeviceBundle>;
    fn allocate_memory(&mut self, size: usize) -> Result<MemoryRegion>;
    fn request_irq(&mut self, irq: u8) -> Result<IrqHandler>;
    fn create_channel(&mut self) -> Result<(Endpoint, Endpoint)>;
}
```

**Design:**
- Single point for all capability operations
- Abstracts untyped memory retype
- Manages capability space
- Device enumeration (ACPI/device tree)

### 2. Shared Memory IPC

**Purpose:** High-performance bulk data transfer

**Architecture:**
```
Producer                    Consumer
   │                           │
   ├─ Write to ring buffer    │
   ├─ Signal (1 IPC) ─────────►│
   │                           ├─ Read from ring
   │◄────── Ack (1 IPC) ───────┤
```

**Performance:**
- 2 IPC per transaction (vs 10+ for message passing)
- Zero-copy for large transfers
- Lock-free ring buffer

### 3. DDDK (Device Driver Development Kit)

**Purpose:** Reduce driver development from 500+ LOC to ~50 LOC

**Approach:**
```rust
#[derive(Driver)]
#[resources(mmio = "bar0", irq = "auto", dma = "4MB")]
pub struct MyDriver {
    #[mmio] regs: &'static mut Registers,
    #[dma_ring(size = 256)] rx_ring: DmaRing<RxDesc>,
}

#[driver_impl]
impl MyDriver {
    #[init]
    fn initialize(&mut self) -> Result<()> { /* ... */ }

    #[interrupt]
    fn handle_interrupt(&mut self) { /* ... */ }
}
```

**Benefits:**
- Declarative resource specification
- Auto-generated boilerplate
- Type-safe MMIO access
- DMA pool management

### 4. DDE-Linux

**Purpose:** Reuse existing Linux drivers without modification

**Architecture:**
```
Linux Driver (unmodified)
    ↓
DDE Compatibility Layer
    ↓
seL4 Capabilities
```

**Emulated APIs:**
- `kmalloc`, `kfree` → Capability Broker
- `ioremap` → MMIO mapping
- `request_irq` → IRQ handler
- `dma_alloc_coherent` → DMA pool

---

## Component Communication

### IPC Patterns

**1. Request-Response (Control Path)**
```
Client                  Server
   │                      │
   ├── Request ──────────►│
   │                      ├─ Process
   │◄───── Response ──────┤
```
- Used for: Syscalls, control operations
- Latency: <2μs

**2. Shared Memory (Data Path)**
```
Producer                Consumer
   │                      │
   ├─ Write to shared buf │
   ├─ Signal ────────────►│
   │                      ├─ Read from buf
```
- Used for: File I/O, network packets
- Throughput: >1 GB/s

**3. Notification (Async Events)**
```
Component A             Component B
   │                      │
   ├─ Notify ────────────►│
   │                      └─ Handle event
```
- Used for: Interrupts, signals
- Latency: <1μs

---

## Memory Layout

### Virtual Address Space (per component)

```
0xFFFF_FFFF_FFFF_FFFF
├─ Kernel Objects (unmapped)
│
├─ 0xFFFF_8000_0000_0000
│  └─ Shared Memory Regions
│     ├─ IPC Ring Buffers (4KB each)
│     ├─ DMA Buffers (driver-specific)
│     └─ Bulk Transfer (4MB default)
│
├─ 0x0000_8000_0000_0000
│  └─ Component Private Memory
│     ├─ Heap (grows up)
│     ├─ Stack (grows down)
│     ├─ .text, .data, .bss
│     └─ Thread Local Storage
│
└─ 0x0000_0000_0000_0000
   └─ NULL guard page
```

### Physical Memory Management

- **Untyped Memory:** Managed by Capability Broker
- **Device Memory:** MMIO mapped on-demand
- **DMA Memory:** Identity-mapped pools

---

## Boot Sequence

```
1. seL4 Kernel Boot
   └─ Initialize capabilities
   └─ Create root task

2. Root Task Initialization
   └─ Parse bootinfo
   └─ Start Capability Broker

3. Capability Broker Init
   └─ Enumerate devices (ACPI/DT)
   └─ Set up capability space
   └─ Create untyped allocator

4. Component Initialization
   └─ Start drivers (serial, block, net)
   └─ Start system services (VFS, network)
   └─ Start POSIX server

5. Application Launch
   └─ Load init process
   └─ Run user applications
```

**Target Boot Time:** <5 seconds

---

## Security Model

### Capability-Based Access Control

- **Principle:** No ambient authority
- **Enforcement:** seL4 kernel (formally verified)
- **Granularity:** Per-resource capabilities

**Example:**
```rust
// Driver requests device bundle
let bundle = cap_broker.request_device(DeviceId::Pci {
    vendor: 0x8086,
    device: 0x100E
})?;

// Bundle contains ONLY what driver needs:
// - MMIO regions for this device
// - IRQ for this device
// - DMA pool for this device
// Nothing else accessible
```

### Component Isolation

- Each component in separate protection domain
- Communication only via IPC or shared memory
- No shared global state
- Crash isolation (component failure doesn't crash system)

### TCB Composition

```
Total TCB: ~125K LOC
├─ seL4 Kernel: 10K LOC (verified)
├─ Runtime Services: 8K LOC
├─ System Services: 75K LOC
├─ Compatibility Shims: 20K LOC
└─ Drivers: Variable (12K typical)
```

Compare to Linux: ~15M LOC

---

## Performance Characteristics

### Latency Targets

| Operation | Target | Overhead vs Linux |
|-----------|--------|-------------------|
| Context Switch | <1μs | 3x |
| IPC (shared mem) | <1μs | N/A |
| File Read (cached) | <5μs | 2.5x |
| Network Send | <10μs | 2x |
| System Call | <2μs | 4x |

### Throughput Targets

| Operation | Target |
|-----------|--------|
| Disk I/O (sequential) | >500 MB/s |
| Network (1 Gbps NIC) | >100 MB/s |
| Memory Copy | >10 GB/s |

### Optimizations

1. **Shared Memory:** Avoid IPC overhead for bulk transfers
2. **Batching:** Combine multiple operations
3. **Zero-Copy:** Direct buffer mapping
4. **Cache Alignment:** Avoid false sharing
5. **Lock-Free:** Atomic operations where possible

---

## Scalability

### Multi-Core Support (Phase 4)

- SMP scheduling
- Per-core data structures
- Cross-core IPC optimization
- NUMA awareness

### Resource Limits

| Resource | Minimum | Recommended | Maximum |
|----------|---------|-------------|---------|
| RAM | 128 MB | 2 GB | 16+ GB |
| CPU Cores | 1 | 4 | 64 |
| Storage | 256 MB | 8 GB | Unlimited |
| Network | None | 1 Gbps | 10+ Gbps |

---

## Extension Points

### Adding New Components

1. Implement required traits
2. Declare dependencies in `system.toml`
3. Register IPC endpoints
4. Write integration tests

### Custom Drivers

1. Use DDDK macros
2. Implement device-specific logic
3. Register in driver database
4. Test with hardware simulator

### Alternative Implementations

All components can be replaced:
- VFS → Custom file system
- Network → Alternative TCP/IP stack
- Scheduler → Custom scheduling algorithm
- Allocator → Different memory allocator

---

## Development Workflow

```
1. Design Component
   └─ Define traits and APIs
   └─ Identify integration points

2. Implement Core
   └─ Write production code (no placeholders)
   └─ Handle all error cases
   └─ Document safety invariants

3. Test Thoroughly
   └─ Unit tests (>80% coverage)
   └─ Integration tests
   └─ Hardware simulation tests

4. Integrate
   └─ Connect to other components
   └─ Verify IPC endpoints
   └─ Test end-to-end

5. Optimize
   └─ Profile hot paths
   └─ Benchmark performance
   └─ Document results
```

---

## Next Steps

1. **Read** [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for development roadmap
2. **Study** [technical_arch_implementation.md](../internal_resource/technical_arch_implementation.md) for details
3. **Review** [.CLAUDE](../.CLAUDE) for coding standards
4. **Start** with Phase 1 implementation

---

**Questions?** Open an issue or discussion on GitHub.

**Want to contribute?** See [CONTRIBUTING.md](../CONTRIBUTING.md).
