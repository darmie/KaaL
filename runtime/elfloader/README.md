# KaaL Elfloader

A Rust-based bootloader for the KaaL microkernel on ARM64 (AArch64) platforms.

## Overview

The KaaL Elfloader is a native Rust bootloader that handles early-stage system initialization and loads the KaaL microkernel. It handles:

- **ARM64 Boot Initialization**: Entry point, stack setup, BSS clearing
- **Memory Management**: Page table setup (MMU configuration deferred to kernel)
- **ELF Loading**: Parsing and loading kernel and root task ELF images
- **Device Tree**: Processing hardware description from firmware
- **Kernel Handoff**: Transferring control to KaaL kernel with proper boot parameters

## Why Rust?

1. **Safety**: Memory safety without runtime overhead
2. **Integration**: Seamless integration with KaaL's Rust-first architecture
3. **Maintainability**: Modern language features and tooling
4. **Control**: Full control over boot process without build system complexity
5. **Debugging**: Better error messages and debugging experience

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                Firmware (QEMU/U-Boot/UEFI)              │
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
│  │    - Load kernel ELF segments                    │  │
│  │    - Load root task ELF segments                 │  │
│  │    - Setup page tables                           │  │
│  │    - Prepare boot parameters                     │  │
│  └───────────────────┬──────────────────────────────┘  │
└────────────────────────┼────────────────────────────────┘
                         │ Call kernel_entry()
                         ▼
┌─────────────────────────────────────────────────────────┐
│                    KaaL Microkernel                     │
│         (Initializes MMU, creates capabilities)         │
└────────────────────────┬────────────────────────────────┘
                         │ Starts initial task
                         ▼
┌─────────────────────────────────────────────────────────┐
│                   KaaL Root Task                        │
│         (IPC, Memory Management, Services)              │
└─────────────────────────────────────────────────────────┘
```

## Memory Layout

```
Physical Memory (QEMU virt platform):
┌──────────────────┐ 0x00000000
│   Device Memory  │ Peripherals
├──────────────────┤ 0x09000000
│   UART (PL011)   │ Serial console
├──────────────────┤ 0x40000000
│   DTB            │ Device tree from firmware
├──────────────────┤ 0x40200000
│   Elfloader      │ This code + stack
│   .text._start   │ Entry point
│   .text          │ Code
│   .rodata        │ Read-only data
│   .kernel_elf    │ Embedded kernel ELF
│   .roottask_data │ Embedded root task ELF
│   .data          │ Data
│   .bss           │ Zero-initialized data
│   Stack          │ Stack space
├──────────────────┤ 0x40400000
│   KaaL Kernel    │ Loaded from ELF segments
│   .text          │ Kernel code
│   .rodata        │ Kernel read-only data
│   .data/.bss     │ Kernel data
├──────────────────┤ ~0x40410000
│   Root Task      │ Loaded from ELF segments
├──────────────────┤ 0x47FFC000
│   Page Tables    │ L1, L2, L3 tables (optional)
├──────────────────┤ 0x48000000
│   End of RAM     │
└──────────────────┘
```

## Building

### Prerequisites

- Rust nightly toolchain
- `rust-src` component for `build-std`
- KaaL kernel built
- Root task built (or dummy placeholder)

### Build Command

Use the provided build script:

```bash
cd runtime
bash build-kaal-with-elfloader.sh
```

This produces a bootable image at:
```
runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
```

## Testing in QEMU

```bash
qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a53 \
  -m 128M \
  -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
```

Expected output:
```
═══════════════════════════════════════════════════════════
  KaaL Elfloader v0.1.0 - Rust Microkernel Boot Loader
═══════════════════════════════════════════════════════════

DTB address: 0x40000000
Device tree parsed successfully
Model: linux,dummy-virt
Memory region: 0x40000000 - 0x48000000 (128 MB)

Loading images...
ELF: entry=0x40400000, 4 program headers at offset 0x40
  LOAD segment 0: vaddr=0x40400000, filesz=0x1430, memsz=0x1430
  LOAD segment 1: vaddr=0x40402000, filesz=0x9f8, memsz=0x9f8
  LOAD segment 2: vaddr=0x40403000, filesz=0x0, memsz=0x4000
Kernel loaded at entry point: 0x40400000
...
Jumping to KaaL kernel at 0x40400000...

═══════════════════════════════════════════════════════════
  KaaL Rust Microkernel v0.1.0
  Chapter 1: Bare Metal Boot & Early Init
═══════════════════════════════════════════════════════════
```

## Boot Parameters

The elfloader passes boot parameters to the KaaL kernel via ARM64 registers:

| Register | Parameter | Description |
|----------|-----------|-------------|
| x0 | `user_img_start` | Physical start of root task image |
| x1 | `user_img_end` | Physical end of root task image |
| x2 | `pv_offset` | Physical-to-virtual offset |
| x3 | `user_entry` | Root task entry point |
| x4 | `dtb_addr` | Device tree blob address |
| x5 | `dtb_size` | Device tree blob size |

## Implementation Status

### ✅ Chapter 1: Complete

- [x] ARM64 entry point and boot sequence
- [x] UART driver (PL011)
- [x] Device tree parsing
- [x] ELF parsing and segment loading
- [x] Kernel handoff with boot parameters
- [x] Build system integration

### 🚧 Future Work

- [ ] SMP (multi-core) support
- [ ] Additional platforms (Raspberry Pi 4, etc.)
- [ ] Image verification
- [ ] Compression support

## Key Features

### Proper ELF Loading
Correctly parses program headers and loads only PT_LOAD segments (not entire ELF file with headers).

### Platform Feature Flags
```rust
#[cfg(feature = "platform-qemu-virt")]
const UART_BASE: usize = 0x9000000;
```

### Custom Target JSON
Uses LLD linker for macOS compatibility and ELF linker script support.

## References

- [ARM Architecture Reference Manual](https://developer.arm.com/documentation/ddi0487/latest)
- [Linux ARM64 Boot Protocol](https://www.kernel.org/doc/html/latest/arm64/booting.html)
- [Device Tree Specification](https://www.devicetree.org/)
- [ELF64 Specification](https://refspecs.linuxfoundation.org/elf/elf.pdf)

## License

Apache 2.0 (same as KaaL project)
