cmake_minimum_required(VERSION 3.7.2)

# Include seL4 test infrastructure (provides build targets)
include(${CMAKE_SOURCE_DIR}/projects/sel4test/settings.cmake)

# Set project name
project(kaal-wrapper C ASM)

# Find required seL4 packages
find_package(seL4 REQUIRED)
find_package(elfloader-tool REQUIRED)
find_package(musllibc REQUIRED)
find_package(sel4muslcsys REQUIRED)
find_package(sel4platsupport REQUIRED)
find_package(sel4utils REQUIRED)
find_package(sel4debug REQUIRED)

# Build KaaL Rust library
set(KAAL_RUST_BINARY "${CMAKE_CURRENT_SOURCE_DIR}/../target/aarch64-unknown-none/release/libmy-kaal-system.a")

# Create wrapper executable (replaces sel4test-driver)
add_executable(sel4test-driver
    main.c
)

target_link_libraries(sel4test-driver
    PRIVATE
    ${KAAL_RUST_BINARY}
    sel4
    muslc
    sel4muslcsys
    sel4platsupport
    sel4utils
    sel4debug
)

# Declare as root server
include(rootserver)
DeclareRootserver(sel4test-driver)

# Generate final boot image
include(simulation)
GenerateSimulateScript()
