# KaaL Elfloader System - Complete Guide

This guide explains the complete elfloader system for creating bootable seL4 images with KaaL.

## Overview

The KaaL elfloader system consists of **two separate crates**:

1. **`runtime/elfloader/`** - Bootloader (runs on target ARM64 hardware)
2. **`runtime/elfloader-builder/`** - Build tool (runs on host during compilation)

## Why Two Crates?

Think of it like a compiler toolchain:
- **Elfloader** = Runtime library (like libc) - runs on target
- **Elfloader-builder** = Compiler (like rustc) - runs on host

This separation allows:
- Clean build process
- Independent versioning
- Different dependencies (host vs target)
- Easier testing

## Complete Workflow

```
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 1: Build Components                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1a. Build Elfloader (Rust, bare-metal)                         │
│      cd runtime/elfloader                                       │
│      cargo build --release --target aarch64-unknown-none        │
│      → libkaal_elfloader.a                                      │
│                                                                  │
│  1b. Build seL4 Kernel (C, separate build)                      │
│      cd kernel && mkdir build && cd build                       │
│      cmake .. -G Ninja -DPLATFORM=qemu-arm-virt                 │
│      ninja kernel.elf                                           │
│      → kernel.elf                                               │
│                                                                  │
│  1c. Build Root Task (Rust)                                     │
│      cargo build --release --bin root-task                      │
│      → root-task                                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 2: Package with Builder                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  2. Run Elfloader Builder                                       │
│     kaal-elfloader-builder \                                    │
│       --loader libkaal_elfloader.a \                            │
│       --kernel kernel.elf \                                     │
│       --app root-task \                                         │
│       --out bootimage.elf                                       │
│                                                                  │
│     This tool:                                                  │
│     • Parses kernel.elf and root-task ELF                       │
│     • Extracts loadable segments                                │
│     • Calculates physical addresses                             │
│     • Serializes metadata + data with postcard                  │
│     • Patches elfloader binary                                  │
│                                                                  │
│     → bootimage.elf (FINAL BOOTABLE IMAGE)                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 3: Boot on Hardware                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  3. Run in QEMU or Hardware                                     │
│     qemu-system-aarch64 \                                       │
│       -machine virt \                                           │
│       -cpu cortex-a53 \                                         │
│       -nographic \                                              │
│       -kernel bootimage.elf                                     │
│                                                                  │
│     Boot sequence:                                              │
│     1. Firmware loads bootimage.elf, sets x0 = DTB address      │
│     2. Elfloader _start runs (assembly)                         │
│     3. elfloader_main (Rust):                                   │
│        - Parse DTB                                              │
│        - Deserialize embedded payload                           │
│        - Load kernel to 0x40000000                              │
│        - Load root task to calculated address                   │
│        - Setup page tables                                      │
│        - Enable MMU                                             │
│        - Jump to kernel                                         │
│     4. seL4 kernel boots                                        │
│     5. KaaL root task starts                                    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Directory Structure

```
runtime/
├── elfloader/                    # Bootloader (target ARM64)
│   ├── Cargo.toml
│   ├── linker.ld                 # Memory layout (loads at 0x10000000)
│   ├── .cargo/config.toml        # Target: aarch64-unknown-none
│   └── src/
│       ├── lib.rs                # Main entry, kernel handoff
│       ├── arch/aarch64.rs       # ARM64 _start, MMU, barriers
│       ├── boot.rs               # Payload deserialization, loading
│       ├── mmu.rs                # Page table management
│       ├── payload.rs            # Payload structures (shared)
│       ├── uart.rs               # PL011 debug output
│       └── utils.rs              # Alignment helpers
│
└── elfloader-builder/            # Build tool (host)
    ├── Cargo.toml
    └── src/
        ├── main.rs               # CLI tool
        ├── payload.rs            # Payload structures (shared)
        └── elf_loader.rs         # ELF parsing, serialization
```

## Key Concepts

### 1. Payload Serialization

The builder creates a compact binary payload:

```
[ Postcard Metadata ] [ Kernel Data ] [ User Data ]
     ~256 bytes         Variable        Variable
```

**Metadata** (serialized with postcard):
```rust
struct Payload {
    kernel_regions: Vec<Region>,  // Where to load kernel
    kernel_entry: usize,          // Kernel entry point
    user_regions: Vec<Region>,    // Where to load root task
    user_entry: usize,            // Root task entry
    total_data_size: usize,
}
```

### 2. Memory Layout

```
Physical Memory (ARM64):
┌──────────────────┐ 0x00000000
│ Device MMIO      │ UART, peripherals
├──────────────────┤ 0x10000000
│ Elfloader        │ ← Bootloader code + payload
│  .text.boot      │   Entry point
│  .text           │   Rust code
│  .rodata.payload │   Embedded payload (added by builder)
│  .data           │   PAYLOAD_START, PAYLOAD_SIZE
│  .bss            │
│  Stack           │   1MB
├──────────────────┤ ~0x11100000
│ Page Tables      │ L1, L2, L3 (dynamically allocated)
├──────────────────┤ 0x40000000 (default)
│ seL4 Kernel      │ ← Loaded here by elfloader
├──────────────────┤ Kernel end + 2MB
│ Root Task        │ ← Loaded here by elfloader
├──────────────────┤
│ Free Memory      │ → Given to seL4
└──────────────────┘
```

### 3. Build Flow

```
Source Code → Compilation → Packaging → Boot
─────────────────────────────────────────────
elfloader.rs ───→ cargo ───┐
kernel.c     ───→ ninja ───┤
root_task.rs ───→ cargo ───┼──→ elfloader-builder ──→ bootimage.elf
                            │
                            └──→ Combines all three
```

## Quick Start

### Step 1: Build Elfloader Builder Tool

```bash
cd runtime/elfloader-builder
cargo install --path .
```

### Step 2: Build All Components

```bash
# Build elfloader
cd runtime/elfloader
cargo build --release --target aarch64-unknown-none \
  -Z build-std=core,alloc

# Build kernel (example)
cd ../../kernel
mkdir -p build && cd build
cmake .. -G Ninja -DPLATFORM=qemu-arm-virt -DAARCH64=1
ninja kernel.elf

# Build root task (example)
cd ../..
cargo build --release --bin my-root-task --target aarch64-unknown-sel4
```

### Step 3: Package into Bootable Image

```bash
kaal-elfloader-builder \
  --loader runtime/elfloader/target/aarch64-unknown-none/release/libkaal_elfloader.a \
  --kernel kernel/build/kernel.elf \
  --app target/aarch64-unknown-sel4/release/my-root-task \
  --out bootimage.elf
```

### Step 4: Test in QEMU

```bash
qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a53 \
  -m 512M \
  -nographic \
  -kernel bootimage.elf
```

Expected output:
```
═══════════════════════════════════════════════════════════
  KaaL Elfloader v0.1.0 - Rust-based seL4 Boot Loader
═══════════════════════════════════════════════════════════

DTB address: 0x40000000
Device tree parsed successfully
Model: linux,dummy-virt
Memory region: 0x40000000 - 0x60000000 (512 MB)

Deserializing payload...
Payload size: 2412800 bytes
Payload metadata:
  Kernel entry: 0x40000000
  User entry:   0x400000
  Kernel regions: 2
  User regions:   3

Loading kernel regions...
  0x40000000 <- 1146880 bytes
  0x40118000 <- 32768 bytes zeroed (BSS)

Loading user regions...
  0x40322000 <- 1196032 bytes
  0x40448000 <- 36864 bytes zeroed (BSS)

Images loaded successfully!

Setting up page tables...
Page tables configured
TTBR0: 0x11000000

Enabling MMU...
MMU enabled successfully

Jumping to seL4 kernel at 0x40000000...
═══════════════════════════════════════════════════════════

Bootstrapping kernel
...
```

## Advanced Usage

### Custom Physical Addresses

```bash
kaal-elfloader-builder \
  --loader elfloader.a \
  --kernel kernel.elf \
  --app root-task \
  --out bootimage.elf \
  --kernel-paddr 0x80000000 \   # Load kernel at 2GB
  --app-offset 0x1000000        # 16MB gap
```

### Debug Logging

```bash
RUST_LOG=debug kaal-elfloader-builder ...
```

### Multiple Configurations

```bash
# Minimal kernel (testing)
kaal-elfloader-builder \
  --loader elfloader-minimal.a \
  --kernel kernel-minimal.elf \
  --app test-task \
  --kernel-paddr 0x40000000 \
  --out bootimage-test.elf

# Production kernel
kaal-elfloader-builder \
  --loader elfloader-release.a \
  --kernel kernel-release.elf \
  --app production-root-task \
  --kernel-paddr 0x40000000 \
  --out bootimage-production.elf
```

## Troubleshooting

### "Payload not initialized!" Panic

**Cause**: Elfloader was run without being patched by the builder.

**Solution**: Always run `kaal-elfloader-builder` after building elfloader:
```bash
cargo build --release -p kaal-elfloader
kaal-elfloader-builder --loader ... --out ...
```

### ELF Parsing Errors

**Cause**: Input files are not valid ARM64 ELF binaries.

**Solution**: Check file types:
```bash
file kernel.elf
# Should show: ELF 64-bit LSB executable, ARM aarch64
```

### Memory Overlap Errors

**Cause**: Kernel and user images overlap in physical memory.

**Solution**: Increase `--app-offset`:
```bash
kaal-elfloader-builder --app-offset 0x800000 ...  # 8MB gap
```

### QEMU Hangs at Boot

**Possible causes**:
1. Wrong platform (`-machine virt` required for ARM)
2. Wrong CPU (`-cpu cortex-a53` or similar)
3. Missing `-nographic` flag

**Solution**:
```bash
qemu-system-aarch64 -machine virt -cpu cortex-a53 -nographic -kernel bootimage.elf
```

## Implementation Status

### ✅ Completed

- [x] Elfloader runtime (ARM64 entry, MMU, UART)
- [x] ELF parsing and loading infrastructure
- [x] Payload serialization with postcard
- [x] Device tree handling
- [x] Kernel handoff interface
- [x] Builder tool CLI
- [x] Build system integration

### 🚧 In Progress

- [ ] Elfloader binary patching (currently outputs `.payload` file)
- [ ] Final bootable ELF generation

### 📋 Future Work

- [ ] SMP (multi-core) support
- [ ] Additional platforms (Raspberry Pi, etc.)
- [ ] Image verification (checksums)
- [ ] Compression support
- [ ] CI/CD integration
- [ ] Automated testing in QEMU

## References

- [runtime/elfloader/README.md](elfloader/README.md) - Elfloader runtime docs
- [runtime/elfloader-builder/README.md](elfloader-builder/README.md) - Builder tool docs
- [runtime/elfloader/IMPLEMENTATION.md](elfloader/IMPLEMENTATION.md) - Technical details
- [seL4 elfloader-tool](https://github.com/seL4/seL4_tools/tree/master/elfloader-tool) - Original C implementation
- [rust-sel4 kernel-loader](https://github.com/seL4/rust-sel4/tree/main/crates/sel4-kernel-loader) - Reference Rust implementation

## License

Apache 2.0 (same as KaaL project)
