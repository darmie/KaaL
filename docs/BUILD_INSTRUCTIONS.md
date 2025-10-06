# KaaL Build Instructions

## Overview

KaaL builds operating systems for **ARM64, x86_64, RISC-V**, and any seL4-supported platform.

**Build host**: Linux (or Docker on macOS/Windows) for real seL4 builds
**Target platform**: Any seL4-supported architecture

The Linux build requirement is a tooling limitation, not fundamental to KaaL.

---

## Quick Start

### Build OS for Any Target Architecture

**Build Host**: Linux (or Docker on macOS)

**Target Platforms**: Choose your deployment hardware

```bash
# 1. Install seL4 SDK and Microkit
# Follow: https://docs.sel4.systems/projects/microkit/

# 2. Set environment variable
export SEL4_PREFIX=/path/to/seL4

# 3. Build OS for your target hardware:

# For ARM64 boards (Raspberry Pi, BeagleBone, etc.)
cargo build --features board-qemu-virt-aarch64

# For x86_64 PCs
cargo build --features board-pc99

# For RISC-V boards
cargo build --features board-qemu-virt-riscv64
```

### Framework Development (Any Platform)

**Build Host**: macOS, Linux, Windows

```bash
# Algorithm development with mocks
cargo build --no-default-features --features mock
```

---

## Build Modes

### 🏭 Production Mode (Default)

**Platform**: Linux with seL4 SDK
**Backend**: Real seL4 + Microkit
**Default**: ✅ YES

```bash
cargo build  # Uses microkit by default
```

**Requirements**:
- Linux (Ubuntu 20.04+ recommended)
- seL4 SDK with Microkit
- `SEL4_PREFIX` environment variable set
- ARM64/x86_64/RISC-V toolchain

**Architecture variants**:
```bash
# ARM64 (default)
cargo build

# x86_64
cargo build --no-default-features --features "microkit,board-pc99"

# RISC-V
cargo build --no-default-features --features "microkit,board-qemu-virt-riscv64"
```

---

### 🧪 Mock Mode (macOS Development)

**Platform**: Any (macOS, Linux, Windows)
**Backend**: Mock syscalls
**Default**: ❌ NO (must explicitly enable)

```bash
# Explicit mock mode
cargo build --no-default-features --features mock

# Using alias
cargo build-mock
```

**Use for**:
- Algorithm development on macOS
- Unit testing
- CI/CD on non-Linux platforms
- API design and prototyping

---

## Environment Setup

### Linux Development Environment

#### Option 1: Native Linux

```bash
# Install dependencies
sudo apt-get install build-essential cmake ninja-build \
    python3-dev python3-pip device-tree-compiler \
    qemu-system-arm qemu-system-x86

# Install seL4 dependencies
pip3 install --user sel4-deps

# Build seL4 kernel
cd /path/to/seL4
mkdir build && cd build
cmake -DPLATFORM=qemu-arm-virt \
      -DAARCH64=1 \
      -G Ninja \
      ..
ninja

# Set environment for KaaL
export SEL4_PREFIX=/path/to/seL4
```

#### Option 2: Docker

```bash
# Use seL4 Docker image
docker run -it --rm \
    -v $(pwd):/workspace \
    -w /workspace \
    -e SEL4_PREFIX=/seL4 \
    trustworthysystems/sel4 \
    cargo build
```

#### Option 3: Dev Container (VSCode)

Use the provided `.devcontainer` configuration (recommended).

---

### macOS Development Environment

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build with mocks (no seL4 SDK needed)
cargo build-mock
```

**Note**: Real seL4 builds are **not supported on macOS**. Use Linux VM/Docker or contribute to cross-platform seL4 tooling.

---

## Verifying Your Build

### Automated Verification

```bash
# Run verification script
./scripts/verify_real_sel4_default.sh
```

### Manual Check - Active Backend

```bash
# Should show 'microkit' on Linux
cargo tree -p sel4-platform -e features | grep -E "sel4|microkit|mock"
```

**Expected on Linux (default)**:
```
sel4-platform v0.1.0
├── sel4 feature "microkit"
├── sel4-sys
├── sel4-config
└── sel4-microkit
```

**Expected on macOS (with mock)**:
```
sel4-platform v0.1.0
├── sel4-mock feature "mock"
└── sel4-mock-sys
```

---

## Common Build Issues

### Issue 1: `SEL4_INCLUDE_DIRS or SEL4_PREFIX must be set`

**Cause**: Building with real seL4 without SDK
**Solution**:
```bash
# Option A: Install seL4 SDK and set SEL4_PREFIX
export SEL4_PREFIX=/path/to/seL4

# Option B: Use mock mode (macOS)
cargo build-mock
```

### Issue 2: `kernel/gen_config.json not found`

**Cause**: seL4 kernel not built/configured
**Solution**:
```bash
# Build seL4 kernel first
cd $SEL4_PREFIX
mkdir build && cd build
cmake -DPLATFORM=qemu-arm-virt -DAARCH64=1 -G Ninja ..
ninja
```

### Issue 3: `unexpected token in '.section' directive`

**Cause**: Platform-specific assembly (x86 on ARM macOS)
**Solution**:
```bash
# This is EXPECTED on macOS - use mock mode
cargo build-mock
```

### Issue 4: `two packages named 'sel4' in this workspace`

**Cause**: rust-sel4 workspace conflict (should not happen - we exclude it)
**Solution**:
```bash
# Verify .gitmodules excludes rust-sel4 workspace
grep exclude Cargo.toml  # Should show external/rust-sel4
```

---

## Development Workflow

### Recommended: Linux-First Development

```bash
# 1. Develop on Linux with real seL4
export SEL4_PREFIX=/path/to/seL4
cargo build
cargo test

# 2. Deploy to QEMU
cargo build --release
microkit system.toml

# 3. Test on hardware
# Flash to ARM64/x86_64/RISC-V board
```

### Alternative: macOS for Algorithms

```bash
# 1. Develop algorithms with mocks
cargo build-mock
cargo test

# 2. Test integration on Linux VM/Docker
docker run -v $(pwd):/workspace \
    -e SEL4_PREFIX=/seL4 \
    trustworthysystems/sel4 \
    cargo build

# 3. Deploy from Linux
```

---

## Build Artifacts

### Mock Mode Output
```
target/debug/
├── kaal-*              # Binaries run on host OS
└── libsel4_mock.a      # Mock seL4 library
```

### Microkit Mode Output
```
target/debug/
├── kaal-*.elf          # seL4 userspace programs
└── system.img          # Microkit system image (via microkit tool)
```

---

## CI/CD Configuration

### GitHub Actions Example

```yaml
name: Build

on: [push, pull_request]

jobs:
  # Mock builds on all platforms
  test-mock:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
      - run: cargo build-mock
      - run: cargo test

  # Real seL4 builds on Linux only
  build-sel4:
    runs-on: ubuntu-latest
    container: trustworthysystems/sel4
    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive
      - run: |
          export SEL4_PREFIX=/seL4
          cargo build --release
```

---

## Summary

| Platform | Default Build | Command | Use Case |
|----------|--------------|---------|----------|
| **Linux** | ✅ Real seL4 Microkit | `cargo build` | Production development |
| **macOS** | ❌ Fails (needs SDK) | `cargo build-mock` | Algorithm development |
| **Windows** | ❌ Not supported | `cargo build-mock` | Testing only |

**Key Principle**: KaaL defaults to **REAL seL4** - mocks are explicitly opt-in for cross-platform development.

---

## Getting Help

- **seL4 SDK Issues**: https://docs.sel4.systems
- **Microkit Guide**: https://github.com/seL4/microkit
- **KaaL Integration**: See `docs/SEL4_INTEGRATION_STATUS.md`
- **Build Modes**: See `docs/BUILD_MODES.md`
