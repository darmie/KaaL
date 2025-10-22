# Component Configuration Guide

## Quick Start

All system components are configured in **`components.toml`** at the project root.

```toml
[[component]]
name = "my_driver"
binary = "my-driver"           # Binary name in target/
type = "driver"                # driver | service | application
priority = 200                 # 0-255 (higher = more important)
autostart = true               # Spawn automatically at boot
capabilities = [
    "memory_map:0x09000000:4096",  # UART MMIO
    "interrupt:33",                 # UART IRQ
    "ipc:serial",                   # IPC endpoint
]
```

## Why components.toml is at Project Root

**Developer-Friendly Location**: No need to navigate into `runtime/` or `kernel/` directories.

**Easy Discovery**: New developers can immediately see what components exist.

**Simple Configuration**: Add/remove/configure components without touching internal code.

**Build-Time Integration**: The manifest is automatically embedded into the system.

## Component Types

### Driver
Device drivers with hardware access (MMIO, interrupts, DMA).

**Example**: Serial driver, timer driver, network card driver

**Typical Capabilities**:
- `memory_map:ADDR:SIZE` - Physical memory access
- `interrupt:IRQ` - Interrupt handling
- `ipc:NAME` - Communication with services

### Service
System services that provide functionality via IPC (no hardware access).

**Example**: VFS, process manager, network stack

**Typical Capabilities**:
- `ipc:NAME` - IPC endpoints
- `process:create` - Can spawn processes (for process manager)
- `memory:allocate` - Memory allocation

### Application
User-facing programs with minimal privileges.

**Example**: Shell, web server, editor

**Typical Capabilities**:
- `ipc:NAME` - Talk to services only

## Capability Syntax

| Capability | Format | Example | Description |
|------------|--------|---------|-------------|
| Memory Map | `memory_map:ADDR:SIZE` | `memory_map:0x09000000:4096` | Map physical memory |
| Interrupt | `interrupt:IRQ` | `interrupt:33` | Access IRQ |
| IPC Endpoint | `ipc:NAME` | `ipc:serial` | IPC communication |
| Process Create | `process:create` | `process:create` | Spawn processes |
| Process Destroy | `process:destroy` | `process:destroy` | Terminate processes |
| Memory Allocate | `memory:allocate` | `memory:allocate` | Allocate memory |

## Boot Sequence

1. **Kernel boots** → Creates root-task (first userspace process)
2. **Root-task reads** `components.toml` → Discovers components
3. **Root-task spawns** `system_init` → First component with elevated privileges
4. **system_init spawns** remaining components:
   - **Phase 1**: Device drivers (serial, timer, etc.)
   - **Phase 2**: System services (process_manager, vfs, etc.)
   - **Phase 3**: User applications (shell, etc.)

## Component Priority

Components are assigned priorities (0-255):

- **255**: root-task (highest priority)
- **200**: system_init (high priority)
- **150**: Device drivers (high priority)
- **100**: Core system services
- **80**: Optional services
- **50**: User applications (lowest priority)

Higher priority = more CPU time when multiple components are runnable.

## Autostart Flag

- `autostart = true`: Spawned automatically at boot by system_init
- `autostart = false`: Spawned on-demand by services

**Tip**: Only mark essential components as autostart. On-demand components reduce boot time and memory usage.

## Adding a New Component

### 1. Write Your Component

Create a component using the KaaL SDK:

```rust
// examples/my-component/src/main.rs

#![no_std]
#![no_main]

use kaal_sdk::{
    component::{Component, DriverBase},
    syscall,
};

kaal_sdk::component_metadata! {
    name: "my_component",
    type: Driver,
    version: "0.1.0",
    capabilities: ["memory_map:0x09000000:4096", "interrupt:33"],
}

struct MyComponent {
    base: DriverBase,
}

impl Component for MyComponent {
    fn init() -> kaal_sdk::Result<Self> {
        let base = DriverBase::new("my_component")?;
        Ok(Self { base })
    }

    fn run(&mut self) -> ! {
        loop {
            // Event loop
            syscall::yield_now();
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    MyComponent::start()
}
```

### 2. Add to components.toml

Add an entry to `components.toml` at the project root:

```toml
[[component]]
name = "my_component"
binary = "my-component"
type = "driver"
priority = 200
autostart = true
capabilities = [
    "memory_map:0x09000000:4096",
    "interrupt:33",
    "ipc:my_component",
]
```

### 3. Build

```bash
# Build your component
cargo build --release -p my-component

# Build the system (embeds components.toml)
./build.sh --platform qemu-virt

# Run in QEMU
qemu-system-aarch64 -machine virt -cpu cortex-a57 -nographic -kernel output/kaal-image
```

### 4. Verify

At boot, you'll see:

```
🔍 Component manifest: /path/to/project/components.toml
📦 Found 7 component(s)

[system_init] Phase 1: Spawning device drivers...
  → my_component...
    ✓ my_component spawned
```

## Example Components

### Serial Driver (Driver)
```toml
[[component]]
name = "serial_driver"
binary = "serial-driver"
type = "driver"
priority = 200
autostart = true
capabilities = ["memory_map:0x09000000:4096", "interrupt:33", "ipc:serial"]
```

### Process Manager (Service)
```toml
[[component]]
name = "process_manager"
binary = "process-manager"
type = "service"
priority = 150
autostart = true
capabilities = ["process:create", "process:destroy", "memory:allocate", "ipc:procmgr"]
```

### Shell (Application)
```toml
[[component]]
name = "shell"
binary = "shell"
type = "application"
priority = 50
autostart = false  # Spawned after system is ready
capabilities = ["ipc:serial", "ipc:vfs", "ipc:procmgr"]
```

## Best Practices

✅ **Minimal Capabilities**: Only grant what's needed (least privilege)

✅ **Descriptive Names**: Use clear, descriptive component names

✅ **Autostart Wisely**: Only mark essential components as autostart

✅ **Priority Order**: Higher priority for time-sensitive components

✅ **Document Capabilities**: Comment why each capability is needed

❌ **Don't**: Grant `ipc:*` to non-system components

❌ **Don't**: Give hardware access to applications

❌ **Don't**: Autostart heavy services that aren't needed immediately

## Troubleshooting

### Component Not Found

```
Error: Component 'my_component' not found in manifest
```

**Solution**: Check that `my_component` is defined in `components.toml` and the binary name matches.

### Build Fails: "components.toml not found"

```
Error: components.toml not found at project root
```

**Solution**: Ensure `components.toml` exists at the project root (not in `runtime/` or `kernel/`).

### Component Won't Spawn

**Check**:
1. Is `autostart = true`?
2. Are required capabilities granted?
3. Is the binary built and in `target/`?
4. Check system_init logs for error messages

## Further Reading

- [COMPONENT_DISCOVERY.md](docs/COMPONENT_DISCOVERY.md) - Detailed architecture
- [SYSTEM_COMPOSITION.md](docs/SYSTEM_COMPOSITION.md) - Overall system design
- [sdk/README.md](sdk/README.md) - SDK component patterns
- [components/system-init/](examples/system-init/) - system_init implementation

## Summary

🎯 **Simple**: Configure all components in one file at project root

🔒 **Secure**: Fine-grained capability control per component

📦 **Modular**: Easy to add/remove components

🚀 **Automated**: Build system handles discovery and embedding

Happy component building! 🦀
