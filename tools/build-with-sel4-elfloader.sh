#!/bin/bash
#
# Build bootable image using seL4's official C elfloader
# This uses seL4's proven boot infrastructure with our Rust root task
#

set -e

echo "Building seL4 kernel + Rust root task with official seL4 elfloader..."
echo

# Build our Rust root task
echo "Step 1: Building Rust root task..."
cd examples/bootable-demo
cargo build --release \
    --target aarch64-unknown-none \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem

# Link into ELF executable
echo "Step 2: Linking root task ELF..."
aarch64-linux-gnu-ld -T ../../tools/roottask.ld \
    --whole-archive target/aarch64-unknown-none/release/libkaal_bootable_demo.a --no-whole-archive \
    -o roottask.elf

echo "Root task ELF created: $(file roottask.elf)"
echo

# Now we need to use seL4's build system to create kernel+elfloader+roottask image
# This requires setting up a proper seL4 project structure

echo "Step 3: Creating seL4 project structure..."
cd ../..
mkdir -p build/sel4-boot
cd build/sel4-boot

# Clone seL4, seL4_tools for elfloader
if [ ! -d "seL4" ]; then
    git clone --depth 1 --branch 13.0.0 https://github.com/seL4/seL4.git
fi

if [ ! -d "seL4_tools" ]; then
    git clone --depth 1 --branch 13.0.x-compatible https://github.com/seL4/seL4_tools.git
fi

echo "✓ seL4 sources ready"
echo

echo "This approach requires full seL4 build system integration."
echo "Alternatively, we can try tweaking our Rust elfloader configuration..."
