# KaaL - seL4 Kernel-as-a-Library Build Configuration
#
# This CMake file integrates KaaL with the seL4 build system.
#
# Usage:
#   mkdir build && cd build
#   cmake -DPLATFORM=x86_64 ..
#   make
#
# Environment Variables:
#   SEL4_DIR - Path to seL4 kernel source
#   PLATFORM - Target platform (x86_64, aarch64, etc.)
#   SIMULATION - Build for QEMU (TRUE/FALSE)

cmake_minimum_required(VERSION 3.12)

project(kaal C ASM)

# Platform configuration
if(NOT DEFINED PLATFORM)
    set(PLATFORM "x86_64" CACHE STRING "Target platform")
endif()

if(NOT DEFINED SIMULATION)
    set(SIMULATION TRUE CACHE BOOL "Build for simulation (QEMU)")
endif()

message(STATUS "Building KaaL for platform: ${PLATFORM}")
message(STATUS "Simulation mode: ${SIMULATION}")

# TODO PHASE 2: Uncomment when seL4 is available
# # Find seL4 SDK
# if(DEFINED ENV{SEL4_DIR})
#     set(SEL4_DIR $ENV{SEL4_DIR})
#     message(STATUS "Using seL4 from: ${SEL4_DIR}")
# else()
#     message(FATAL_ERROR "SEL4_DIR environment variable not set")
# endif()

# # Import seL4 build system
# include(${SEL4_DIR}/tools/seL4.cmake OPTIONAL)

# # Configure seL4 kernel
# if(PLATFORM STREQUAL "x86_64")
#     sel4_configure_platform_settings()
#     set(KernelPlatform "pc99" CACHE STRING "" FORCE)
#     set(KernelArch "x86" CACHE STRING "" FORCE)
#     set(KernelX86Sel4Arch "x86_64" CACHE STRING "" FORCE)
# elseif(PLATFORM STREQUAL "aarch64")
#     set(KernelPlatform "qemu-arm-virt" CACHE STRING "" FORCE)
#     set(KernelArch "arm" CACHE STRING "" FORCE)
#     set(KernelArmSel4Arch "aarch64" CACHE STRING "" FORCE)
# else()
#     message(FATAL_ERROR "Unsupported platform: ${PLATFORM}")
# endif()

# # Import seL4 kernel
# sel4_import_kernel()

# Determine Rust target triple
if(PLATFORM STREQUAL "x86_64")
    set(RUST_TARGET "x86_64-unknown-none")
    set(RUST_TARGET_JSON "${CMAKE_SOURCE_DIR}/x86_64-sel4.json")
elseif(PLATFORM STREQUAL "aarch64")
    set(RUST_TARGET "aarch64-unknown-none")
    set(RUST_TARGET_JSON "${CMAKE_SOURCE_DIR}/aarch64-sel4.json")
else()
    message(FATAL_ERROR "Unsupported platform: ${PLATFORM}")
endif()

# Build Rust workspace
add_custom_target(kaal_rust ALL
    COMMAND ${CMAKE_COMMAND} -E env
        RUST_TARGET_PATH=${CMAKE_SOURCE_DIR}
        cargo build
            --release
            --target ${RUST_TARGET}
            $<$<BOOL:${VERBOSE}>:--verbose>
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    COMMENT "Building Rust workspace for ${RUST_TARGET}"
    BYPRODUCTS
        ${CMAKE_SOURCE_DIR}/target/${RUST_TARGET}/release/libcap_broker.a
        ${CMAKE_SOURCE_DIR}/target/${RUST_TARGET}/release/libkaal_ipc.a
        ${CMAKE_SOURCE_DIR}/target/${RUST_TARGET}/release/libdddk_runtime.a
)

# TODO PHASE 2: Create root task executable
# add_executable(kaal_root
#     # Root task entry point (to be created)
#     src/root_task.rs
#
#     # Link with Rust libraries
#     ${CMAKE_SOURCE_DIR}/target/${RUST_TARGET}/release/libcap_broker.a
#     ${CMAKE_SOURCE_DIR}/target/${RUST_TARGET}/release/libkaal_ipc.a
#     ${CMAKE_SOURCE_DIR}/target/${RUST_TARGET}/release/libdddk_runtime.a
# )

# add_dependencies(kaal_root kaal_rust)

# target_link_libraries(kaal_root
#     sel4
#     sel4_autoconf
#     sel4runtime
# )

# # Declare as root server
# DeclareRootserver(kaal_root)

# # Generate bootable image
# if(SIMULATION)
#     # Create QEMU-bootable image
#     add_custom_command(TARGET kaal_root POST_BUILD
#         COMMAND ${CMAKE_COMMAND} -E copy
#             $<TARGET_FILE:kaal_root>
#             ${CMAKE_BINARY_DIR}/kaal-image-${PLATFORM}
#         COMMENT "Creating bootable image for ${PLATFORM}"
#     )
# endif()

# Testing (Phase 1 - native tests)
enable_testing()

add_test(
    NAME cargo_test
    COMMAND cargo test --workspace
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
)

# TODO PHASE 2: Add seL4 simulation tests
# add_test(
#     NAME qemu_test_serial
#     COMMAND ${CMAKE_SOURCE_DIR}/scripts/test-in-qemu.sh
#         ${CMAKE_BINARY_DIR}/images/kernel-${PLATFORM}
#         ${CMAKE_BINARY_DIR}/kaal-image-${PLATFORM}
#         serial_test
# )

# Documentation
add_custom_target(docs
    COMMAND cargo doc --workspace --no-deps
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    COMMENT "Generating Rust documentation"
)

# Clean target
add_custom_target(clean-all
    COMMAND cargo clean
    COMMAND ${CMAKE_COMMAND} -E remove_directory ${CMAKE_BINARY_DIR}
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    COMMENT "Cleaning all build artifacts"
)

# Install targets (for deployment)
install(
    FILES
        ${CMAKE_SOURCE_DIR}/README.md
        ${CMAKE_SOURCE_DIR}/docs/GETTING_STARTED.md
    DESTINATION share/doc/kaal
)

# Print build summary
message(STATUS "")
message(STATUS "========================================")
message(STATUS "KaaL Build Configuration")
message(STATUS "========================================")
message(STATUS "Platform: ${PLATFORM}")
message(STATUS "Rust Target: ${RUST_TARGET}")
message(STATUS "Simulation: ${SIMULATION}")
# TODO PHASE 2: Uncomment
# message(STATUS "seL4 Kernel: ${SEL4_DIR}")
message(STATUS "Build Type: ${CMAKE_BUILD_TYPE}")
message(STATUS "========================================")
message(STATUS "")
message(STATUS "Phase 1 (Current): Mock seL4 for development")
message(STATUS "Phase 2 (Future): Real seL4 integration")
message(STATUS "")
message(STATUS "Build commands:")
message(STATUS "  make               - Build Rust workspace")
message(STATUS "  make test          - Run tests")
message(STATUS "  make docs          - Generate documentation")
message(STATUS "  make clean-all     - Clean everything")
message(STATUS "")
