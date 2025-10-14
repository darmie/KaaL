# KaaL Build Tools

This directory contains tools for building and testing KaaL bootable images.

## Quick Start

Build a bootable image:

```bash
./tools/build-bootimage.sh
```

Build and test in QEMU:

```bash
./tools/build-bootimage.sh --test
```

## Tools

### `build-bootimage.sh`

Main build orchestration script that uses Docker to build a complete bootable seL4 image.

**Usage:**
```bash
./tools/build-bootimage.sh [options]
```

**Options:**
- `--clean` - Clean temporary workspace before build
- `--keep-workspace` - Keep temporary build workspace after completion
- `--test` - Run QEMU test after build
- `--output PATH` - Output path for bootimage.elf (default: ./bootimage.elf)
- `--help` - Show help message

**Examples:**

```bash
# Basic build
./tools/build-bootimage.sh

# Clean build with QEMU test
./tools/build-bootimage.sh --clean --test

# Build with custom output location
./tools/build-bootimage.sh --output /tmp/my-boot.elf

# Build and keep workspace for inspection
./tools/build-bootimage.sh --keep-workspace
```

### `Dockerfile.bootimage`

Multi-stage Dockerfile that builds all components:

1. **Stage 1**: Build environment setup
2. **Stage 2**: Build seL4 kernel
3. **Stage 3**: Build Rust elfloader
4. **Stage 4**: Create minimal root task
5. **Stage 5**: Build elfloader-builder tool
6. **Stage 6**: Assemble bootable image
7. **Stage 7**: Extract output

## What Gets Built

The build process creates:

1. **seL4 Kernel** (`kernel.elf`)
   - Platform: QEMU ARM virt
   - Architecture: ARM64 (AArch64)
   - Load address: 0x40000000

2. **KaaL Elfloader** (`libkaal_elfloader.a`)
   - Pure Rust bootloader
   - Loads kernel + root task
   - Sets up MMU and page tables

3. **Minimal Root Task** (`libminimal_root_task.a`)
   - Simple test application
   - Just spins in infinite loop

4. **Bootable Image** (`bootimage.elf`)
   - Complete image ready for QEMU
   - Elfloader + serialized payload (kernel + root task)

## Build Workflow

```
┌─────────────────────────────────────────────────────────┐
│  ./tools/build-bootimage.sh                             │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  1. Creates temporary workspace (.build-workspace/)     │
│                                                          │
│  2. Runs Docker build (Dockerfile.bootimage):           │
│     ┌─────────────────────────────────────────────┐    │
│     │ Stage 1: Setup build environment            │    │
│     │ Stage 2: Build seL4 kernel                  │    │
│     │ Stage 3: Build Rust elfloader               │    │
│     │ Stage 4: Create minimal root task           │    │
│     │ Stage 5: Build elfloader-builder tool       │    │
│     │ Stage 6: Assemble bootable image            │    │
│     │ Stage 7: Prepare output                     │    │
│     └─────────────────────────────────────────────┘    │
│                                                          │
│  3. Extracts bootimage.elf from container               │
│                                                          │
│  4. Optionally tests in QEMU (--test flag)              │
│                                                          │
│  5. Cleans up workspace (unless --keep-workspace)       │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

## Output Files

After successful build:

```
.
├── bootimage.elf              # Main bootable image (extracted)
└── .build-workspace/          # Temporary workspace (optional)
    ├── docker-build.log       # Docker build log
    ├── build-info.txt         # Build information
    └── bootimage.payload      # Serialized payload
```

## Testing

### QEMU Test

```bash
# Automatic test
./tools/build-bootimage.sh --test

# Manual test
qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a53 \
  -m 512M \
  -nographic \
  -kernel bootimage.elf
```

**Expected output:**
```
═══════════════════════════════════════════════════════════
  KaaL Elfloader v0.1.0 - Rust-based seL4 Boot Loader
═══════════════════════════════════════════════════════════

DTB address: 0x40000000
Device tree parsed successfully
Model: linux,dummy-virt
Memory region: 0x40000000 - 0x60000000 (512 MB)

Deserializing payload...
Payload size: <size> bytes
Payload metadata:
  Kernel entry: 0x40000000
  User entry:   0x400000
  Kernel regions: <N>
  User regions:   <M>

Loading kernel regions...
  0x40000000 <- <size> bytes

Loading user regions...
  0x<address> <- <size> bytes

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

Press `Ctrl-A` then `X` to exit QEMU.

### Inspect Build

```bash
# Build with workspace preserved
./tools/build-bootimage.sh --keep-workspace

# Check build logs
cat .build-workspace/docker-build.log

# Check build info
cat .build-workspace/build-info.txt

# Inspect bootable image
file bootimage.elf
readelf -h bootimage.elf
```

## Troubleshooting

### Docker Not Found

```bash
# Install Docker Desktop for Mac
# Or install via Homebrew
brew install --cask docker
```

### Build Fails at Kernel Stage

Check Docker build log:
```bash
./tools/build-bootimage.sh --keep-workspace
cat .build-workspace/docker-build.log | grep -A 10 "Stage 2"
```

### Build Fails at Elfloader Stage

Common issue: Rust toolchain not configured correctly.
The Dockerfile installs everything needed, so this shouldn't happen.

If it does:
```bash
# Rebuild with clean cache
docker build --no-cache -f tools/Dockerfile.bootimage -t kaal-builder .
```

### QEMU Not Found

```bash
# Install QEMU
brew install qemu
```

### QEMU Hangs

Common causes:
- Wrong platform (`-machine virt` required)
- Wrong CPU (`-cpu cortex-a53`)
- Missing `-nographic` flag

Correct command:
```bash
qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a53 \
  -m 512M \
  -nographic \
  -kernel bootimage.elf
```

## Advanced Usage

### Custom Kernel Configuration

To build with custom kernel config, modify `Dockerfile.bootimage` Stage 2:

```dockerfile
RUN cd /build/kernel/build && \
    cmake .. \
        -G Ninja \
        -DCMAKE_TOOLCHAIN_FILE=../gcc.cmake \
        -DPLATFORM=qemu-arm-virt \
        -DAARCH64=1 \
        -DKernelPrinting=ON \
        -DKernelDebugBuild=ON \      # Enable debug
        -DKernelVerificationBuild=ON # Enable verification
        && \
    ninja kernel.elf
```

### Custom Root Task

Replace Stage 4 in `Dockerfile.bootimage` with your own root task build.

### Different Platform

Modify kernel build stage:
```dockerfile
-DPLATFORM=imx8mq-evk  # For i.MX8MQ
-DPLATFORM=tx2         # For NVIDIA TX2
```

## Development Workflow

For active development:

```bash
# 1. Make changes to elfloader/kernel/root-task

# 2. Quick test build
./tools/build-bootimage.sh --test

# 3. If successful, commit changes
git add -A
git commit -m "Update elfloader implementation"

# 4. Clean up
rm -rf .build-workspace/
```

## CI/CD Integration

Example GitHub Actions workflow:

```yaml
name: Build Bootable Image

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build bootable image
        run: ./tools/build-bootimage.sh --clean

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: bootimage
          path: bootimage.elf

      - name: Test in QEMU
        run: |
          sudo apt-get install -y qemu-system-arm
          timeout 30s ./tools/build-bootimage.sh --test || true
```

## See Also

- [../runtime/ELFLOADER_GUIDE.md](../runtime/ELFLOADER_GUIDE.md) - Complete elfloader system guide
- [../runtime/elfloader/README.md](../runtime/elfloader/README.md) - Elfloader runtime docs
- [../runtime/elfloader-builder/README.md](../runtime/elfloader-builder/README.md) - Builder tool docs
