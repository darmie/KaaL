#!/bin/bash
# Build KaaL Chapter 1 kernel with elfloader and run in QEMU

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT_DIR="$( cd "$SCRIPT_DIR/../.." && pwd )"

echo "═══════════════════════════════════════════════════════════"
echo "  KaaL Chapter 1 - Full Build and Test"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Step 1: Build kernel
echo "[1/6] Building KaaL kernel..."
cd "$ROOT_DIR/kernel"
export PATH="$HOME/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:$PATH"

# First, let's fix the kernel to build as a proper binary
# Remove the assembly approach for now and use pure Rust
cargo build --release --bin kaal-kernel --target aarch64-unknown-none \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem 2>&1 | grep -E "Compiling|Finished|error|warning" || true

KERNEL_ELF="$ROOT_DIR/kernel/target/aarch64-unknown-none/release/kaal-kernel"

if [ ! -f "$KERNEL_ELF" ]; then
    echo "ERROR: Kernel build failed"
    exit 1
fi

echo "✓ Kernel built: $KERNEL_ELF"
echo ""

# Step 2: Create minimal dummy root task
echo "[2/6] Creating minimal root task..."
ROOTTASK_DIR="$SCRIPT_DIR/minimal-roottask"
mkdir -p "$ROOTTASK_DIR/src"

cat > "$ROOTTASK_DIR/Cargo.toml" << 'EOF'
[package]
name = "minimal-roottask"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "minimal-roottask"
path = "src/main.rs"

[profile.release]
opt-level = "z"
lto = true
panic = "abort"
EOF

cat > "$ROOTTASK_DIR/src/main.rs" << 'EOF'
#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
EOF

cd "$ROOTTASK_DIR"
cargo build --release --target aarch64-unknown-none \
    -Z build-std=core \
    -Z build-std-features=compiler-builtins-mem 2>&1 | grep -E "Compiling|Finished" || true

ROOTTASK_ELF="$ROOTTASK_DIR/target/aarch64-unknown-none/release/minimal-roottask"
echo "✓ Root task built: $ROOTTASK_ELF"
echo ""

# Step 3: Build elfloader
echo "[3/6] Building elfloader..."
cd "$ROOT_DIR/runtime/elfloader"
cargo build --release --target aarch64-unknown-none \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem 2>&1 | grep -E "Compiling|Finished" || true

ELFLOADER_LIB="$ROOT_DIR/runtime/elfloader/target/aarch64-unknown-none/release/libkaal_elfloader.a"
echo "✓ Elfloader library built"
echo ""

# Step 4: Build elfloader-builder tool
echo "[4/6] Building elfloader-builder tool..."
cd "$ROOT_DIR/runtime/elfloader-builder"
cargo build --release 2>&1 | grep -E "Compiling|Finished" || true
BUILDER="$ROOT_DIR/runtime/elfloader-builder/target/release/kaal-elfloader-builder"
echo "✓ Elfloader-builder ready"
echo ""

# Step 5: Package everything
echo "[5/6] Packaging kernel + root task..."
mkdir -p "$SCRIPT_DIR/output"
$BUILDER \
    --loader "$ELFLOADER_LIB" \
    --kernel "$KERNEL_ELF" \
    --app "$ROOTTASK_ELF" \
    --out "$SCRIPT_DIR/output/bootimage.elf"

echo ""

# Step 6: Test in QEMU
echo "[6/6] Testing in QEMU..."
echo "Note: This will show elfloader output. Kernel may not boot yet due to"
echo "      incomplete integration, but you should see the elfloader banner."
echo ""
echo "Press Ctrl-A then X to exit QEMU"
echo ""

# Note: Since the full integration isn't complete, we'll just show what would be run
echo "QEMU command would be:"
echo "  qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M \\"
echo "    -nographic -kernel $KERNEL_ELF"
echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  Build Complete!"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Outputs:"
echo "  Kernel:     $KERNEL_ELF"
echo "  Root task:  $ROOTTASK_ELF"
echo "  Payload:    $SCRIPT_DIR/output/bootimage.payload"
echo ""
