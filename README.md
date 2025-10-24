```
$$╲   $$╲                    $$╲
$$ │ $$  │                   $$ │
$$ │$$  ╱ $$$$$$╲   $$$$$$╲  $$ │
$$$$$  ╱  ╲____$$╲  ╲____$$╲ $$ │
$$  $$<   $$$$$$$ │ $$$$$$$ │$$ │
$$ │╲$$╲ $$  __$$ │$$  __$$ │$$ │
$$ │ ╲$$╲╲$$$$$$$ │╲$$$$$$$ │$$$$$$$$╲
╲__│  ╲__│╲_______│ ╲_______│╲________│
```

# KaaL Framework

**A composable OS development framework with a native Rust microkernel**

[![QEMU Build](.github/badges/qemu-build.svg)](https://github.com/darmie/KaaL/releases)
[![Verification](.github/badges/verification.svg)](docs/VERIFICATION_COVERAGE.md)

> KaaL is the skeleton, not the OS. Build your own capability-based operating system using composable components.

## 🎯 What is KaaL?

KaaL is a **framework for composable operating system development**. It provides:

- **Native Rust Microkernel**: Capability-based kernel built from scratch in Rust
- **Composable Components**: Mix and match VFS, network, POSIX layers
- **Capability-Based Architecture**: Security by design
- **Multi-Platform**: ARM64 support (QEMU, Raspberry Pi, custom boards)

Think of KaaL as the **skeleton** upon which you build your custom OS for embedded, IoT, or security-critical systems.

---

## 🏗️ Architecture Philosophy

```
Your Custom OS (you build this)
    ↓
┌─────────────────────────────────────┐
│  KaaL Framework (the skeleton)       │
│  ┌─────────────────────────────────┐│
│  │ Composable Components (optional) ││
│  │  VFS │ Network │ POSIX │ Drivers ││
│  ├─────────────────────────────────┤│
│  │ Capability Broker (your policies)││
│  ├─────────────────────────────────┤│
│  │ IPC Layer (message passing)      ││
│  ├─────────────────────────────────┤│
│  │ Kernel Abstraction (pluggable)   ││
│  │  ├── KaaL Microkernel (Rust)     ││
│  │  ├── Mock (development)          ││
│  │  └── Other kernels (future)      ││
│  └─────────────────────────────────┘│
└─────────────────────────────────────┘
    ↓
  KaaL Rust Microkernel (capability-based, ARM64)
```

**You decide**: Which components? Which policies? Which deployment?

---

## 🚀 Quick Start

### Prerequisites

Before building KaaL, ensure you have the required dependencies installed:

```bash
# Run the automated setup script
nu setup.nu

# Or verify existing installation
nu setup.nu --verify-only
```

The setup script will install:

- **Rust nightly** toolchain with `aarch64-unknown-none` target
- **QEMU** (qemu-system-aarch64) for ARM64 emulation
- **Device Tree Compiler** (dtc) for device tree manipulation
- **LLVM** tools (llvm-objcopy) for object file conversion
- **Nushell** (if not already installed)

**Supported Platforms:**

- macOS (via Homebrew)
- Linux (Debian/Ubuntu, Fedora/RHEL, Arch)
- Windows (WSL2 recommended)

**Manual Installation:**

If you prefer manual setup or the script doesn't work for your platform:

**macOS:**

```bash
brew install qemu dtc llvm
rustup toolchain install nightly
rustup default nightly
rustup target add aarch64-unknown-none
```

**Linux (Debian/Ubuntu):**

```bash
sudo apt-get update
sudo apt-get install -y qemu-system-aarch64 device-tree-compiler llvm
rustup toolchain install nightly
rustup default nightly
rustup target add aarch64-unknown-none
```

**Linux (Fedora/RHEL):**

```bash
sudo dnf install -y qemu-system-aarch64 dtc llvm
rustup toolchain install nightly
rustup default nightly
rustup target add aarch64-unknown-none
```

**Linux (Arch):**

```bash
sudo pacman -S qemu-system-aarch64 dtc llvm
rustup toolchain install nightly
rustup default nightly
rustup target add aarch64-unknown-none
```

### Building and Running

KaaL uses a Nushell-based build system to create bootable images for any configured platform:

```bash
# Build bootable image for QEMU virt (default)
nu build.nu

# Build and run in QEMU
nu build.nu --run

# Or run manually
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
```

The build system:
1. Discovers components from `components.toml`
2. Generates platform-specific configurations from `build-config.toml`
3. Builds components (excluding system_init)
4. Generates component registry for system_init
5. Builds system_init (with embedded component binaries)
6. Packages kernel + root-task into bootable ELF image

### Try the Demo Apps

KaaL includes fully functional TUI (Terminal User Interface) applications:

**📝 Notepad** - A line-based text editor with screen clearing:
```bash
# notepad is enabled by default
nu build.nu
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
```

**✅ Todo App** - A vi-style task manager with rich TUI:
```bash
# Enable todo-app in components.toml (set autostart = true)
nu build.nu
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
```

**Todo App Features:**
- Vi-style navigation (j/k keys)
- Add/delete/toggle todos
- Visual checkboxes and highlighting
- Full-screen TUI with box drawing
- Color-coded status bar

Both apps demonstrate:
- **kaal-tui** library: ANSI escape sequences for colors, cursor control, and drawing
- **Keyboard input**: IRQ-driven UART with proper EOI handling
- **IPC**: Cross-CSpace notification sharing between UART driver and apps
- **Screen clearing**: Clean startup without kernel boot messages

Configure platforms in [build-config.toml](build-config.toml).

---

## 📦 What KaaL Provides

### Core Kernel

- **Microkernel**: seL4-style capability-based security model
- **Memory Management**: Physical allocator, virtual memory, page tables
- **Process Isolation**: Separate address spaces (VSpace) and capability spaces (CSpace)
- **IPC**: Shared memory + notifications for inter-component communication
- **Scheduling**: Priority-based preemptive scheduler
- **Exception Handling**: EL1 exception vectors, syscall interface

### Runtime

- **Root-Task**: Minimal bootstrap runtime, initializes kernel objects
- **Elfloader**: Multi-stage bootloader, loads kernel + root-task
- **Component Spawning**: Userspace ELF loading without kernel bloat

### SDK

- **kaal-sdk**: Syscall wrappers, component patterns, spawning helpers
- **kaal-tui**: Terminal UI library with ANSI escape sequences (colors, cursor control, box drawing)
- **Capability API**: Allocate, transfer, manage capabilities
- **Memory API**: Allocate, map, unmap physical memory
- **IPC API**: Shared memory channels, notifications, typed messaging
- **Component Spawning**: `spawn_from_elf()` - no new syscalls needed!

### Build System

- **Nushell-based**: Type-safe, modern build orchestration
- **Component Discovery**: Auto-discovery from `components.toml`
- **Registry Generation**: Automatic component registry for boot orchestration
- **Multi-Platform**: QEMU virt, Raspberry Pi 4, custom boards
- **Code Generation**: Linker scripts, platform configs, component registries

### Formal Verification

![Verification](.github/badges/verification.svg)

- **Verus**: Mathematical verification of critical kernel components
- **Verified Modules**: 22 modules, 428 items, 0 errors
  - Memory operations (PhysAddr, VirtAddr, PageFrameNumber)
  - Capability system (CapRights, capability derivation, rights checking, transfer operations)
  - Capability transfer (rights diminishing, badge minting, GRANT validation, CSpace isolation)
  - CNode operations (slot management, power-of-2 proofs)
  - Page table operations (ARMv8-A 4-level page tables, shift/index calculations)
  - Page table entries (PTE descriptor types, address extraction, permission bits)
  - IPC operations (thread queues, endpoint state, message transfer, FIFO properties)
  - IPC message transfer (message info encoding/decoding, MR0-MR7 registers, buffer copying)
  - Scheduler operations (priority bitmap, O(1) priority lookup with leading_zeros)
  - Syscall invocation (argument validation, rights checking, label parsing)
  - Frame allocator (alloc/dealloc, free count tracking, bounds safety)
  - Untyped memory (watermark allocator, child tracking, revocation safety)
  - VSpace operations (page table walking L0-L3, map/unmap 4KB/2MB/1GB pages)
  - TLB management (invalidate by VA/ASID/all, ASID allocation, context switch)
  - Exception handling (ESR parsing, EL transitions, fault address capture, vector selection)
  - Interrupt handling (IRQ validation, GICv2/v3 operations, priority levels, acknowledge/EOI)
  - Production bitmap (frame conditions, loop invariants)
  - Thread Control Block (TCB state machine, capability checking)
- **Advanced Features**: State machine verification, bit-level axioms, stateful specs with `old()`, termination proofs, power-of-2 arithmetic, FIFO queue properties, priority-based scheduling, error propagation
- **Zero Runtime Overhead**: All proofs erased during compilation
- **Production Code**: Verifying actual implementation, not simplified examples

Run verification: `nu scripts/verify.nu`

**Documentation:**
- [Advanced Verification Techniques](docs/ADVANCED_VERIFICATION.md) - Detailed guide to advanced Verus features
- [Verification Coverage Report](docs/VERIFICATION_COVERAGE.md) - Complete coverage analysis and remaining work

---

## 🎯 Capability-Based Process Spawning

KaaL implements **seL4-style capability-based process spawning**, where all resource allocation goes through capabilities rather than direct kernel calls:

### Resource Delegation Model

```text
Kernel (EL1)
  └─ UntypedMemory Capabilities (untyped physical RAM)
      └─ root-task (EL0) receives parent Untyped
          └─ Creates child Untyped via sys_retype()
              └─ Delegates to system_init
                  └─ Spawns applications using spawn_from_elf_with_untyped()
```

**Key Features:**

- **sys_retype()**: Retype untyped memory into kernel objects (TCB, Endpoint, Page, etc.)
- **Watermark Allocator**: Efficient sequential allocation from untyped regions
- **Userspace Spawning**: No kernel involvement after initial setup - spawning happens entirely in userspace
- **Fine-Grained Control**: Parent processes control exactly what resources children receive

### Why This Matters

Traditional microkernels use direct kernel syscalls for process creation (`fork()`, `exec()`). KaaL follows seL4's approach:

- **Security**: Parent explicitly delegates resources - no implicit allocation
- **Accountability**: Every byte of RAM is tracked through capability hierarchy
- **Flexibility**: Userspace can implement custom spawning policies
- **No Kernel Bloat**: ELF loading and memory allocation logic stays in userspace

---

## 💡 Example: Building a Custom Component

Here's how you'd build a custom service using KaaL's composable APIs:

```rust
// components/my-service/src/main.rs
#![no_std]
#![no_main]

use kaal_sdk::{
    component::Component,
    capability::Notification,
    syscall,
};

// Declare component (generates metadata, entry point, panic handler)
kaal_sdk::component! {
    name: "my_service",
    type: Service,
    version: "0.1.0",
    capabilities: ["notification:wait", "ipc:my_channel"],
    impl: MyService
}

pub struct MyService {
    notification: Notification,
}

impl Component for MyService {
    fn init() -> kaal_sdk::Result<Self> {
        syscall::print("[my_service] Initializing...\n");

        // Create notification for event handling
        let notification = Notification::create()?;

        Ok(Self { notification })
    }

    fn run(&mut self) -> ! {
        syscall::print("[my_service] Running event loop\n");

        loop {
            // Wait for notification
            match self.notification.wait() {
                Ok(signals) => {
                    syscall::print("[my_service] Received event\n");
                    // Process event...
                }
                Err(_) => {
                    syscall::print("[my_service] Error\n");
                }
            }
        }
    }
}
```

Add to `components.toml`:

```toml
[component.my_service]
name = "my_service"
binary = "my-service"
type = "service"
priority = 100
autostart = true
capabilities = ["ipc:*", "memory:allocate"]
```

Build and run:

```bash
nu build.nu --run
```

Your component will be discovered, built, added to the registry, and spawned automatically by system_init!

---

## 🎨 Design Principles

1. **Composability**: Mix and match components
2. **Security by Default**: Capabilities, not ACLs
3. **Native Rust**: Type safety and memory safety throughout
4. **Multi-Platform**: Easy to port to new ARM64 boards
5. **Explicit Everything**: No magic, no implicit state

---

## 📊 Platform Support

| Platform | Status | CPU | Memory | Boot Method |
|----------|--------|-----|--------|-------------|
| **QEMU virt** | ✅ Working | Cortex-A53 | 128MB | ELF image |
| **Raspberry Pi 4** | 🚧 In Progress | Cortex-A72 | 1GB | SD card / TFTP |
| **Custom ARM64** | 📝 Template | Configurable | Configurable | Platform-specific |

Add new platforms by configuring [build-config.toml](build-config.toml).

---

## 💡 Philosophy

**KaaL is NOT:**
- ❌ A complete operating system
- ❌ A general-purpose OS
- ❌ Opinionated about your use case

**KaaL IS:**
- ✅ A skeleton for building custom OS
- ✅ A collection of composable primitives
- ✅ A kernel abstraction layer
- ✅ A security-first foundation

**You bring the vision. KaaL provides the foundation.**

---

## 📚 Documentation

- [BUILD_SYSTEM.md](BUILD_SYSTEM.md) - Config-driven build system guide
- [kernel/README.md](kernel/README.md) - KaaL microkernel documentation
- [kernel/src/verified/mod.rs](kernel/src/verified/mod.rs) - Formal verification status
- [runtime/elfloader/README.md](runtime/elfloader/README.md) - Bootloader documentation
- [build-config.toml](build-config.toml) - Platform configuration reference

---

## 📝 License

MIT License - see [LICENSE](LICENSE) for details.

Copyright (c) 2025 Damilare Darmie Akinlaja

---

**KaaL**: The framework that gets out of your way. Build the OS you need, not the one someone else designed.
