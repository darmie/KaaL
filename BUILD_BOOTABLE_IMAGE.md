# Building a Bootable KaaL Image

This guide explains how to build a complete bootable image with the new Rust elfloader.

## Current Status

### ✅ What We Have

1. **seL4 Kernel** - Built and available at:
   - `/Users/amaterasu/Vibranium/kaal/examples/my-kaal-system/build/kernel.elf`
   - `/Users/amaterasu/Vibranium/kaal/build/kernel.elf`

2. **Elfloader-Builder Tool** - Just built successfully
   - Binary at: `target/release/kaal-elfloader-builder`

3. **Elfloader Runtime Source** - Complete implementation at `runtime/elfloader/`

### ❌ What We Need

1. **Elfloader Binary** - Needs cross-compilation for `aarch64-unknown-none`
2. **Root Task Binary** - A simple seL4 application to boot

## Why Docker?

Building on macOS (Apple Silicon) has challenges:
- Need ARM64 bare-metal toolchain (`aarch64-unknown-none`)
- Need Rust `no_std` cross-compilation
- Need seL4 syscall bindings properly configured

**Docker provides a clean, reproducible build environment.**

## Build Approaches

### Approach 1: Docker Build (Recommended)

Create a Dockerfile that builds everything:

```dockerfile
# Dockerfile.bootable
FROM rust:1.75 AS builder

# Install ARM64 cross-compilation tools
RUN rustup target add aarch64-unknown-none
RUN rustup component add rust-src

# Install seL4 dependencies
RUN apt-get update && apt-get install -y \
    cmake ninja-build gcc-aarch64-linux-gnu \
    device-tree-compiler qemu-system-arm

WORKDIR /build

# Copy source
COPY . .

# Stage 1: Build seL4 kernel
RUN cd examples/my-kaal-system && mkdir -p build && cd build && \
    cmake ../../../kernel -G Ninja \
        -DCMAKE_TOOLCHAIN_FILE=../../../kernel/gcc.cmake \
        -DPLATFORM=qemu-arm-virt \
        -DAARCH64=1 && \
    ninja kernel.elf

# Stage 2: Build elfloader
RUN cd runtime/elfloader && \
    cargo build --release \
        --target aarch64-unknown-none \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem

# Stage 3: Create minimal root task
RUN cd examples && mkdir -p minimal-root-task && cd minimal-root-task && \
    cat > Cargo.toml <<'EOF'
[package]
name = "minimal-root-task"
version = "0.1.0"
edition = "2021"

[dependencies]
sel4 = { path = "../../external/rust-sel4/crates/sel4", default-features = false }

[profile.release]
panic = "abort"
EOF
    && \
    mkdir src && cat > src/main.rs <<'EOF'
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
EOF
    && \
    cargo build --release --target aarch64-unknown-linux-gnu

# Stage 4: Package with elfloader-builder
RUN target/release/kaal-elfloader-builder \
    --loader runtime/elfloader/target/aarch64-unknown-none/release/libkaal_elfloader.a \
    --kernel examples/my-kaal-system/build/kernel.elf \
    --app examples/minimal-root-task/target/aarch64-unknown-linux-gnu/release/minimal-root-task \
    --out /bootimage.elf

# Final stage: Extract bootable image
FROM scratch
COPY --from=builder /bootimage.elf /bootimage.elf
```

Build with:
```bash
docker build -f Dockerfile.bootable -t kaal-bootable .
docker create --name kaal-extract kaal-bootable
docker cp kaal-extract:/bootimage.elf ./bootimage.elf
docker rm kaal-extract
```

### Approach 2: Step-by-Step Manual Build

This approach requires properly configured Rust toolchain and takes longer but provides more control.

#### Step 1: Install Dependencies

```bash
# Install Rust nightly and targets
rustup toolchain install nightly
rustup +nightly target add aarch64-unknown-none
rustup +nightly component add rust-src

# Note: On macOS, you cannot build seL4 kernel or elfloader natively
# Use Docker or a Linux VM
```

#### Step 2: Build Elfloader (Linux/Docker only)

```bash
cd runtime/elfloader

cargo +nightly build --release \
  --target aarch64-unknown-none \
  -Z build-std=core,alloc \
  -Z build-std-features=compiler-builtins-mem
```

Output: `target/aarch64-unknown-none/release/libkaal_elfloader.a`

#### Step 3: Create Minimal Root Task

```bash
mkdir -p /tmp/minimal-root-task
cd /tmp/minimal-root-task

cat > Cargo.toml <<'EOF'
[package]
name = "minimal-root-task"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib"]

[profile.release]
panic = "abort"
lto = true
EOF

mkdir src
cat > src/lib.rs <<'EOF'
#![no_std]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // Spin forever - minimal root task
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
EOF

cargo build --release --target aarch64-unknown-none -Z build-std=core
```

#### Step 4: Run Elfloader-Builder

```bash
cd /Users/amaterasu/Vibranium/kaal

./target/release/kaal-elfloader-builder \
  --loader runtime/elfloader/target/aarch64-unknown-none/release/libkaal_elfloader.a \
  --kernel examples/my-kaal-system/build/kernel.elf \
  --app /tmp/minimal-root-task/target/aarch64-unknown-none/release/libminimal_root_task.a \
  --out bootimage.elf
```

#### Step 5: Test in QEMU

```bash
qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a53 \
  -m 512M \
  -nographic \
  -kernel bootimage.elf
```

## Current Limitation

The elfloader-builder currently outputs a `.payload` file rather than directly patching the elfloader binary. The final step (embedding payload into ELF) needs to be implemented.

**Workaround**: For now, the payload can be linked at build time into the elfloader.

## Next Steps

1. **Complete elfloader-builder** - Implement ELF patching to embed payload
2. **Create proper root task** - Use actual KaaL root-task with cap_broker
3. **Add to CI/CD** - Automate bootable image builds
4. **Test on hardware** - Beyond QEMU testing

## Quick Test (Without Full Build)

To test if the system would work, you can check individual components:

```bash
# Check kernel
file examples/my-kaal-system/build/kernel.elf
# Should show: ELF 64-bit LSB executable, ARM aarch64

# Check builder tool
./target/release/kaal-elfloader-builder --help
# Should show CLI help

# Inspect kernel details
readelf -h examples/my-kaal-system/build/kernel.elf
```

## Summary

**We're close but not quite there yet**. The pieces are:

| Component | Status |
|-----------|--------|
| seL4 Kernel | ✅ Built |
| Elfloader Source | ✅ Complete |
| Elfloader Binary | ❌ Needs cross-compile |
| Root Task | ❌ Needs creation |
| Builder Tool | ✅ Built (macOS) |
| Final Integration | ❌ Needs ELF patching |

**Best path forward**: Use Docker to build everything in a single reproducible step, or wait until the elfloader-builder can properly embed payloads into the ELF binary.
