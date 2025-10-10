# Linker Script Location

The linker script for the bootable image is **NOT** located in this directory.

## Authoritative Linker Script

**Location**: [`tools/bootimage.ld`](../../tools/bootimage.ld)

This linker script is used by the Docker multi-stage build system and defines:
- Load address: `0x40100000` (after DTB region in QEMU virt RAM)
- Memory sections for elfloader code, kernel image, and root task
- Stack configuration (1MB)
- Symbol exports for memory regions

## Why is it in tools/?

The linker script is in `tools/` because it's part of the **build system**, not the elfloader source code. The bootable image is created by:

1. Building elfloader as a static library (`.a` file)
2. Embedding kernel and root task as binary data
3. Linking everything together with `tools/bootimage.ld`

The elfloader itself is built with standard Rust/Cargo for `aarch64-unknown-none` target and doesn't need its own linker script during the library build phase.

## Memory Layout (QEMU ARM virt)

```
0x00000000 - 0x08000000    Flash/ROM region
0x09000000 - 0x09001000    UART PL011 device
0x40000000 - 0x40100000    Device Tree Blob (DTB) - 1MB
0x40100000 - ...           Elfloader + embedded images
```

The DTB is placed by QEMU at the base of RAM (0x40000000), so the elfloader must load after it to avoid overlap.

## Build Process

See [`tools/Dockerfile.bootimage`](../../tools/Dockerfile.bootimage) for the complete multi-stage Docker build process that uses this linker script.
