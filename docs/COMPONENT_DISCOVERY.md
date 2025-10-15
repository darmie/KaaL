# Component Discovery and Initialization

This document describes how KaaL discovers, loads, and spawns system components.

## Overview

KaaL uses a **declarative component system** where components are:
1. Defined in a manifest file (`components.toml`)
2. Discovered by root-task at boot
3. Spawned by the **system_init** component
4. Managed through their lifecycle

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Boot Sequence                          │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│  Kernel (EL1)                                               │
│  • Initialize MMU, interrupts, syscalls                     │
│  • Create root-task (first userspace process)              │
│  • Jump to EL0                                              │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│  Root Task (EL0)                                            │
│  • Read components.toml manifest                            │
│  • Embed component binaries at compile time                 │
│  • Spawn system_init component with elevated privileges     │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│  system_init Component (EL0)                                │
│  • Discovers components from manifest/registry              │
│  • Phase 1: Spawn device drivers                            │
│  • Phase 2: Spawn system services                           │
│  • Phase 3: Spawn user applications                         │
│  • Phase 4: Monitor component health                        │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
         ┌──────────────┴──────────────┐
         │                             │
┌────────▼───────┐           ┌─────────▼──────┐
│    Drivers     │           │    Services    │
│                │           │                │
│ • serial       │           │ • process_mgr  │
│ • timer        │           │ • vfs          │
│ • gpio         │           │ • network      │
└────────────────┘           └────────────────┘
```

## Component Manifest Format

Components are defined in `runtime/root-task/components.toml`:

```toml
[[component]]
name = "serial_driver"
binary = "serial-driver"      # Binary name (without path)
type = "driver"               # driver | service | application
priority = 200                # 0-255 (higher = more important)
autostart = true              # Spawn automatically at boot
capabilities = [
    "memory_map:0x09000000:4096",  # UART0 MMIO region
    "interrupt:33",                 # UART0 IRQ
    "ipc:serial",                   # IPC endpoint name
]
```

### Component Types

- **Driver**: Hardware access (MMIO, interrupts, DMA)
- **Service**: System service (no hardware, provides IPC services)
- **Application**: User-facing program (minimal privileges)

### Capability Syntax

- `memory_map:ADDR:SIZE` - Physical memory mapping
- `interrupt:IRQ` - Interrupt capability
- `ipc:NAME` - IPC endpoint access
- `process:create` - Can spawn processes
- `process:destroy` - Can terminate processes
- `memory:allocate` - General memory allocation

## system_init Component

The **system_init** component is the first component spawned by root-task. It acts as the system bootstrapper with elevated privileges.

### Responsibilities

1. **Component Discovery**
   - Read component registry/manifest
   - Validate component requirements
   - Determine spawn order

2. **Component Spawning**
   - Load component ELF binaries
   - Create process resources (TCB, address space, stack)
   - Grant capabilities per manifest
   - Start process execution

3. **Lifecycle Management**
   - Monitor component health
   - Detect crashed components
   - Restart failed components (if configured)
   - Handle graceful shutdown

### Implementation

```rust
use kaal_sdk::component::{Component, ServiceBase};

// Declare system_init with elevated privileges
kaal_sdk::component_metadata! {
    name: "system_init",
    type: Service,
    version: "0.1.0",
    capabilities: ["process:create", "memory:allocate", "ipc:*"],
}

struct SystemInit {
    base: ServiceBase,
    components_spawned: usize,
}

impl Component for SystemInit {
    fn init() -> Result<Self> {
        // Initialize system bootstrapper
    }

    fn run(&mut self) -> ! {
        // Phase 1: Spawn drivers
        self.spawn_drivers();

        // Phase 2: Spawn services
        self.spawn_services();

        // Phase 3: Spawn applications
        self.spawn_applications();

        // Phase 4: Monitor components
        self.monitoring_loop()
    }
}
```

### Binary

- **Location**: `examples/system-init/`
- **Binary Size**: 3.3KB
- **Language**: Rust (no_std, uses KaaL SDK)

## Component Discovery Methods

### 1. Static Registry (Compile-Time)

Components known at compile time are embedded in root-task:

```rust
// In root-task/src/component_loader.rs

static COMPONENTS: &[ComponentDescriptor] = &[
    ComponentDescriptor::new("serial_driver", "serial-driver", ComponentType::Driver)
        .with_priority(200)
        .with_autostart(true)
        .with_capabilities(&[
            ComponentCapability::MemoryMap { phys_addr: 0x09000000, size: 4096 },
            ComponentCapability::Interrupt { irq: 33 },
        ])
        .with_binary(include_bytes!("../../target/serial-driver")),
];

let registry = ComponentRegistry::new(COMPONENTS);
```

### 2. Component Metadata Section

Components can embed their own metadata using the SDK macro:

```rust
// In component binary (e.g., serial-driver)

kaal_sdk::register_component! {
    name: "serial_driver",
    type: Driver,
    version: "0.1.0",
    priority: 200,
    autostart: true,
    capabilities: ["memory_map:0x09000000", "interrupt:33"],
}
```

This places metadata in `.component_registry` ELF section that can be read by the loader.

### 3. Manifest File (Runtime)

The `components.toml` manifest can be parsed at runtime (future enhancement):

```rust
// Parse TOML manifest
let manifest = parse_toml(COMPONENTS_TOML);

for component in manifest.components() {
    if component.autostart {
        spawn_component(component);
    }
}
```

## Component Spawning Process

When system_init spawns a component:

### 1. Binary Loading

```rust
// Load ELF binary
let elf_data = component.binary_data;
let elf_header = parse_elf_header(elf_data)?;

// Validate ELF (aarch64, ET_EXEC, proper entry point)
validate_elf(&elf_header)?;
```

### 2. Resource Allocation

```rust
// Create TCB (thread control block)
let tcb = syscall::process_create(
    priority: component.priority,
    affinity: Core0,
)?;

// Allocate address space
let address_space = syscall::address_space_create(
    size: 4MB,  // Initial address space size
)?;

// Allocate stack
let stack = syscall::memory_allocate(64 * 1024)?;  // 64KB stack
```

### 3. Capability Granting

```rust
for cap in component.capabilities {
    match cap {
        ComponentCapability::MemoryMap { phys_addr, size } => {
            let virt_addr = syscall::memory_map(
                tcb,
                phys_addr,
                size,
                PERM_READ | PERM_WRITE,
            )?;
        }
        ComponentCapability::Interrupt { irq } => {
            let irq_cap = syscall::irq_control_get(irq)?;
            syscall::capability_grant(tcb, irq_cap)?;
        }
        ComponentCapability::IpcEndpoint { name } => {
            let endpoint = syscall::endpoint_create()?;
            syscall::capability_grant(tcb, endpoint)?;
            // Register endpoint in name service
            name_service::register(name, endpoint);
        }
    }
}
```

### 4. ELF Segment Loading

```rust
for segment in elf_header.program_headers() {
    if segment.p_type == PT_LOAD {
        // Map segment into address space
        syscall::memory_map(
            tcb,
            segment.p_paddr,     // Physical address
            segment.p_vaddr,     // Virtual address
            segment.p_memsz,     // Size
            segment.p_flags,     // Permissions
        )?;

        // Copy segment data
        unsafe {
            core::ptr::copy_nonoverlapping(
                elf_data.as_ptr().add(segment.p_offset),
                segment.p_vaddr as *mut u8,
                segment.p_filesz,
            );
        }
    }
}
```

### 5. Process Start

```rust
// Configure TCB
syscall::tcb_configure(
    tcb,
    entry_point: elf_header.e_entry,
    stack_pointer: stack_top,
)?;

// Start process
syscall::tcb_resume(tcb)?;
```

## Component SDK Integration

Components use the KaaL SDK for standardized patterns:

### Component Trait

```rust
pub trait Component: Sized {
    fn init() -> Result<Self>;
    fn run(&mut self) -> !;

    fn start() -> ! {
        match Self::init() {
            Ok(mut component) => component.run(),
            Err(_) => {
                // Failed to initialize
                loop { core::arch::asm!("wfi"); }
            }
        }
    }
}
```

### Metadata Declaration

```rust
kaal_sdk::component_metadata! {
    name: "my_driver",
    type: Driver,
    version: "0.1.0",
    capabilities: ["memory_map:0x09000000", "interrupt:33"],
}
```

### Driver Base

```rust
use kaal_sdk::component::DriverBase;

struct MyDriver {
    base: DriverBase,
    // driver-specific state
}

impl Component for MyDriver {
    fn init() -> Result<Self> {
        let mut base = DriverBase::new("my_driver")?;
        base.register_irq()?;  // Register IRQ handler

        Ok(Self { base })
    }

    fn run(&mut self) -> ! {
        loop {
            // Wait for IRQ or IPC
            let signals = self.base.wait_irq()?;
            self.handle_interrupt(signals);
        }
    }
}
```

## Component Registry

The component loader maintains a registry of known components:

```rust
pub struct ComponentRegistry {
    components: &'static [ComponentDescriptor],
}

impl ComponentRegistry {
    pub fn components(&self) -> &[ComponentDescriptor];
    pub fn autostart_components(&self) -> impl Iterator<Item = &ComponentDescriptor>;
    pub fn find(&self, name: &str) -> Option<&ComponentDescriptor>;
}
```

### Usage in Root Task

```rust
// Define component registry
static COMPONENT_REGISTRY: ComponentRegistry = ComponentRegistry::new(&[
    // system_init must be first
    ComponentDescriptor::new("system_init", "system-init", ComponentType::Service)
        .with_priority(255)
        .with_autostart(true)
        .with_binary(include_bytes!("../../../target/system-init")),

    // Other components embedded here
]);

// In root-task main()
let loader = ComponentLoader::new(&COMPONENT_REGISTRY);

// Spawn system_init (it will spawn the rest)
loader.spawn("system_init")?;
```

## Future Enhancements

### Dynamic Component Loading

Load components from filesystem at runtime:

```rust
// Read component binary from VFS
let binary = vfs::read("/system/drivers/serial_driver")?;

// Parse ELF and spawn
let component = ComponentDescriptor::from_elf(binary)?;
loader.spawn_component(component)?;
```

### Component Marketplace

Discover and install components:

```rust
// Search for components
let results = marketplace::search("network driver")?;

// Install component
marketplace::install("network_driver_v2")?;

// Spawn installed component
loader.spawn("network_driver_v2")?;
```

### Hot Reload

Reload components without system restart:

```rust
// Stop component
loader.stop("serial_driver")?;

// Load new version
let new_binary = vfs::read("/system/drivers/serial_driver.new")?;

// Replace component
loader.replace("serial_driver", new_binary)?;

// Restart component
loader.start("serial_driver")?;
```

### Dependency Management

Components declare dependencies:

```toml
[[component]]
name = "network_driver"
dependencies = ["dma_service", "interrupt_controller"]
```

Loader ensures dependencies are spawned first:

```rust
fn spawn_with_dependencies(name: &str) -> Result<()> {
    let component = registry.find(name)?;

    // Spawn dependencies first
    for dep in component.dependencies {
        if !is_running(dep) {
            spawn_with_dependencies(dep)?;
        }
    }

    // Spawn component
    spawn_component(component)?;
}
```

## Summary

KaaL's component discovery system provides:

✅ **Declarative Configuration** - Components defined in TOML manifest
✅ **system_init Bootstrapper** - First component with elevated privileges
✅ **Phased Initialization** - Drivers → Services → Applications
✅ **Capability-based Security** - Fine-grained access control
✅ **SDK Integration** - Standardized component patterns
✅ **Lifecycle Management** - Health monitoring and restart

This enables building composable, secure, and maintainable microkernel systems.

## References

- [SYSTEM_COMPOSITION.md](./SYSTEM_COMPOSITION.md) - Overall system architecture
- [sdk/README.md](../sdk/README.md) - SDK component patterns
- [runtime/root-task/components.toml](../runtime/root-task/components.toml) - Component manifest
- [examples/system-init/](../examples/system-init/) - system_init implementation
