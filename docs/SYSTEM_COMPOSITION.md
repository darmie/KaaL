# KaaL System Composition Guide

## Overview

This guide explains how to compose a complete KaaL-based operating system by combining the native Rust microkernel with userspace components and services. It demonstrates the architecture, workflow, and patterns for building composable systems.

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│                   Applications & Services                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │   VFS    │  │ Network  │  │ Process  │  │   App    │   │
│  │ Service  │  │  Stack   │  │ Manager  │  │          │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└────────┬──────────────┬──────────────┬──────────────┬──────┘
         │              │              │              │
         └──────────────┴──────────────┴──────────────┘
                              │
                    IPC (Message Passing)
                              │
┌─────────────────────────────▼───────────────────────────────┐
│                    Runtime Services (EL0)                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Serial  │  │  Timer   │  │  Block   │  │ Network  │   │
│  │  Driver  │  │  Driver  │  │  Driver  │  │  Driver  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└────────┬──────────────┬──────────────┬──────────────┬──────┘
         │              │              │              │
         └──────────────┴──────────────┴──────────────┘
                              │
                        System Calls
                              │
┌─────────────────────────────▼───────────────────────────────┐
│              KaaL Microkernel (Native Rust, EL1)            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Core Mechanisms:                                   │   │
│  │  • Memory Management (MMU, Page Tables)             │   │
│  │  • Thread Control Blocks (TCBs)                     │   │
│  │  • IPC (Endpoints, Notifications)                   │   │
│  │  • Capability System                                │   │
│  │  • Exception Handling                               │   │
│  │  • Scheduling                                       │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│                        Hardware (ARM64)                      │
│  • CPU (Exception Levels: EL0, EL1, EL2, EL3)              │
│  • MMU (Memory Management Unit)                             │
│  • GIC (Generic Interrupt Controller)                       │
│  • Devices (UART, Timer, Storage, Network)                  │
└─────────────────────────────────────────────────────────────┘
```

## Component Architecture

### Kernel Layer (EL1)

The KaaL microkernel provides **core mechanisms only**:

- **Memory Management**: Page tables, address spaces, frame allocation
- **Thread Management**: TCBs, scheduling, context switching
- **IPC**: Synchronous message passing, endpoints, notifications
- **Capabilities**: Fine-grained access control, delegation
- **Exception Handling**: System calls, interrupts, faults

**What the kernel does NOT do:**
- File systems (done in userspace)
- Network protocols (done in userspace)
- Device drivers (done in userspace)
- Policy decisions (done in userspace)

This keeps the kernel small, secure, and verifiable.

### Runtime Services Layer (EL0)

Components running in userspace that provide OS services:

```rust
Component {
    name: "serial_driver",
    privileges: Minimal,        // Only what's needed
    address_space: Isolated,    // Can't access other components
    capabilities: [
        MemoryMap(0x09000000),  // UART MMIO region
        Interrupt(IRQ_UART),    // UART interrupt
        IpcEndpoint(serial_ep), // IPC with other components
    ],
}
```

Each component:
- Runs in its own address space (isolation)
- Has only the capabilities it needs (least privilege)
- Communicates via IPC (no shared memory by default)
- Can crash without affecting others (fault isolation)

### Application Layer (EL0)

User applications and high-level services:

```rust
Component {
    name: "web_server",
    capabilities: [
        IpcEndpoint(network_ep),  // Talk to network driver
        IpcEndpoint(fs_ep),       // Talk to filesystem
    ],
}
```

Applications have even fewer privileges than drivers:
- No hardware access
- No memory management
- Only IPC to authorized services

## System Composition Workflow

### Step 1: Boot Sequence

```
1. Power On / Reset
   ↓
2. Elfloader (EL2)
   • Loads kernel binary
   • Loads root task binary
   • Sets up initial page tables
   • Jumps to kernel
   ↓
3. Kernel Initialization (EL1)
   • Chapter 1: UART, device tree, early init
   • Chapter 2: Frame allocator, MMU setup
   • Chapter 3: Exception vectors, syscalls
   • Chapter 7: Root task creation
   ↓
4. Transition to EL0
   • Kernel calls ERET
   • CPU drops to userspace
   ↓
5. Root Task Starts (EL0)
   • First userspace program
   • Spawns system components
   • Configures system
```

### Step 2: Root Task Initialization

The root task is the first userspace program. It bootstraps the system:

```rust
// runtime/root-task/src/main.rs

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Initialize root task environment
    init_heap();
    init_logging();

    // 2. Parse kernel boot info
    let boot_info = parse_kernel_bootinfo();

    // 3. Spawn system components
    spawn_serial_driver(boot_info);
    spawn_timer_driver(boot_info);
    spawn_block_driver(boot_info);

    // 4. Spawn system services
    spawn_filesystem_service();
    spawn_network_stack();
    spawn_process_manager();

    // 5. Enter event loop
    root_task_event_loop();
}
```

### Step 3: Component Spawning

Each component is created by the root task:

```rust
fn spawn_serial_driver(boot_info: &BootInfo) {
    // 1. Allocate resources
    let tcb = create_tcb(
        priority: 200,      // High priority for drivers
        affinity: Core0,    // Pin to core 0
    );

    let address_space = create_address_space(
        size: 4MB,          // Small address space
    );

    let stack = allocate_stack(
        size: 64KB,         // Driver stack
    );

    // 2. Grant capabilities
    let caps = [
        // Hardware access
        Capability::MemoryMap {
            physical: 0x09000000,  // UART base address
            size: 4096,            // One page
            permissions: ReadWrite,
        },
        Capability::Interrupt {
            irq: IRQ_UART,
        },

        // IPC endpoints
        Capability::Endpoint {
            id: serial_ep,
            rights: SendRecv,
        },
    ];

    // 3. Load component binary
    load_elf_binary(
        path: "serial_driver.elf",
        address_space: address_space,
    );

    // 4. Configure TCB
    configure_tcb(tcb, {
        entry_point: 0x1000,      // From ELF
        stack_pointer: stack_top,
        capabilities: caps,
    });

    // 5. Start component
    syscall::tcb_resume(tcb);
}
```

### Step 4: IPC Communication

Components communicate via message passing:

```rust
// Application wants to print to serial

// Send message to serial driver
let message = SerialMessage {
    command: Write,
    data: b"Hello, World!\n",
};

syscall::ipc_send(serial_ep, &message)?;

// Serial driver receives
let message: SerialMessage = syscall::ipc_recv()?;
match message.command {
    Write => {
        // Write to UART hardware
        uart_write(message.data);

        // Send reply
        syscall::ipc_reply(Ok(()));
    }
}
```

**IPC Properties:**
- Synchronous (blocking until reply)
- Type-safe (Rust types)
- Fast (<1μs for small messages)
- Secure (capability-protected)

## Component Patterns

### Pattern 1: Device Driver

```rust
// Minimal driver structure
pub struct SerialDriver {
    uart_base: *mut u8,          // MMIO base
    irq_notification: Notification,  // IRQ signaling
    tx_buffer: RingBuffer,       // Transmit queue
    rx_buffer: RingBuffer,       // Receive queue
}

impl SerialDriver {
    pub fn init() -> Self {
        // Map MMIO region
        let uart_base = map_device(UART_BASE, 4096);

        // Register IRQ handler
        let irq_notification = register_irq(IRQ_UART);

        Self {
            uart_base,
            irq_notification,
            tx_buffer: RingBuffer::new(1024),
            rx_buffer: RingBuffer::new(1024),
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            // Wait for IPC or IRQ
            let event = syscall::wait();

            match event {
                Event::IpcMessage(msg) => self.handle_request(msg),
                Event::Interrupt => self.handle_irq(),
            }
        }
    }

    fn handle_request(&mut self, msg: SerialMessage) {
        match msg.command {
            Write => {
                self.tx_buffer.push(msg.data);
                self.start_transmission();
            }
        }
    }

    fn handle_irq(&mut self) {
        // Read from hardware, push to rx_buffer
        // Continue transmission from tx_buffer
    }
}
```

### Pattern 2: System Service

```rust
// Filesystem service (no hardware access)
pub struct FilesystemService {
    block_driver_ep: Endpoint,   // Talk to block driver
    cache: PageCache,            // In-memory cache
    mount_table: Vec<Mount>,     // Mounted filesystems
}

impl FilesystemService {
    pub fn run(&mut self) -> ! {
        loop {
            // Wait for file operations
            let request: FileRequest = syscall::ipc_recv()?;

            let result = match request.op {
                Open(path) => self.open(path),
                Read(fd, buf) => self.read(fd, buf),
                Write(fd, data) => self.write(fd, data),
                Close(fd) => self.close(fd),
            };

            // Reply to caller
            syscall::ipc_reply(result)?;
        }
    }

    fn read(&mut self, fd: FileDescriptor, buf: &mut [u8]) -> Result<usize> {
        // Check cache
        if let Some(data) = self.cache.get(fd) {
            return Ok(data.copy_to(buf));
        }

        // Cache miss - read from block driver via IPC
        let block_request = BlockRequest::Read {
            block: fd.block,
            count: 1,
        };

        let data = syscall::ipc_call(self.block_driver_ep, block_request)?;

        // Update cache
        self.cache.insert(fd, data);

        Ok(data.copy_to(buf))
    }
}
```

### Pattern 3: Application

```rust
// Web server application
pub struct WebServer {
    network_ep: Endpoint,        // Network stack
    filesystem_ep: Endpoint,     // Filesystem
    connections: Vec<Connection>,
}

impl WebServer {
    pub fn run(&mut self) -> ! {
        loop {
            // Wait for network events
            let event: NetworkEvent = syscall::ipc_recv()?;

            match event {
                NewConnection(conn) => {
                    self.connections.push(conn);
                }

                DataReceived(conn, data) => {
                    let request = parse_http_request(data)?;
                    let response = self.handle_request(request)?;

                    // Send response via network stack
                    syscall::ipc_send(
                        self.network_ep,
                        NetworkRequest::Send(conn, response)
                    )?;
                }
            }
        }
    }

    fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse> {
        // Read file from filesystem
        let file = syscall::ipc_call(
            self.filesystem_ep,
            FileRequest::Open(req.path)
        )?;

        let content = syscall::ipc_call(
            self.filesystem_ep,
            FileRequest::Read(file, 4096)
        )?;

        Ok(HttpResponse {
            status: 200,
            body: content,
        })
    }
}
```

## Development Status

### ✅ Currently Working

- **Kernel boot** (Chapters 1-3, 7)
- **Memory management** (frame allocator, page tables)
- **Exception handling** (syscalls, traps)
- **Userspace execution** (EL0 transition)

**You can build:**
- Boot to userspace
- Make syscalls from root task
- Run simple userspace programs

### 🚧 In Progress

- **TCB management** (Chapter 4) - Creating and managing threads
- **IPC** (Chapter 5) - Message passing between components
- **Capabilities** (Chapter 6) - Fine-grained access control

### 📝 Planned

- **Interrupts** (Chapter 8) - Hardware interrupt handling
- **Virtual memory** (Chapter 9) - Dynamic memory management
- **Device management** (Chapter 10) - Standardized driver interface
- **Scheduling** (Chapter 11) - Multi-threaded execution
- **Advanced features** (Chapter 12) - Notifications, shared memory

## Building a Complete System

### Example: Minimal System

```bash
# Define your system in build-config.toml

[system]
name = "minimal-system"

[[components]]
name = "root-task"
path = "runtime/root-task"

[[components]]
name = "serial-driver"
path = "drivers/serial"

[[components]]
name = "hello-app"
path = "apps/hello"

# Build
./build.sh --platform qemu-virt

# Run
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
```

### Example: Full System

```
System Components:
├── root-task (EL0)           - System bootstrap
├── Drivers (EL0)
│   ├── serial-driver         - UART console
│   ├── timer-driver          - System clock
│   ├── block-driver          - VirtIO disk
│   └── network-driver        - VirtIO network
├── System Services (EL0)
│   ├── filesystem-service    - VFS and file I/O
│   ├── network-stack         - TCP/IP stack
│   └── process-manager       - Process lifecycle
└── Applications (EL0)
    ├── shell                 - Command line
    ├── web-server            - HTTP server
    └── user-apps             - Custom applications
```

## Best Practices

### 1. Keep Components Small

- Each component should do one thing well
- Small address spaces (faster context switches)
- Minimal capabilities (better security)

### 2. Use IPC for Everything

- No shared memory by default (better isolation)
- Explicit communication (easier to reason about)
- Type-safe messages (catch errors at compile time)

### 3. Fail Gracefully

- Components should handle errors locally
- Use Result types for all operations
- Don't panic in drivers (restart instead)

### 4. Design for Composability

- Well-defined interfaces (IPC message types)
- Stateless when possible (easier to restart)
- Small, focused components (easier to replace)

## Testing

### Unit Tests

```bash
# Test individual kernel modules
cd kernel
cargo test

# Test components in isolation
cd drivers/serial
cargo test --lib
```

### Integration Tests

```bash
# Build complete system
./build.sh --platform qemu-virt

# Run automated tests in QEMU
./test.sh --integration
```

### Hardware Testing

```bash
# Build for real hardware
./build.sh --platform rpi4

# Flash to SD card
dd if=bootimage.bin of=/dev/sdX bs=4M

# Boot on hardware
# (connect serial console to see output)
```

## Next Steps

1. **Understand the architecture** (you're here!)
2. **Read the code** - Start with `kernel/src/main.rs`
3. **Study boot process** - Follow the boot from elfloader to root task
4. **Build examples** - Try the example components
5. **Create your own component** - Start with a simple driver
6. **Contribute** - Help implement the next chapter!

## References

- **[GETTING_STARTED.md](GETTING_STARTED.md)** - Build and run instructions
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Detailed architecture design
- **[MICROKERNEL_CHAPTERS.md](MICROKERNEL_CHAPTERS.md)** - Development roadmap
- **[HOBBYIST_GUIDE.md](HOBBYIST_GUIDE.md)** - Beginner-friendly guide

---

**Document Version:** 2.0
**Last Updated:** 2025-10-14
**Status:** KaaL Framework - Native Rust Microkernel (Chapter 7 Complete)
