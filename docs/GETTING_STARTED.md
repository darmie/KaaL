# Getting Started with KaaL Framework

This guide will help you get started with KaaL Framework development, from setting up your environment to building and running your first kernel.

---

## Prerequisites

### Required Tools

1. **Rust Toolchain** (1.75+ with nightly)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup default nightly

   # Add ARM64 bare-metal target
   rustup target add aarch64-unknown-none
   ```

2. **QEMU** (for testing)
   ```bash
   # macOS
   brew install qemu

   # Linux (Ubuntu/Debian)
   sudo apt install qemu-system-aarch64

   # Verify installation
   qemu-system-aarch64 --version
   ```

3. **Basic Development Tools**
   ```bash
   # macOS
   brew install git cmake

   # Linux
   sudo apt install build-essential git cmake
   ```

### Verification

Verify your setup:

```bash
# Check Rust (should show nightly)
rustc --version

# Check QEMU
qemu-system-aarch64 --version

# Check targets
rustup target list --installed | grep aarch64-unknown-none
```

---

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/darmie/kaal.git
cd kaal
```

### 2. Build the Kernel

```bash
# Build bootable image for QEMU virt platform
./build.sh --platform qemu-virt

# This builds:
# - KaaL native Rust microkernel
# - Root task (userspace program)
# - Elfloader (bootloader)
# - Packages everything into a single bootable ELF image
```

### 3. Run in QEMU

```bash
# Run the built kernel
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader

# You should see:
# - Elfloader boot messages
# - KaaL kernel initialization (Chapters 1-3)
# - EL0 transition
# - Beautiful KaaL ASCII art banner
# - "Hello from userspace" message
```

### 4. Exit QEMU

Press `Ctrl-A` then `X` to exit QEMU.

---

## Project Structure

```
kaal/
├── README.md                  # Project overview
├── LICENSE                    # MIT License
├── build.sh                   # Build system entry point
├── build-config.toml          # Platform configurations
├── Cargo.toml                 # Workspace configuration
│
├── kernel/                    # Native Rust microkernel
│   ├── src/
│   │   ├── main.rs           # Kernel entry point
│   │   ├── boot/             # Boot & initialization
│   │   ├── arch/aarch64/     # ARM64 architecture code
│   │   ├── memory/           # Memory management
│   │   ├── syscall/          # System call interface
│   │   └── objects/          # Kernel objects (TCB, endpoints, etc.)
│   └── Cargo.toml
│
├── runtime/
│   ├── elfloader/            # Bootloader
│   ├── root-task/            # First userspace program
│   └── dummy-roottask/       # Minimal root task
│
├── docs/                      # Documentation
│   ├── ARCHITECTURE.md        # System architecture
│   ├── MICROKERNEL_CHAPTERS.md  # Development roadmap
│   └── PROJECT_STATUS.md      # Current status
│
└── examples/                  # Example kernels and programs
```

---

## Development Workflow

### Building for Different Platforms

```bash
# QEMU virt (default - for development)
./build.sh --platform qemu-virt

# Raspberry Pi 4 (work in progress)
./build.sh --platform rpi4

# Custom platform (configure in build-config.toml)
./build.sh --platform my-board
```

### Working on the Kernel

```bash
cd kernel

# Build kernel only
./build-kernel.sh

# Run tests (when available)
cargo test

# Check code
cargo clippy -- -D warnings
cargo fmt --check
```

### Debugging with QEMU

```bash
# Terminal 1: Start QEMU with GDB stub
qemu-system-aarch64 \
  -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader \
  -s -S

# Terminal 2: Connect with GDB/LLDB
lldb runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
(lldb) gdb-remote localhost:1234
(lldb) breakpoint set --name _start
(lldb) continue
```

---

## Understanding the Boot Process

1. **Elfloader** (runtime/elfloader)
   - Loads at address 0x40000000
   - Parses device tree
   - Loads kernel and root task binaries
   - Sets up initial page tables
   - Jumps to kernel entry point

2. **Kernel Initialization** (kernel/src/boot/mod.rs)
   - **Chapter 1**: Bare metal boot & early init
   - **Chapter 2**: Memory management (frame allocator, MMU setup)
   - **Chapter 3**: Exception handling & syscalls
   - **Chapter 7**: Root task creation & EL0 transition

3. **Root Task** (runtime/root-task)
   - First userspace program
   - Runs at EL0 (unprivileged mode)
   - Demonstrates syscalls
   - Prints KaaL ASCII art banner

---

## Current Status & Roadmap

**Current Status**: Chapter 7 Complete - Full userspace execution working! ✅

The kernel is developed chapter-by-chapter. Completed chapters:

- ✅ **Chapter 1**: Bare Metal Boot & Early Init
- ✅ **Chapter 2**: Memory Management (frame allocator, MMU, page tables)
- ✅ **Chapter 3**: Exception Handling & Syscalls (TrapFrame, context switching)
- ✅ **Chapter 7**: Root Task & Boot Protocol (EL0 transition, userspace execution)

**Next chapters to implement:**

- 📝 **Chapter 4**: Thread Control Blocks (TCBs) - process management
- 📝 **Chapter 5**: IPC & Endpoints - inter-process communication
- 📝 **Chapter 6**: Capability Management - security & access control
- 📝 **Chapters 8-12**: Scheduling, interrupts, device drivers, and more

See [docs/MICROKERNEL_CHAPTERS.md](MICROKERNEL_CHAPTERS.md) for the complete development plan.

---

## Platform-Specific Notes

### macOS (Apple Silicon)

- ✅ Native ARM64 development
- ✅ QEMU ARM64 runs near-native speed
- ✅ LLDB debugging works natively
- ✅ No cross-compilation needed for ARM64 targets

### Linux

- ✅ Works on both x86_64 and ARM64 hosts
- ✅ Full QEMU support
- ✅ GDB debugging available

---

## Common Tasks

### View Boot Messages

```bash
# Run kernel and see all boot output
./build.sh --platform qemu-virt && \
  qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
```

### Clean Build

```bash
# Clean all build artifacts
./build.sh --clean

# Or manually
cargo clean
cd runtime/elfloader && cargo clean
cd runtime/root-task && cargo clean
```

### Modify Root Task

Edit `runtime/root-task/src/main.rs` to change what runs in userspace:

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        sys_print("Hello from my custom userspace!\n");
    }
    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}
```

Then rebuild: `./build.sh --platform qemu-virt`

---

## Troubleshooting

### "aarch64-unknown-none target not found"

```bash
rustup target add aarch64-unknown-none
```

### "qemu-system-aarch64 not found"

```bash
# macOS
brew install qemu

# Linux
sudo apt install qemu-system-aarch64
```

### Build fails with "linker error"

Make sure you're using nightly Rust:
```bash
rustup default nightly
```

### QEMU hangs or doesn't show output

- Make sure you're using `-nographic` flag
- Press `Ctrl-A` then `X` to exit QEMU
- Check that kernel binary exists at expected path

---

## Getting Help

- **Documentation**: [docs/](.)
- **Issues**: [GitHub Issues](https://github.com/darmie/kaal/issues)
- **Architecture**: [docs/ARCHITECTURE.md](ARCHITECTURE.md)
- **Status**: [docs/PROJECT_STATUS.md](PROJECT_STATUS.md)

---

## Next Steps

1. ✅ **Set up your environment** (you're here!)
2. 📖 **Read** [ARCHITECTURE.md](ARCHITECTURE.md) to understand the design
3. 📖 **Read** [MICROKERNEL_CHAPTERS.md](MICROKERNEL_CHAPTERS.md) for the roadmap
4. 🔨 **Build and run** the kernel
5. 🚀 **Start contributing** - pick a chapter and implement!

Welcome to KaaL Framework development! 🚀
