#!/bin/bash
#
# Build using seL4's official build system with our Rust root task
# This is the CORRECT way to integrate kernel + rootserver
#

set -e

echo "Building seL4 system with KaaL Rust root task..."
echo

# Step 1: Build our Rust root task
echo "========================================="
echo "Step 1: Building Rust root task"
echo "========================================="
cd examples/bootable-demo
~/.cargo/bin/cargo +nightly build --release \
    --target aarch64-unknown-none \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem

# Link into executable
aarch64-linux-gnu-ld -T ../../tools/roottask.ld \
    --whole-archive target/aarch64-unknown-none/release/libkaal_bootable_demo.a --no-whole-archive \
    -o roottask.elf

echo "✓ Root task ELF: $(file roottask.elf)"
cd ../..

# Step 2: Set up seL4 project structure
echo
echo "========================================="
echo "Step 2: Setting up seL4 project"
echo "========================================="
mkdir -p build/sel4-integrated
cd build/sel4-integrated

# Clone necessary repositories if not present
if [ ! -d "seL4" ]; then
    echo "Cloning seL4 kernel..."
    git clone --depth 1 --branch 13.0.0 https://github.com/seL4/seL4.git
fi

if [ ! -d "seL4_tools" ]; then
    echo "Cloning seL4 tools (includes elfloader)..."
    git clone --depth 1 --branch 13.0.x-compatible https://github.com/seL4/seL4_tools.git
fi

# Step 3: Create CMakeLists.txt that uses DeclareRootserver
echo
echo "========================================="
echo "Step 3: Creating CMake configuration"
echo "========================================="

cat > CMakeLists.txt << 'EOF'
cmake_minimum_required(VERSION 3.12)

project(kaal-sel4-system C ASM)

# Include seL4 CMake modules
set(SEL4_DIR "${CMAKE_CURRENT_SOURCE_DIR}/seL4" CACHE STRING "")
set(NANOPB_SRC_ROOT_FOLDER "${CMAKE_CURRENT_SOURCE_DIR}/nanopb" CACHE STRING "")
set(CMAKE_TOOLCHAIN_FILE "${SEL4_DIR}/gcc.cmake" CACHE STRING "")

# Platform configuration
set(KernelPlatform "qemu-arm-virt" CACHE STRING "")
set(KernelSel4Arch "aarch64" CACHE STRING "")
set(KernelARMPlatform "qemu-arm-virt" CACHE STRING "")
set(KernelDebugBuild ON CACHE BOOL "")
set(KernelPrinting ON CACHE BOOL "")

# Include seL4 kernel
add_subdirectory(seL4 sel4_kernel)

# Include seL4 tools (elfloader, etc)
add_subdirectory(seL4_tools/elfloader-tool elfloader)
add_subdirectory(seL4_tools/cmake-tool/helpers)

# Import our pre-built Rust root task
add_executable(kaal_roottask IMPORTED GLOBAL)
set_target_properties(kaal_roottask PROPERTIES
    IMPORTED_LOCATION "${CMAKE_CURRENT_SOURCE_DIR}/../../examples/bootable-demo/roottask.elf"
)

# THIS IS THE KEY: Declare our root task as the rootserver
DeclareRootserver(kaal_roottask)

# The build system will now create a bootable image with:
# - seL4 kernel configured for our root task
# - seL4's elfloader
# - Our Rust root task
EOF

echo "✓ CMakeLists.txt created"

# Step 4: Configure and build
echo
echo "========================================="
echo "Step 4: Configuring build with CMake"
echo "========================================="
mkdir -p build
cd build

cmake -G Ninja \
    -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
    -DCMAKE_BUILD_TYPE=Debug \
    ..

echo
echo "========================================="
echo "Step 5: Building bootable image"
echo "========================================="
ninja

echo
echo "✓ Build complete!"
echo
echo "Bootable image should be in build/images/"
ls -lh images/ 2>/dev/null || echo "Note: Check build output for image location"
