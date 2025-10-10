cmake_minimum_required(VERSION 3.12)

# KaaL Bootable Demo - seL4 Kernel + Rust Root Task
project(kaal-bootable-demo C ASM)

# Find seL4 kernel
set(SEL4_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../external/seL4" CACHE PATH "Path to seL4 kernel")

# Include seL4 CMake helpers
include(${SEL4_ROOT}/configs/seL4Config.cmake)

# Configure kernel for ARM64 QEMU virt platform
set(KernelPlatform "qemu-arm-virt" CACHE STRING "seL4 platform")
set(KernelSel4Arch "aarch64" CACHE STRING "seL4 architecture")
set(KernelARMPlatform "qemu-arm-virt" CACHE STRING "ARM platform")
set(KernelDebugBuild OFF CACHE BOOL "Enable kernel debugging")
set(KernelPrinting ON CACHE BOOL "Enable kernel printing")
set(KernelVerificationBuild OFF CACHE BOOL "Disable verification build for faster compilation")

# Build the seL4 kernel
add_subdirectory(${SEL4_ROOT} sel4_kernel)

# Custom target to build Rust root task
add_custom_command(
    OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/libkaal_bootable_demo.a
    COMMAND cargo build --release
        --target aarch64-unknown-none
        -Z build-std=core,alloc
        -Z build-std-features=compiler-builtins-mem
    COMMAND ${CMAKE_COMMAND} -E copy
        ${CMAKE_CURRENT_SOURCE_DIR}/target/aarch64-unknown-none/release/libkaal_bootable_demo.a
        ${CMAKE_CURRENT_BINARY_DIR}/libkaal_bootable_demo.a
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    COMMENT "Building Rust root task"
    VERBATIM
)

add_custom_target(rust_roottask ALL
    DEPENDS ${CMAKE_CURRENT_BINARY_DIR}/libkaal_bootable_demo.a
)

# Create root task executable from Rust static library
add_executable(roottask IMPORTED GLOBAL)
set_target_properties(roottask PROPERTIES
    IMPORTED_LOCATION ${CMAKE_CURRENT_BINARY_DIR}/libkaal_bootable_demo.a
)
add_dependencies(roottask rust_roottask)

# Declare this as the root server for seL4
# This tells seL4's build system to link the kernel with this root task
include(${SEL4_ROOT}/tools/cmake-tool/helpers/rootserver.cmake)
DeclareRootserver(roottask)

# Set linker script for root task
set_target_properties(roottask PROPERTIES
    LINK_FLAGS "-T ${CMAKE_CURRENT_SOURCE_DIR}/../../tools/roottask.ld"
)
