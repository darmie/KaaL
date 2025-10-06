# KaaL Build Modes

**Last Updated**: 2025-10-05

## Overview

KaaL supports three build modes for seL4 integration, allowing development on any platform while deploying to real seL4.

## Build Modes

### 1. Mock Mode (Default) ✅

**Purpose**: Cross-platform development and testing
**Platform**: macOS, Linux, Windows
**Default**: YES

```bash
# Uses mock backend automatically
cargo build

# Or explicitly
cargo build -p sel4-platform --features mock
```

**What it does**:
- Uses mock seL4 syscalls that return success
- No real kernel integration
- Works on any platform (including macOS ARM)
- Perfect for:
  - Algorithm development
  - API design
  - Unit testing
  - Cross-platform CI/CD

**Verification**:
```bash
$ cargo tree -p sel4-platform -e features
sel4-platform v0.1.0
├── sel4-mock feature "default"
│   └── sel4-mock-sys v0.1.0
```

---

### 2. Microkit Mode (Production)

**Purpose**: Real seL4 deployment using Microkit framework
**Platform**: Linux only (requires seL4 SDK)
**Default**: NO

```bash
# ARM64 QEMU
cargo build -p sel4-platform --no-default-features \
    --features "microkit,board-qemu-virt-aarch64"

# x86_64 PC
cargo build -p sel4-platform --no-default-features \
    --features "microkit,board-pc99"

# RISC-V QEMU
cargo build -p sel4-platform --no-default-features \
    --features "microkit,board-qemu-virt-riscv64"
```

**Requirements**:
- Linux build environment
- seL4 SDK installed with `SEL4_INCLUDE_DIRS` set
- seL4 Microkit toolchain
- Target architecture cross-compiler

**What it does**:
- Links to real seL4 kernel
- Uses actual seL4 syscalls
- Deploys as Microkit components
- Production-ready for embedded systems

**Expected on macOS**:
```
❌ error: SEL4_INCLUDE_DIRS or SEL4_PREFIX must be set
```
This is CORRECT - microkit mode requires Linux + seL4 SDK.

---

### 3. Runtime Mode (Advanced)

**Purpose**: Direct rust-sel4 runtime integration
**Platform**: Linux only (requires seL4 SDK)
**Default**: NO

```bash
cargo build -p sel4-platform --no-default-features \
    --features "runtime,arch-aarch64"
```

**Use cases**:
- Custom seL4 deployments without Microkit
- Direct kernel integration
- Advanced seL4 features

---

## Feature Flags

### Backend Selection (Mutually Exclusive)

| Feature | Backend | Platform | Default |
|---------|---------|----------|---------|
| `mock` | sel4-mock | Any | ✅ YES |
| `microkit` | rust-sel4 + Microkit | Linux | NO |
| `runtime` | rust-sel4 runtime | Linux | NO |

### Architecture/Board Selection

**For ARM64**:
- `board-qemu-virt-aarch64` - ARM64 QEMU
- `board-maaxboard` - ARM64 MaaxBoard
- `board-odroidc4` - ARM64 Odroid-C4
- `arch-aarch64` - Generic ARM64

**For x86_64**:
- `board-pc99` - x86_64 PC
- `arch-x86_64` - Generic x86_64

**For RISC-V**:
- `board-qemu-virt-riscv64` - RISC-V QEMU
- `board-pine64-star64` - RISC-V Star64
- `arch-riscv64` - Generic RISC-V

---

## How It Works

### Adapter Pattern

All KaaL code uses the platform adapter:

```rust
use sel4_platform::adapter as sel4;

unsafe {
    sel4::untyped_retype(...);  // Works in ALL modes
    sel4::tcb_configure(...);   // Delegates to active backend
}
```

### Compile-Time Backend Selection

```rust
// In sel4-platform/src/adapter.rs
#[cfg(feature = "mock")]
use sel4_mock_sys as backend;

#[cfg(feature = "microkit")]
use sel4_sys as backend;

#[cfg(feature = "runtime")]
use sel4_sys as backend;

// All syscalls delegate to selected backend
pub unsafe fn untyped_retype(...) -> Error {
    backend::seL4_Untyped_Retype(...)
}
```

---

## Testing Backend Selection

Run the included test script:

```bash
./scripts/test_backend_selection.sh
```

**Expected output**:
```
=== Testing seL4 Platform Backend Selection ===

1. Testing DEFAULT (should be mock)...
   ✅ Mock mode builds successfully (default)

2. Testing EXPLICIT MOCK mode...
   ✅ Explicit mock mode builds successfully

3. Testing MICROKIT mode...
   ⚠️  Microkit mode failed (EXPECTED on macOS - requires Linux + seL4 SDK)

4. Verifying mock is used by default...
sel4-platform v0.1.0
├── sel4-mock feature "default"
│   └── sel4-mock-sys v0.1.0
```

---

## Development Workflow

### Local Development (macOS/Windows)

```bash
# Develop with mocks (default)
cargo build
cargo test

# No seL4 SDK needed!
```

### CI/CD Pipeline

```yaml
# Mock mode for testing
- run: cargo test --all-features

# Microkit build in Linux container
- run: |
    docker run --rm -v $(pwd):/workspace seL4/microkit \
      cargo build --features microkit,board-qemu-virt-aarch64
```

### Production Deployment (Linux)

```bash
# 1. Install seL4 SDK
export SEL4_INCLUDE_DIRS=/path/to/seL4/include

# 2. Build for target board
cargo build --release \
    --no-default-features \
    --features "microkit,board-qemu-virt-aarch64"

# 3. Deploy with Microkit
microkit system.toml
```

---

## Verification

### Confirm Mock is Default

```bash
$ cargo build -p sel4-platform
$ cargo tree -p sel4-platform | head -5
sel4-platform v0.1.0
├── sel4-mock feature "default"
│   └── sel4-mock-sys v0.1.0
```

✅ If you see `sel4-mock`, mock mode is active (correct default)

### Confirm Microkit Requires Linux

```bash
$ cargo build -p sel4-platform --no-default-features --features microkit
error: SEL4_INCLUDE_DIRS or SEL4_PREFIX must be set
```

✅ This error on macOS is EXPECTED and CORRECT

---

## Summary

| Mode | Command | Works on macOS? | Use Case |
|------|---------|-----------------|----------|
| Mock (default) | `cargo build` | ✅ YES | Development |
| Microkit | `cargo build --features microkit,board-*` | ❌ NO (needs Linux) | Production |
| Runtime | `cargo build --features runtime,arch-*` | ❌ NO (needs Linux) | Advanced |

**Key Takeaway**:
- ✅ Mock mode is the default - works everywhere
- ✅ Microkit/runtime modes require Linux + seL4 SDK
- ✅ All modes use the same adapter API
- ✅ Switch modes with feature flags, zero code changes
