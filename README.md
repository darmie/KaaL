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

### Building a Bootable Image

KaaL uses a config-driven build system to create bootable images for ARM64 platforms:

```bash
# Build bootable image for QEMU virt (default)
./build.sh

# Build for Raspberry Pi 4
./build.sh --platform rpi4

# Build for custom platform
./build.sh --platform my-board

# Test in QEMU
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
```

The build system packages your kernel, user-space components, and drivers into a single bootable ELF image. Configure platforms in [build-config.toml](build-config.toml).

---

## 📦 What KaaL Provides

### Core Framework (The Skeleton)
- **`kaal-kernel`**: Native Rust microkernel (capability-based)
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
- **Build system**: Config-driven multi-platform build
- **Platform support**: QEMU, Raspberry Pi, custom boards

**Pick what you need. Leave what you don't.**

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
- [runtime/elfloader/README.md](runtime/elfloader/README.md) - Bootloader documentation
- [build-config.toml](build-config.toml) - Platform configuration reference

---

## 📝 License

MIT License - see [LICENSE](LICENSE) for details.

Copyright (c) 2025 Damilare Darmie Akinlaja

---

**KaaL**: The framework that gets out of your way. Build the OS you need, not the one someone else designed.
