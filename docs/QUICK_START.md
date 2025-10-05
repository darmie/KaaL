# KaaL Quick Start Guide

Get up and running with KaaL in under 5 minutes!

## Prerequisites

- Rust 1.70+ (`rustup update`)
- Cargo
- Git

## Installation

```bash
# Clone the repository
git clone https://github.com/your-org/kaal.git
cd kaal

# Build the project
cargo build --workspace

# Run tests to verify
cargo test --workspace
```

## Run Your First Example

### System Composition Demo

See a complete multi-component system in action:

```bash
cargo run --bin system-composition
```

**What it does:**
- Parses seL4 bootinfo
- Initializes Capability Broker
- Spawns 3 components (serial driver, network driver, filesystem)
- Allocates device resources (MMIO, IRQ, DMA)
- Starts all components
- Shows system status

**Expected output:**
```
╔═══════════════════════════════════════════════╗
║   KaaL System Composition Demonstration      ║
║   Phase 2: Complete Integration              ║
╚═══════════════════════════════════════════════╝

🚀 STEP 1: System Initialization
  ✓ Parsed bootinfo from seL4 kernel
  ✓ Initialized Capability Broker

...

╔═══════════════════════════════════════════════╗
║              ✅ SYSTEM READY                  ║
╚═══════════════════════════════════════════════╝
```

### Serial Driver Example

```bash
cargo run --bin serial-driver-example
```

Shows DDDK (Device Driver Development Kit) integration.

### Root Task Example

```bash
cargo run --bin root-task-example
```

Demonstrates VSpace and CNode management.

## Project Structure

```
kaal/
├── runtime/
│   ├── cap_broker/      # Capability management & device allocation
│   ├── ipc/             # Inter-process communication
│   ├── dddk/            # Device Driver Development Kit (macros)
│   ├── dddk-runtime/    # DDDK runtime support
│   └── root-task/       # Root task initialization
├── components/
│   ├── vfs/             # Virtual File System
│   ├── posix/           # POSIX compatibility layer
│   └── network/         # Network stack
├── examples/
│   ├── system-composition/  # Complete system demo ⭐
│   ├── serial-driver/       # Serial driver example
│   └── root-task-example/   # Root task demo
├── docs/
│   ├── SYSTEM_COMPOSITION.md  # System composition guide
│   ├── SEL4_INTEGRATION_ROADMAP.md  # Real seL4 integration
│   └── IMPLEMENTATION_PLAN.md  # Development roadmap
└── tools/
    └── sel4-compose/    # CLI tool (coming soon)
```

## Key Concepts

### 1. Capability Broker

Central service for device and resource management:

```rust
use cap_broker::DefaultCapBroker;

let mut broker = unsafe {
    DefaultCapBroker::init().expect("Failed to init broker")
};

// Request device resources
let device_bundle = broker.request_device(DeviceId::Serial { port: 0 })?;
```

### 2. Component Spawner

Create isolated execution contexts:

```rust
use cap_broker::{ComponentSpawner, ComponentConfig};

let mut spawner = ComponentSpawner::new(
    cspace_root, vspace_root,
    0x4000_0000, 512 * 1024 * 1024
);

let component = spawner.spawn_component_with_device(
    ComponentConfig {
        name: "my_driver",
        entry_point: 0x400000,
        stack_size: 64 * 1024,
        priority: 200,
        device: Some(DeviceId::Serial { port: 0 }),
        fault_ep: None,
    },
    allocator,
    untyped_cap,
    &mut broker,
)?;

spawner.start_component(&component)?;
```

### 3. Device Drivers (DDDK)

Write drivers in ~50 lines:

```rust
use dddk::Driver;

#[derive(Driver)]
#[pci(vendor = 0x8086, device = 0x100E)]
#[resources(mmio = "bar0", irq = "auto", dma = "4MB")]
pub struct E1000Driver {
    #[mmio]
    regs: &'static mut E1000Registers,

    #[dma_ring(size = 256)]
    rx_ring: DmaRing<RxDescriptor>,
}

#[driver_impl]
impl E1000Driver {
    #[init]
    fn initialize(&mut self) -> Result<()> {
        // Driver initialization
        Ok(())
    }

    #[interrupt]
    fn handle_interrupt(&mut self) {
        // IRQ handler
    }
}
```

## Development Workflow

### 1. Write a New Component

```rust
// my_component/src/lib.rs
pub fn component_main() {
    loop {
        // Your component logic
    }
}
```

### 2. Add to System Composition

```rust
let config = ComponentConfig {
    name: "my_component",
    entry_point: my_component::component_main as usize,
    stack_size: 128 * 1024,
    priority: 100,
    device: None,
    fault_ep: None,
};

let component = spawner.spawn_component(config, allocator, untyped_cap)?;
spawner.start_component(&component)?;
```

### 3. Test

```bash
# Unit tests
cargo test --package my_component

# Integration tests
cargo test --test integration_test

# Run the system
cargo run --bin system-composition
```

## Current Status (Phase 2 Complete ✅)

- ✅ **86 tests passing** (77 unit + 9 integration)
- ✅ Bootinfo parsing from seL4
- ✅ Capability Broker with device allocation
- ✅ Component spawning (isolated execution)
- ✅ VSpace management (virtual memory)
- ✅ TCB management (threads) - **x86_64 + aarch64**
- ✅ MMIO mapping (device registers)
- ✅ IRQ handling (interrupts)
- ✅ DMA pool allocation
- ✅ IPC endpoints & notifications
- ✅ **Mac Silicon (aarch64) support!**

## Next Steps

### Option 1: Continue with Mocks (Recommended)

Stay in fast iteration mode and build:
- IPC message passing (`seL4_Call/Reply`)
- Driver implementations (serial, network, timer)
- System services (VFS, network stack)

### Option 2: Real seL4 Integration

Switch to real seL4 kernel (~4 hours):

See [`docs/SEL4_INTEGRATION_ROADMAP.md`](SEL4_INTEGRATION_ROADMAP.md) for step-by-step guide.

Quick commands:
```bash
# Get seL4 kernel
git clone https://github.com/seL4/seL4.git
cd seL4 && mkdir build && cd build
cmake .. -DPLATFORM=x86_64 -DCMAKE_BUILD_TYPE=Release
ninja

# Update Cargo.toml
sed -i 's|path = "runtime/sel4-mock"|git = "https://github.com/seL4/rust-sel4"|' Cargo.toml

# Build with real seL4
cargo build --features sel4-real
```

## Common Tasks

### Add a New Driver

1. Create driver in `components/drivers/`
2. Use `#[derive(Driver)]` macro
3. Implement `#[init]` and `#[interrupt]` handlers
4. Add to system composition

### Add a System Service

1. Create service in `components/`
2. Define IPC interface
3. Spawn as component (no device needed)
4. Connect to other components via IPC

### Debug a Component

```rust
// Use log macros (already configured)
log::info!("Component starting");
log::debug!("Processing data: {:?}", data);
log::error!("Failed to allocate: {:?}", err);
```

### Run Performance Tests

```bash
cargo bench
```

## Documentation

- **[System Composition Guide](SYSTEM_COMPOSITION.md)** - Complete workflow
- **[Implementation Plan](IMPLEMENTATION_PLAN.md)** - Development roadmap
- **[seL4 Integration](SEL4_INTEGRATION_ROADMAP.md)** - Real kernel setup
- **[Technical Architecture](../internal_resource/technical_arch_implementation.md)** - Deep dive

## Examples

- `examples/system-composition/` - **Complete multi-component system ⭐**
- `examples/serial-driver/` - DDDK driver example
- `examples/root-task-example/` - VSpace/CNode management

## Getting Help

- 📖 Read the [documentation](docs/)
- 🐛 Open an [issue](https://github.com/your-org/kaal/issues)
- 💬 Join [discussions](https://github.com/your-org/kaal/discussions)
- 📧 Email: team@kaal.dev

## Contributing

We welcome contributions! See [`CONTRIBUTING.md`](../CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
git clone https://github.com/your-org/kaal.git
cd kaal
cargo build --workspace
cargo test --workspace
cargo clippy -- -D warnings
cargo fmt
```

## License

Dual-licensed under MIT or Apache-2.0. See [`LICENSE-MIT`](../LICENSE-MIT) and [`LICENSE-APACHE`](../LICENSE-APACHE).

---

**Built with ❤️ by the KaaL Team**

*Making verified OS development accessible to everyone*
