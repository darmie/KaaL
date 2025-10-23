# KaaL Hobbyist Guide: Build Your OS in Hours, Not Years

## Welcome!

You're about to build your own operating system using a clean-room native Rust microkernel. No PhD required!

## What Makes KaaL Special?

- **Native Rust Microkernel**: Built from scratch in pure Rust for ARM64
- **Fast to Start**: Get a working kernel in minutes
- **Simple & Clean**: Capability-based security without complexity
- **Composable**: Build your OS incrementally, component by component
- **MIT Licensed**: Use it however you want

## Quick Start (5 Minutes)

### 1. Install Prerequisites

You only need two things:

```bash
# 1. Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly
rustup target add aarch64-unknown-none

# 2. Install QEMU (for testing)
# macOS:
brew install qemu

# Linux:
sudo apt install qemu-system-aarch64
```

### 2. Build and Run

```bash
# Clone the repository
git clone https://github.com/darmie/kaal.git
cd kaal

# Build the kernel
./build.sh --platform qemu-virt

# Run in QEMU
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
```

**You'll see:**
```
╔═══════════════════════════════════════════╗
║            KaaL Framework                 ║
║     Native Rust ARM64 Microkernel        ║
╚═══════════════════════════════════════════╝

[Elfloader] Loading kernel...
[Kernel] Chapter 1: Bare Metal Boot - OK
[Kernel] Chapter 2: Memory Management - OK
[Kernel] Chapter 3: Exception Handling - OK
[Kernel] Chapter 7: Root Task Boot - OK
[Kernel] Transitioning to EL0...

Hello from userspace!
```

**Congratulations!** You just booted a capability-based microkernel with:
- Memory isolation
- Syscall interface
- Userspace execution
- ARM64 exception handling

### 3. Exit QEMU

Press `Ctrl-A` then `X` to exit.

## Understanding KaaL Architecture

### The Big Picture

```
┌─────────────────────────────────────────┐
│         Userspace (EL0)                 │
│  • Root Task                            │
│  • Components (your OS services)        │
│  • Applications                         │
└──────────────┬──────────────────────────┘
               │ syscalls
┌──────────────▼──────────────────────────┐
│         Kernel (EL1)                    │
│  • Memory Management                    │
│  • Capability System                    │
│  • Thread Scheduling                    │
│  • Exception Handling                   │
│  • IPC                                  │
└─────────────────────────────────────────┘
```

KaaL is a **microkernel**, which means:
- The kernel is small (core mechanisms only)
- Most OS services run in userspace
- Everything communicates via IPC
- Components are isolated for security

### Key Concepts

#### 1. Native Rust Microkernel

Unlike other teaching OS projects, KaaL is:
- **Clean-room implementation**: Written from scratch in Rust
- **No C dependencies**: 100% Rust, no legacy code
- **ARM64-native**: Targets modern 64-bit ARM processors
- **Capability-based**: Fine-grained access control built-in

#### 2. Composable Components

Build your OS by composing isolated components:

```rust
// Each component is an isolated program
Component {
    name: "filesystem",
    entry_point: fs_main,
    memory: 512KB,
    capabilities: [ReadDisk, WriteCache],
}
```

Components:
- Run in their own address space
- Have minimal privileges (only what they need)
- Communicate via IPC (Inter-Process Communication)
- Can't crash other components

#### 3. Capabilities = Access Rights

Instead of "root can do anything", KaaL uses **capabilities**:
- A capability is a token that grants a specific right
- No capability = no access (even for "root")
- Capabilities can't be forged or stolen
- Fine-grained: access specific memory, not all memory

Example:
```rust
// Component can ONLY write to serial port
capabilities: [
    SerialWrite(port: 0),  // Can write to serial0
    // No SerialRead - can't read
    // No DiskAccess - can't touch disk
]
```

## Building Your First Component

### Step 1: Write the Component

Create `components/my-service/src/main.rs`:

```rust
#![no_std]
#![no_main]

use kaal_sdk::{
    component::Component,
    printf,
    syscall,
};

// Declare your component using the SDK macro
kaal_sdk::component! {
    name: "my_service",
    type: Service,
    version: "0.1.0",
    capabilities: ["memory:map", "caps:allocate"],
    impl: MyService
}

/// Your component implementation
pub struct MyService {
    counter: usize,
}

impl Component for MyService {
    fn init() -> kaal_sdk::Result<Self> {
        printf!("╔═══════════════════════════════════════╗\n");
        printf!("║       My Service v0.1.0              ║\n");
        printf!("╚═══════════════════════════════════════╝\n");
        printf!("\n");
        printf!("Service initialized successfully!\n");

        Ok(Self { counter: 0 })
    }

    fn run(&mut self) -> ! {
        printf!("Entering main loop...\n");

        loop {
            // Do useful work
            self.counter += 1;

            if self.counter % 1000 == 0 {
                printf!("Service running: {} iterations\n", self.counter);
            }

            // Yield CPU to other processes
            syscall::yield_now();
        }
    }
}
```

### Step 2: Configure Your Component

Create `components/my-service/Cargo.toml`:

```toml
[package]
name = "my-service"
version = "0.1.0"
edition = "2021"

[dependencies]
kaal-sdk = { path = "../../sdk/kaal-sdk" }

[[bin]]
name = "my-service"
path = "src/main.rs"

[profile.release]
panic = "abort"
lto = true
opt-level = "z"
codegen-units = 1
```

### Step 3: Register in Components Config

Add to `components.toml`:

```toml
[[component]]
name = "my_service"
binary = "my-service"
type = "service"
priority = 100
autostart = false  # Set true to spawn at boot
spawned_by = "system_init"  # Spawned via capability-based allocation
capabilities = [
    "memory:map",      # Can map memory
    "caps:allocate",   # Can allocate capability slots
]
```

### Step 4: Build and Test

```bash
# Build using the Nushell build system
nu build.nu

# Run in QEMU
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader

# If autostart=true, your service will spawn automatically!
# Otherwise, it will be available for on-demand spawning.
```

**What happens:**
1. Build system compiles your component binary
2. system_init embeds your component's ELF in its registry
3. At boot, system_init spawns your component from UntypedMemory
4. Your `init()` runs, then `run()` loop starts
5. Component runs alongside other processes (ipc_producer, ipc_consumer, notepad)

## Incremental Development Path

Start simple, add features as you learn:

### Level 1: Hello World (5 minutes)
- **Goal**: Get one component running
- **What you'll learn**: Build system, bootloader, syscalls
- **Code**: ~20 lines
- **Status**: ✅ Working (you just did this!)

### Level 2: Multiple Components (30 minutes)
- **Goal**: Run 2-3 components simultaneously
- **What you'll learn**: Component isolation, address spaces
- **Code**: ~100 lines
- **Status**: 🚧 In progress (Chapter 4)

### Level 3: IPC Communication (1 hour)
- **Goal**: Components talk to each other
- **What you'll learn**: Message passing, shared memory
- **Code**: ~200 lines
- **Status**: 📝 Planned (Chapter 5)

### Level 4: Simple Driver (2 hours)
- **Goal**: Serial port or timer driver
- **What you'll learn**: MMIO, interrupts, device management
- **Code**: ~300 lines
- **Status**: 📝 Planned (Chapter 8)

### Level 5: Storage & Filesystem (4 hours)
- **Goal**: Read/write files from disk
- **What you'll learn**: VirtIO, block devices, filesystem basics
- **Code**: ~500 lines
- **Status**: 📝 Planned (Chapter 10)

### Level 6: Network Stack (6 hours)
- **Goal**: Send/receive network packets
- **What you'll learn**: VirtIO-net, TCP/IP basics
- **Code**: ~800 lines
- **Status**: 📝 Planned (Chapter 11)

## Development Workflow

### Daily Development

```bash
# Edit kernel code
cd kernel
vim src/my_feature.rs

# Build and test
./build.sh --platform qemu-virt

# Run in QEMU
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader

# Exit with Ctrl-A then X
```

### Debugging

```bash
# Terminal 1: Start QEMU with GDB stub
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader \
  -s -S

# Terminal 2: Connect debugger
lldb runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
(lldb) gdb-remote localhost:1234
(lldb) breakpoint set --name _start
(lldb) continue
```

### Clean Builds

```bash
# Clean everything
./build.sh --clean

# Or just kernel
cd kernel && cargo clean
```

## What Can You Build with KaaL?

KaaL provides a capability-based microkernel foundation. Here's what's available:

### Available Features

**Process Management:**

- Spawn multiple userspace processes
- Capability-based memory allocation (UntypedMemory)
- Preemptive multitasking with configurable priorities
- Cooperative yielding between processes

**Inter-Process Communication:**

- Shared memory channels between processes
- Async notifications for event signaling
- Type-safe message passing via the SDK

**Device Interaction:**

- UART serial I/O with interrupt support
- Timer-based preemption (configurable timeslice)
- Device driver framework via SDK components

**Memory Management:**

- Virtual address spaces (4-level page tables)
- Physical frame allocation
- Memory mapping with permission control

### Example Systems You Can Build

**1. Message Passing Services** (Level: Beginner)

- Producer/consumer patterns
- Event-driven architectures
- Simple IPC demonstrations

**2. Device Drivers** (Level: Intermediate)

- Serial port drivers (UART example included)
- Timer drivers
- GPIO control (on supported hardware)

**3. Interactive Applications** (Level: Intermediate)

- Text editors (notepad example included)
- Command shells
- System monitoring tools

**4. Custom Microkernels** (Level: Advanced)

- Add your own syscalls
- Implement custom scheduling policies
- Build specialized capability brokers

## Learning Resources

### Documentation

- **[GETTING_STARTED.md](GETTING_STARTED.md)** - Setup and first build
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and structure
- **[MICROKERNEL_CHAPTERS.md](MICROKERNEL_CHAPTERS.md)** - Development roadmap
- **[PROJECT_STATUS.md](PROJECT_STATUS.md)** - Current implementation status

### Code Examples

1. **Kernel Entry** (`kernel/src/main.rs`)
   - Boot sequence
   - Initialization flow
   - Chapter progression

2. **Root Task** (`runtime/root-task/src/main.rs`)
   - First userspace program
   - Syscall examples
   - Component template

3. **Elfloader** (`runtime/elfloader/src/boot.rs`)
   - Bootloader implementation
   - Binary loading
   - Address space setup

### External Resources

- **ARM64 Architecture**: ARM Architecture Reference Manual
- **Microkernel Design**: "The seL4 Whitepaper" (for concepts, not code)
- **Rust Embedded**: The Embedded Rust Book
- **OS Development**: OSDev Wiki

## Common Questions

### "Why ARM64 instead of x86?"

- **Simpler**: ARM64 has cleaner exception handling and memory management
- **Modern**: Most new hardware is ARM (phones, tablets, servers, laptops)
- **Native Development**: If you have Apple Silicon, you're already on ARM64!
- **Future-proof**: ARM is the future of computing

### "Can I run this on real hardware?"

Yes! KaaL is designed for real ARM64 hardware:
- Raspberry Pi 4 (work in progress)
- Other ARM64 boards (planned)
- Currently, QEMU is the easiest way to develop

### "Do I need to know seL4?"

No! KaaL is a **clean-room implementation** - not based on seL4 code.

While KaaL takes inspiration from seL4's security model (capabilities, isolation), you don't need to know anything about seL4 to use KaaL.

### "Is this a toy or production-ready?"

Currently: **Learning/hobby project** (Chapter 7 of 12)

Future goal: **Production-capable** microkernel for embedded and IoT

Right now, it's perfect for:
- Learning OS development
- Experimenting with microkernels
- Understanding ARM64 architecture
- Building hobby operating systems

### "How can I contribute?"

1. **Try it out**: Build and run the kernel
2. **Report issues**: Found a bug? Open an issue!
3. **Write docs**: Help improve guides and documentation
4. **Implement features**: Pick a chapter and start coding
5. **Share feedback**: What's confusing? What's great?

## Troubleshooting

### Build Errors

**"aarch64-unknown-none target not found"**
```bash
rustup target add aarch64-unknown-none
```

**"nightly required"**
```bash
rustup default nightly
```

**"linker error"**
```bash
# Make sure you're in the kaal directory
./build.sh --clean
./build.sh --platform qemu-virt
```

### Runtime Issues

**QEMU hangs with no output**
- Make sure you're using `-nographic` flag
- Try adding `-serial mon:stdio` for more debug output
- Press `Ctrl-A` then `X` to force exit

**"No output after kernel loads"**
- Check that root task binary was built
- Look for error messages in elfloader output
- Try running with `-d int,guest_errors` for QEMU debug logs

### Getting Help

- **Check docs**: Most questions are answered in documentation
- **GitHub Issues**: Report bugs or ask questions
- **Read code**: The kernel is small enough to understand fully
- **Experiment**: Try changing things and see what happens!

## Next Steps

1. ✅ **You've built and run KaaL** - Great start!
2. 📖 **Read [ARCHITECTURE.md](ARCHITECTURE.md)** - Understand the design
3. 🔍 **Explore the code** - Start with `kernel/src/main.rs`
4. 🚀 **Implement a feature** - Pick a chapter from [MICROKERNEL_CHAPTERS.md](MICROKERNEL_CHAPTERS.md)
5. 🤝 **Join development** - Contribute to the project!

## Philosophy

**Start minimal. Build incrementally. Learn deeply.**

You don't need:
- Complex toolchains
- Gigabytes of dependencies
- Years of experience
- A CS degree

You just need:
- Curiosity about how computers work
- Willingness to read and experiment
- Rust and QEMU installed
- This guide

**Welcome to OS development!** 🚀

---

**Happy Hacking!**

*"From bare metal to userspace - one chapter at a time"*
