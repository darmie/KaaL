# KaaL Quick Start Guide

Get up and running with KaaL in 5 minutes.

## Prerequisites

- Docker installed and running
- macOS, Linux, or WSL2
- 4GB+ free disk space
- QEMU (optional, for testing)

## Build a Bootable Image

### One-Command Build

```bash
./tools/build-bootimage.sh
```

That's it! This will:
1. Build seL4 kernel
2. Build Rust elfloader
3. Build minimal root task
4. Create bootimage.elf

### Build and Test

```bash
./tools/build-bootimage.sh --test
```

This builds the image and automatically runs it in QEMU.

## Test the Image

### In QEMU

```bash
qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a53 \
  -m 512M \
  -nographic \
  -kernel bootimage.elf
```

**Press Ctrl-A then X to exit QEMU**

## Learn More

- [tools/README.md](tools/README.md) - Build tools documentation
- [runtime/ELFLOADER_GUIDE.md](runtime/ELFLOADER_GUIDE.md) - Complete elfloader guide
- [BUILD_BOOTABLE_IMAGE.md](BUILD_BOOTABLE_IMAGE.md) - Detailed build guide
