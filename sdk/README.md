# KaaL SDK - Software Development Kit

Clean, ergonomic API for building KaaL microkernel components.

## Overview

The KaaL SDK provides:
- **Type-safe syscall wrappers** - No raw inline assembly required
- **Component patterns** - Structured templates for drivers, services, and apps
- **RAII resource management** - Automatic cleanup
- **IPC integration** - Built-in shared memory ring buffers and notifications
- **Memory management** - Safe allocation and mapping
- **Capability management** - Type-safe capability wrappers

## Modules

### `syscall` - System Call Wrappers

```rust
use kaal_sdk::syscall;

// Print to debug console
syscall::print("Hello!\n");

// Yield to scheduler
syscall::yield_now();

// Allocate memory
let phys_addr = syscall::memory_allocate(4096)?;

// Create notification
let notification = syscall::notification_create()?;
syscall::signal(notification, 0x1)?;
let signals = syscall::poll(notification)?;
```

### `capability` - Capability Management

```rust
use kaal_sdk::capability::Notification;

// RAII-style notification management
let notification = Notification::create()?;
notification.signal(0x42)?;
let signals = notification.poll()?;
// Automatically cleaned up on drop
```

### `memory` - Memory Management

```rust
use kaal_sdk::memory::{PhysicalMemory, MappedMemory, Permissions};

// Allocate physical memory
let phys = PhysicalMemory::allocate(4096)?;

// Map into virtual address space
let mapped = MappedMemory::map(
    phys.phys_addr(),
    phys.size(),
    Permissions::RW
)?;

// Use the memory
unsafe {
    let ptr = mapped.as_mut_ptr::<u32>();
    *ptr = 0x12345678;
}

// Automatically unmapped on drop
```

### `component` - Component Development Patterns

```rust
use kaal_sdk::component::{Component, DriverBase};

kaal_sdk::component_metadata! {
    name: "my_driver",
    type: Driver,
    version: "0.1.0",
    capabilities: ["memory_map:0x09000000", "interrupt:33"],
}

struct MyDriver {
    base: DriverBase,
    // driver-specific state
}

impl Component for MyDriver {
    fn init() -> Result<Self> {
        let base = DriverBase::new("my_driver")?;
        base.register_irq()?;

        Ok(Self { base })
    }

    fn run(&mut self) -> ! {
        loop {
            // Wait for events
            let event = self.base.wait_irq()?;

            // Handle event
            self.handle_event(event);
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    MyDriver::start()
}
```

### `ipc` - Inter-Process Communication

Re-exports from `kaal-ipc`:

```rust
use kaal_sdk::ipc::{SharedRing, Producer, Consumer};

// Create shared memory ring buffer
let ring = SharedRing::<Message, 16>::with_notifications(
    consumer_notify,
    producer_notify
);

// Producer side
let producer = Producer::new(&ring);
producer.push(message)?;

// Consumer side
let consumer = Consumer::new(&ring);
let message = consumer.pop()?;
```

## Component Types

### Device Drivers

Components that interact with hardware:

```rust
// Pattern from SYSTEM_COMPOSITION.md
pub struct SerialDriver {
    base: DriverBase,
    uart_base: *mut u8,          // MMIO
    tx_buffer: RingBuffer,
    rx_buffer: RingBuffer,
}

impl Component for SerialDriver {
    fn init() -> Result<Self> {
        let mut base = DriverBase::new("serial")?;
        base.register_irq()?;

        // Map MMIO region
        let uart_base = memory::map_device(UART_BASE, 4096)?;

        Ok(Self {
            base,
            uart_base,
            tx_buffer: RingBuffer::new(1024),
            rx_buffer: RingBuffer::new(1024),
        })
    }

    fn run(&mut self) -> ! {
        loop {
            match wait_event() {
                Event::IpcMessage(msg) => self.handle_request(msg),
                Event::Interrupt => self.handle_irq(),
            }
        }
    }
}
```

### System Services

Components that provide services without hardware access:

```rust
pub struct FilesystemService {
    base: ServiceBase,
    block_driver_ep: Endpoint,
    cache: PageCache,
}

impl Component for FilesystemService {
    fn init() -> Result<Self> {
        Ok(Self {
            base: ServiceBase::new("filesystem"),
            block_driver_ep: Endpoint::create()?,
            cache: PageCache::new(1024 * 1024),
        })
    }

    fn run(&mut self) -> ! {
        loop {
            // Handle IPC requests
            let request: FileRequest = ipc_recv()?;
            let result = self.handle_request(request)?;
            ipc_reply(result)?;
        }
    }
}
```

### Applications

End-user applications:

```rust
pub struct WebServer {
    network_ep: Endpoint,
    filesystem_ep: Endpoint,
}

impl Component for WebServer {
    fn init() -> Result<Self> {
        Ok(Self {
            network_ep: connect_to_service("network")?,
            filesystem_ep: connect_to_service("filesystem")?,
        })
    }

    fn run(&mut self) -> ! {
        loop {
            let event: NetworkEvent = ipc_recv()?;
            self.handle_request(event)?;
        }
    }
}
```

## Examples

### 1. Hello World (`examples/sdk-hello-world/`)

Demonstrates basic SDK usage:
- Syscall wrappers
- Notification management
- Memory allocation
- Capability management

**Build:**
```bash
cargo build --release
```

**Binary size:** 2.9KB

### 2. Serial Driver (`examples/sdk-serial-driver/`)

Demonstrates device driver pattern:
- Component trait implementation
- Driver base usage
- Event loop structure
- Component metadata

**Build:**
```bash
cargo build --release
```

## Benefits

### Before SDK (Raw Syscalls)

```rust
unsafe {
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {msg_ptr}",
        "mov x1, {msg_len}",
        "svc #0",
        syscall_num = in(reg) 0x1001,
        msg_ptr = in(reg) msg.as_ptr() as usize,
        msg_len = in(reg) msg.len(),
        out("x0") _,
        out("x1") _,
        out("x8") _,
    );
}
```

### With SDK

```rust
syscall::print("Hello!\n");
```

**Improvements:**
- ✅ 70% less boilerplate
- ✅ Type-safe APIs
- ✅ RAII resource management
- ✅ Better error handling (Result types)
- ✅ No raw assembly for users
- ✅ Self-documenting code

## Architecture Alignment

The SDK aligns with SYSTEM_COMPOSITION.md goals:

1. **Component Isolation**: Each component runs in its own address space
2. **Least Privilege**: Components have only needed capabilities
3. **IPC-based Communication**: Built-in IPC support (SharedRing + Notifications)
4. **Composability**: Well-defined Component trait for lifecycle management
5. **Fault Isolation**: Component failures don't affect others

## Building Components

### 1. Create Component Crate

```bash
cargo new --bin my-component
cd my-component
```

### 2. Add SDK Dependency

```toml
[dependencies]
kaal-sdk = { path = "../sdk/kaal-sdk" }

[profile.release]
opt-level = "z"
lto = true
panic = "abort"
```

### 3. Configure for aarch64-unknown-none

`.cargo/config.toml`:
```toml
[build]
target = "aarch64-unknown-none"

[unstable]
build-std = ["core"]
build-std-features = ["compiler-builtins-mem"]
```

### 4. Implement Component

```rust
#![no_std]
#![no_main]

use kaal_sdk::component::Component;

kaal_sdk::component_metadata! {
    name: "my_component",
    type: Driver,  // or Service, Application
    version: "0.1.0",
}

struct MyComponent {
    // state
}

impl Component for MyComponent {
    fn init() -> Result<Self> {
        // Initialize
        Ok(Self { /* ... */ })
    }

    fn run(&mut self) -> ! {
        loop {
            // Event loop
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    MyComponent::start()
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

### 5. Build

```bash
cargo build --release
```

Binary will be at: `target/aarch64-unknown-none/release/my-component`

## Documentation

- [SYSTEM_COMPOSITION.md](../docs/SYSTEM_COMPOSITION.md) - System architecture
- [API Documentation](https://docs.rs/kaal-sdk) - Full API reference (TODO)
- [Examples](../examples/) - Working examples

## Status

**Phase 3: COMPLETE** ✅

- ✅ Core SDK (~600 LOC)
- ✅ Syscall wrappers (syscall.rs)
- ✅ Capability management (capability.rs)
- ✅ Memory management (memory.rs)
- ✅ Component patterns (component.rs)
- ✅ IPC integration (re-export kaal-ipc)
- ✅ Example: Hello World (2.9KB)
- ✅ Example: Serial Driver (component pattern)

## Next Steps

- [ ] Add more component examples (block driver, filesystem service)
- [ ] Create DDDK (Device Driver Development Kit) on top of SDK
- [ ] Add proc macros for component boilerplate reduction
- [ ] Performance benchmarking
- [ ] Integration with build system for component composition

---

**SDK Version:** 0.1.0
**License:** MIT
**Status:** Phase 3 Complete
