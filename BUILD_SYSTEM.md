# KaaL Build System

A configuration-driven, multi-platform build system for the KaaL microkernel.

## Overview

The KaaL build system generates platform-specific bootable images by:
1. Reading platform configuration from `build-config.toml`
2. Dynamically generating linker scripts with correct memory addresses
3. Building kernel, root task, and elfloader components
4. Packaging everything into a bootable ELF image

## Quick Start

```bash
# Build for default platform (QEMU virt)
./build.sh

# Build for specific platform
./build.sh --platform qemu-virt
./build.sh --platform rpi4

# Verbose output
./build.sh --platform qemu-virt -v
```

## Configuration File

Platform configurations are defined in [build-config.toml](build-config.toml).

### Supported Platforms

| Platform | Description | Memory Layout |
|----------|-------------|---------------|
| `qemu-virt` | QEMU ARM64 virt machine | RAM at 0x40000000 (128MB) |
| `rpi4` | Raspberry Pi 4 | RAM at 0x0 (1GB) |
| `generic-arm64` | Template for custom boards | Configurable |

### Platform Configuration

Each platform defines:

```toml
[platform.PLATFORM_NAME]
name = "Human readable name"
arch = "aarch64"
kernel_target = "aarch64-unknown-none"
elfloader_target_json = "aarch64-unknown-none-elf.json"

# Memory layout
ram_base = "0x40000000"       # Physical RAM base address
ram_size = "0x8000000"        # Total RAM size

# Device addresses
uart_base = "0x09000000"      # UART base address

# Boot memory layout (offsets from ram_base)
dtb_offset = "0x0"            # Device tree blob
elfloader_offset = "0x200000" # Bootloader
kernel_offset = "0x400000"    # Kernel

# Stack location
stack_top_offset = "0x8000000" # Stack grows down from here

# QEMU launch parameters (optional)
qemu_machine = "virt"
qemu_cpu = "cortex-a53"
qemu_memory = "128M"
```

## Build Process

### Step 1: Kernel Build

```
kernel/kernel.ld (generated) → kernel build → kernel ELF → kernel.o (embeddable)
```

- Generates `kernel/kernel.ld` with platform-specific addresses
- Builds kernel at configured `kernel_offset`
- Converts to embeddable object file

### Step 2: Root Task Build

```
root task build → root task ELF → roottask.o (embeddable)
```

- Builds root task (user-space initial process)
- Converts to embeddable object file

### Step 3: Elfloader Build

```
kernel.o + roottask.o + linker.ld (generated) → elfloader → bootable image
```

- Generates `runtime/elfloader/linker.ld` with embedded images
- Links kernel and root task objects into elfloader
- Produces final bootable ELF image

## Memory Layout

### QEMU virt Platform

```
0x40000000  ┌─────────────────┐  DTB (Device Tree Blob)
            │                 │
0x40200000  ├─────────────────┤  Elfloader
            │  .text           │
            │  .rodata         │
            │  .kernel_elf     │  ← Embedded kernel ELF
            │  .roottask_data  │  ← Embedded root task
            │  .data           │
            │  .bss            │
            ├─────────────────┤
0x40400000  │  Kernel          │  ← Loaded by elfloader
            │  .text           │
            │  .rodata         │
            │  .data           │
            │  .bss            │
            │  .stack          │
            └─────────────────┘
0x48000000  Stack top (elfloader)
```

### Raspberry Pi 4 Platform

```
0x0         ┌─────────────────┐  DTB
            │                 │
0x80000     ├─────────────────┤  Elfloader (standard ARM64 boot offset)
            │                 │
0x200000    ├─────────────────┤  Kernel
            │                 │
0x800000    └─────────────────┘  Stack top
```

## Adding a New Platform

1. Add platform configuration to `build-config.toml`:

```toml
[platform.my-board]
name = "My Custom Board"
arch = "aarch64"
kernel_target = "aarch64-unknown-none"
elfloader_target_json = "aarch64-unknown-none-elf.json"

ram_base = "0x80000000"
ram_size = "0x20000000"       # 512MB
uart_base = "0x10000000"

dtb_offset = "0x0"
elfloader_offset = "0x80000"
kernel_offset = "0x200000"
stack_top_offset = "0x1000000"
```

2. Build for your platform:

```bash
./build.sh --platform my-board
```

3. Deploy the bootable image to your hardware

## Generated Files

The build system generates these files (gitignored):

- `kernel/kernel.ld` - Kernel linker script with platform-specific addresses
- `runtime/elfloader/linker.ld` - Elfloader linker script with embedded images
- `runtime/build/kernel.o` - Embeddable kernel object
- `runtime/build/roottask.o` - Embeddable root task object

## Build Script Options

```bash
./build.sh [OPTIONS]

OPTIONS:
  --platform PLATFORM    Platform to build for (default: qemu-virt)
  -v, --verbose          Show detailed configuration
  -h, --help             Show help message
```

## Configuration-Driven Benefits

✅ **No hardcoded addresses** - All values read from config
✅ **Platform flexibility** - Easy to add new boards
✅ **Reproducible builds** - Same config = same output
✅ **Development speed** - No manual linker script editing
✅ **Documentation** - Config serves as platform documentation

## Troubleshooting

### Build fails with "Kernel not linked at correct address"

Check that `build-config.toml` has correct memory layout for your platform.

### QEMU doesn't boot

Verify QEMU launch parameters match platform config:

```bash
qemu-system-aarch64 \
  -machine $(config qemu_machine) \
  -cpu $(config qemu_cpu) \
  -m $(config qemu_memory) \
  -nographic \
  -kernel path/to/elfloader
```

### Adding custom platform

1. Copy an existing platform config in `build-config.toml`
2. Update memory addresses for your hardware
3. Test with `./build.sh --platform your-platform -v`

## Dependencies

- Rust nightly with `-Z build-std`
- `llvm-objcopy` (from LLVM toolchain)
- `awk` (for TOML parsing)
- QEMU (optional, for testing)

## See Also

- [build-config.toml](build-config.toml) - Platform configurations
- [kernel/README.md](kernel/README.md) - Kernel documentation
- [runtime/elfloader/README.md](runtime/elfloader/README.md) - Elfloader documentation
