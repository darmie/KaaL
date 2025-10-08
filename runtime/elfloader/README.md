# KaaL Elfloader

A Rust-based bootloader for seL4 microkernel, designed specifically for the KaaL framework on ARM64 (AArch64) platforms.

## Overview

The KaaL Elfloader replaces seL4's C-based elfloader-tool with a native Rust implementation. It handles:

- **ARM64 Boot Initialization**: Entry point, stack setup, BSS clearing
- **Memory Management**: MMU configuration and page table setup
- **ELF Loading**: Parsing and loading kernel and user (root task) images
- **Device Tree**: Processing hardware description from firmware
- **Kernel Handoff**: Transferring control to seL4 with proper boot parameters

## Why Rust?

1. **Safety**: Memory safety without runtime overhead
2. **Integration**: Seamless integration with KaaL's Rust-first architecture
3. **Maintainability**: Modern language features and tooling
4. **Control**: Full control over boot process without CMake complexity
5. **Debugging**: Better error messages and debugging experience

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Firmware (U-Boot/UEFI)               │
│                 Loads elfloader into memory             │
└────────────────────────┬────────────────────────────────┘
                         │ x0 = DTB address
                         ▼
┌─────────────────────────────────────────────────────────┐
│              KaaL Elfloader (Rust)                      │
│  ┌──────────────────────────────────────────────────┐  │
│  │ 1. _start (assembly)                             │  │
│  │    - Preserve DTB address                        │  │
│  │    - Setup stack                                 │  │
│  │    - Clear BSS                                   │  │
│  │    - Jump to Rust                                │  │
│  └───────────────────┬──────────────────────────────┘  │
│                      ▼                                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │ 2. elfloader_main                                │  │
│  │    - Initialize UART                             │  │
│  │    - Parse device tree                           │  │
│  │    - Load kernel ELF                             │  │
│  │    - Load user (root task) ELF                   │  │
│  │    - Setup page tables                           │  │
│  │    - Enable MMU                                  │  │
│  │    - Prepare boot parameters                     │  │
│  └───────────────────┬──────────────────────────────┘  │
└────────────────────────┼────────────────────────────────┘
                         │ Call kernel_entry()
                         ▼
┌─────────────────────────────────────────────────────────┐
│                    seL4 Microkernel                     │
│              (Initializes and starts root task)         │
└────────────────────────┬────────────────────────────────┘
                         │ Creates initial task
                         ▼
┌─────────────────────────────────────────────────────────┐
│                   KaaL Root Task                        │
│         (Cap Broker, IPC, Platform Services)           │
└─────────────────────────────────────────────────────────┘
```

## Memory Layout

```
Physical Memory:
┌──────────────────┐ 0x00000000
│   Device Memory  │ UART, peripherals
├──────────────────┤ 0x10000000
│   Elfloader      │ This code + stack (1MB)
│   .text.boot     │ Entry point
│   .text          │ Code
│   .rodata.kernel │ Embedded kernel ELF
│   .rodata.user   │ Embedded user ELF
│   .data          │ Data
│   .bss           │ Zero-initialized data
│   Stack          │ 1MB stack space
├──────────────────┤ ~0x11100000
│   Page Tables    │ L1, L2, L3 tables
├──────────────────┤ 0x40000000 (typical)
│   seL4 Kernel    │ Loaded from ELF
├──────────────────┤
│   User Image     │ Root task (KaaL)
├──────────────────┤
│   Device Tree    │ Relocated DTB
├──────────────────┤
│   Free Memory    │ Given to seL4
└──────────────────┘
```

## Module Structure

- **`src/lib.rs`**: Main entry point and kernel handoff
- **`src/arch/aarch64.rs`**: ARM64-specific code (`_start`, MMU ops)
- **`src/mmu.rs`**: Page table management
- **`src/elf.rs`**: ELF parsing and loading
- **`src/boot.rs`**: Boot sequence orchestration
- **`src/uart.rs`**: PL011 UART driver for debug output
- **`src/utils.rs`**: Utility functions (alignment, etc.)

## Building

### Prerequisites

- Rust nightly toolchain
- `aarch64-unknown-none` target
- seL4 kernel ELF image
- KaaL root task ELF image

### Build Command

```bash
cd runtime/elfloader

# With custom kernel/user images
KERNEL_IMAGE_PATH=/path/to/kernel.elf \
USER_IMAGE_PATH=/path/to/user.elf \
cargo build --release \
  --target aarch64-unknown-none \
  -Z build-std=core,alloc \
  -Z build-std-features=compiler-builtins-mem
```

### Output

The build produces `target/aarch64-unknown-none/release/libkaal_elfloader.a`, which can be:
1. Linked into a final bootable image
2. Converted to a raw binary for direct loading
3. Packaged as a U-Boot uImage

## Testing in QEMU

```bash
qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a53 \
  -nographic \
  -m 512M \
  -kernel path/to/elfloader.elf
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

Loading images...
Kernel entry: 0x40080000
User image: 0x40200000 - 0x40400000

Setting up page tables...
Page tables configured
TTBR0: 0x11000000

Enabling MMU...
MMU enabled successfully

Jumping to seL4 kernel at 0x40080000...
═══════════════════════════════════════════════════════════

Bootstrapping kernel
...
```

## Boot Parameters

The elfloader passes these parameters to the seL4 kernel (via registers):

| Register | Parameter | Description |
|----------|-----------|-------------|
| x0 | `user_img_start` | Physical address of user image |
| x1 | `user_img_end` | End of user image |
| x2 | `pv_offset` | Physical-to-virtual offset |
| x3 | `user_entry` | User image entry point |
| x4 | `dtb_addr` | Device tree location |
| x5 | `dtb_size` | Device tree size |

## Implementation Status

### ✅ Completed

- [x] ARM64 entry point (`_start`)
- [x] UART driver (PL011)
- [x] Device tree parsing
- [x] ELF parsing infrastructure
- [x] MMU setup and page tables
- [x] Kernel handoff interface
- [x] Build system integration

### 🚧 In Progress

- [ ] Full ELF loading implementation
- [ ] Embedded image support (CPIO)
- [ ] SMP (multi-core) support

### 📋 Future Work

- [ ] Additional platform support (Raspberry Pi, etc.)
- [ ] Image verification (checksums)
- [ ] Compression support
- [ ] EFI boot protocol
- [ ] ACPI support (for x86_64 port)

## Comparison with C Elfloader

| Feature | C Elfloader | Rust Elfloader |
|---------|-------------|----------------|
| Language | C | Rust |
| Lines of Code | ~3000 | ~800 |
| Memory Safety | Manual | Compiler-enforced |
| Build System | CMake (complex) | Cargo (simple) |
| Dependencies | seL4 build system | Standalone |
| Debugging | Limited | Good (backtraces, formatting) |
| Platforms | Multi (ARM, RISC-V, x86) | ARM64 (extensible) |

## References

- [seL4 elfloader-tool](https://github.com/seL4/seL4_tools/tree/master/elfloader-tool)
- [rust-sel4 kernel-loader](https://github.com/seL4/rust-sel4/tree/main/crates/sel4-kernel-loader)
- [ARM ARM (Architecture Reference Manual)](https://developer.arm.com/documentation/ddi0487/latest)
- [Linux ARM64 Boot Protocol](https://www.kernel.org/doc/html/latest/arm64/booting.html)

## License

Apache 2.0 (same as KaaL project)
