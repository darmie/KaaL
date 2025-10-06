# KaaL Hobbyist Guide: Build Your OS in Hours, Not Years

## Welcome! 🎉

You're about to build your own operating system on top of the formally verified seL4 microkernel. No PhD required!

## What Makes KaaL Special?

- ⚡ **Fast**: Get a working OS in hours, not years
- 🔒 **Secure**: Built on formally verified seL4
- 🎯 **Simple**: Start minimal, add incrementally
- 🔧 **Composable**: Clean callback-based architecture

## Quick Start (5 Minutes)

### 1. Run the Minimal Example

```bash
cd kaal
# Build with Microkit (default - production seL4)
cargo build --bin minimal-component

# For quick testing on macOS, use mock mode
cargo run --features mock-sel4 --bin minimal-component
```

**You'll see:**
```
╔═══════════════════════════════════════════════╗
║        Minimal KaaL System Example          ║
║     Perfect for Hobbyists Getting Started   ║
╚═══════════════════════════════════════════════╝

📋 Step 1: Initialize Root Task
  ✓ Root task initialized

📋 Step 2 & 3: Run System (Composable Pattern)
  • Using run_with() for composability
  • Spawning components via closure

🚀 Spawning Components (via callback)...
  ✓ Spawned: hello_world
  ✓ Started: hello_world (running!)

✅ System Ready!
   • 1 component(s) running
```

**Congratulations!** You just ran a micro-OS with:
- Capability-based security
- Component isolation
- Private address spaces
- IPC ready

### 2. Understand the Code

Open `examples/minimal-component/src/main.rs`:

```rust
// Initialize the root task
let root = unsafe { RootTask::init(RootTaskConfig::default())? };

// Use composable pattern to spawn components
root.run_with(|broker| {
    // Your components here!
    spawn_hello_component(broker);
    spawn_my_driver(broker);
});
```

That's it! The **composable callback pattern** makes it clean and extensible.

## Building Your Own System

### Pattern 1: Minimal Component

```rust
// 1. Define component entry point
pub extern "C" fn my_component() -> ! {
    loop {
        // Your logic here
        core::hint::spin_loop();
    }
}

// 2. Spawn function
fn spawn_my_component(broker: &mut DefaultCapBroker) {
    let bootinfo = unsafe { BootInfo::get().unwrap() };
    let mut spawner = ComponentSpawner::new(...);

    let config = ComponentConfig {
        name: "my_component",
        entry_point: my_component as usize,
        stack_size: DEFAULT_STACK_SIZE,
        priority: 100,
        device: None,  // No hardware needed
        fault_ep: None,
    };

    let component = spawner.spawn_component(config, ...).unwrap();
    spawner.start_component(&component).unwrap();
}

// 3. Use in root task
root.run_with(|broker| {
    spawn_my_component(broker);
});
```

### Pattern 2: Driver with DDDK

```rust
// 1. Define driver with macros (future)
#[derive(Driver)]
#[pci(vendor = 0x8086, device = 0x100E)]
#[resources(mmio = "bar0", irq = "auto", dma = "4MB")]
pub struct MyDriver {
    #[mmio]
    regs: &'static mut Registers,
}

// 2. Auto-generated probe (DDDK does this!)
impl MyDriver {
    #[init]
    fn initialize(&mut self) -> Result<()> {
        // Your init code - that's it!
        Ok(())
    }

    #[interrupt]
    fn handle_irq(&mut self) {
        // Your IRQ handler
    }
}

// 3. Spawn in root task
root.run_with(|broker| {
    let driver = MyDriver::probe(broker).unwrap();
    // Driver is ready!
});
```

## Incremental Development Path

Start simple, add complexity as needed:

### Level 1: Hello World ✅
- **What**: One component, no devices
- **Time**: 5 minutes
- **Example**: `examples/minimal-component`

### Level 2: Multiple Components
- **What**: 2-3 software components communicating via IPC
- **Time**: 30 minutes
- **Add**: IPC ring buffers, message passing

### Level 3: Simple Driver
- **What**: Serial port or timer driver
- **Time**: 2 hours
- **Add**: MMIO access, IRQ handling

### Level 4: Network/Storage
- **What**: E1000 network or AHCI storage
- **Time**: 4-6 hours
- **Add**: DMA, advanced IRQ handling

### Level 5: Full System
- **What**: VFS, network stack, multiple drivers
- **Time**: 1-2 weeks
- **Add**: System services, process management

## Key Concepts (15-Minute Read)

### 1. Root Task = Your Main Function

```rust
let root = unsafe { RootTask::init(config)? };
root.run_with(|broker| {
    // All your system initialization here
});
```

The root task is **your** program. It:
- Runs first after seL4 kernel boots
- Has access to all capabilities
- Spawns your components
- Never exits

### 2. Components = Isolated Programs

Each component:
- Runs in its own address space (isolation!)
- Has its own thread (TCB)
- Communicates via IPC
- Can have device access (optional)

```rust
ComponentConfig {
    name: "my_service",
    entry_point: my_service_main as usize,
    stack_size: 64 * 1024,
    priority: 150,
    device: None,  // or Some(DeviceId::...)
    fault_ep: None,
}
```

### 3. Capability Broker = Resource Manager

The broker manages:
- **Capabilities**: seL4 access tokens
- **Devices**: MMIO, IRQ, DMA
- **Memory**: Untyped memory allocation

```rust
// Request device resources
let device = broker.request_device(DeviceId::Serial { port: 0 })?;
// Now you have: MMIO regions, IRQ handler, DMA pool
```

### 4. IPC = Communication

Components talk via:
- **Shared memory**: Zero-copy, <1μs latency
- **Notifications**: seL4 signals for events
- **Endpoints**: Request/reply messaging

```rust
// Send data via shared ring buffer
ring.push(data)?;
// Signal the consumer
seL4_Signal(notification);
```

## Common Tasks

### Add a New Component

1. **Create entry function:**
```rust
pub extern "C" fn my_component() -> ! {
    loop { /* work */ }
}
```

2. **Spawn it:**
```rust
root.run_with(|broker| {
    spawn_my_component(broker);
});
```

### Add Device Access

```rust
ComponentConfig {
    device: Some(DeviceId::Pci {
        vendor: 0x8086,
        device: 0x100E,
    }),
    // ... rest of config
}
```

The broker automatically allocates MMIO/IRQ/DMA!

### Debug with Logging

```rust
// Use log macros (already configured)
log::info!("Component started!");
log::debug!("Processing: {:?}", data);
log::error!("Failed: {:?}", err);
```

### Test in QEMU

```bash
# 1. Build components (Microkit mode - default)
cargo build --release

# 2. Create system.xml for Microkit
# See examples/system-composition/system.xml

# 3. Generate bootable image
microkit build system.xml

# 4. Run in QEMU
qemu-system-aarch64 -M virt -cpu cortex-a53 -kernel loader.img -nographic
```

**Note:** Microkit requires Linux. On macOS:
- Use Docker: `docker run -it --rm -v $(pwd):/kaal rustlang/rust:nightly`
- Use Lima: `lima cargo build`
- Or use mocks for testing: `cargo test --features mock-sel4`

## Examples to Study

1. **[minimal-component](../examples/minimal-component/)** ⭐
   - Start here!
   - Single component
   - Composable pattern

2. **[system-composition](../examples/system-composition/)**
   - Multi-component system
   - DDDK integration
   - IPC demonstrations

3. **[serial-driver](../examples/serial-driver/)**
   - DDDK driver example
   - Device resources
   - Metadata & probing

## Troubleshooting

### "Component didn't spawn"
- Check bootinfo has available slots
- Verify untyped memory available
- Check capability allocation

### "IRQ already allocated"
- Each IRQ can only be allocated once
- Use different IRQ numbers

### "Out of memory"
- Increase heap size in RootTaskConfig
- Check untyped regions in bootinfo

## Next Steps

1. **Run Examples**: Try all examples in `examples/`
2. **Read Code**: Study the implementations
3. **Modify**: Change stack sizes, priorities, etc.
4. **Build**: Create your own components!

## Resources

- **[SEL4_INTEGRATION.md](SEL4_INTEGRATION.md)** - Dual-mode seL4 deployment (Microkit/Runtime)
- **[SYSTEM_COMPOSITION.md](SYSTEM_COMPOSITION.md)** - Complete integration guide
- **Build Scripts:**
  - `./scripts/build-microkit.sh` - Production deployment (default)
  - `./scripts/build-runtime.sh` - Advanced Rust seL4 runtime
  - `./scripts/build-mock.sh` - Unit tests only

## Join the Community

- 💬 GitHub Discussions: [github.com/your-org/kaal/discussions](https://github.com/your-org/kaal/discussions)
- 🐛 Report Issues: [github.com/your-org/kaal/issues](https://github.com/your-org/kaal/issues)
- 📧 Email: team@kaal.dev

## Philosophy

**Start minimal. Build incrementally. The system is yours!**

You don't need:
- ❌ All features day 1
- ❌ Perfect architecture
- ❌ Every driver

You just need:
- ✅ One working component
- ✅ Willingness to iterate
- ✅ Curiosity to experiment

**Welcome to OS development!** 🚀

---

**Happy Hacking!**

*"From Hello World to Full OS - One Component at a Time"*
