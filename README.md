# KaaL - Kernel-as-a-Library

**A composable OS development framework built on seL4**

> KaaL is the skeleton, not the OS. Build your own capability-based operating system using composable components.

## 🎯 What is KaaL?

KaaL is a **framework for composable operating system development**. It provides:

- **Kernel-as-a-Library**: seL4 integration as a pluggable backend
- **Composable Components**: Mix and match VFS, network, POSIX layers
- **Capability-Based Architecture**: Security by design
- **Production-First**: Defaults to real seL4, not mocks

Think of KaaL as the **skeleton** upon which you build your custom OS for embedded, IoT, or security-critical systems.

**Mac Silicon Users**: See [QUICKSTART.md](QUICKSTART.md) for 3-command setup with Docker!

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
│  │  ├── seL4 Microkit (default)     ││
│  │  ├── seL4 Runtime (advanced)     ││
│  │  └── Mock (development)          ││
│  └─────────────────────────────────┘│
└─────────────────────────────────────┘
    ↓
  seL4 Microkernel (or future: other kernels)
```

**You decide**: Which components? Which policies? Which deployment?

---

## 🚀 Quick Start

### Build Your OS for Any Platform

**Your OS runs on**: ARM64, x86_64, RISC-V, or any seL4-supported architecture
**Build environment**: Currently requires Linux (or Docker) due to seL4 SDK tooling

```bash
# Build real OS for ARM64 (runs on ARM hardware/QEMU)
export SEL4_PREFIX=/path/to/seL4
cargo build --features board-qemu-virt-aarch64

# Build real OS for x86_64 (runs on PC/QEMU)
cargo build --features board-pc99

# Framework development (any platform - macOS, Linux, Windows)
cargo build --no-default-features --features mock
```

**Important**: The OS you build runs everywhere - the build environment limitation is temporary.

---

## 📦 What KaaL Provides

### Core Framework (The Skeleton)
- **`sel4-platform`**: Kernel abstraction layer (pluggable)
- **`cap-broker`**: Capability management (bring your policies)
- **`kaal-ipc`**: Typed message passing
- **`kaal-allocator`**: Memory primitives
- **`dddk`**: Driver development kit

### Optional Components (Composable)
- **`kaal-vfs`**: Virtual filesystem (optional)
- **`kaal-network`**: Network stack (optional)  
- **`kaal-posix`**: POSIX layer (optional)
- **`kaal-drivers`**: Hardware drivers (optional)

### Tools
- **`sel4-compose`**: Declarative OS builder *(planned)*

**Pick what you need. Leave what you don't.**

---

## 🎨 Design Principles

1. **Composability**: Mix and match components
2. **Security by Default**: Capabilities, not ACLs
3. **Kernel as Library**: Pluggable backends
4. **Production-First**: Real kernel integration (seL4)
5. **Explicit Everything**: No magic, no implicit state

---

## 📊 Build Modes

| Mode | Default | Build Host | Output OS Runs On | Use Case |
|------|---------|------------|-------------------|----------|
| **Microkit** | ✅ YES | Linux* | ARM64, x86, RISC-V | Build production OS |
| **Runtime** | ❌ NO | Linux* | ARM64, x86, RISC-V | Advanced seL4 OS |
| **Mock** | ❌ NO | Any | N/A (testing only) | Framework dev |

*Build host limitation is temporary - the OS you build runs on any seL4 target

```bash
# Build OS for ARM64 (build on Linux, runs on ARM hardware)
cargo build --features board-qemu-virt-aarch64

# Build OS for x86 (build on Linux, runs on PC)
cargo build --features board-pc99

# Framework development (any build platform)
cargo build --no-default-features --features mock
```

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

- [docs/BUILD_INSTRUCTIONS.md](docs/BUILD_INSTRUCTIONS.md) - Complete build guide
- [docs/CROSS_PLATFORM.md](docs/CROSS_PLATFORM.md) - Build for any architecture
- [docs/BUILD_MODES.md](docs/BUILD_MODES.md) - Build mode details
- [docs/INTEGRATION_SUMMARY.md](docs/INTEGRATION_SUMMARY.md) - Integration summary
- [scripts/README.md](scripts/README.md) - Build scripts documentation

---

## 📝 License

MIT OR Apache-2.0

---

**KaaL**: The framework that gets out of your way. Build the OS you need, not the one someone else designed.
