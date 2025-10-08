cmake_minimum_required(VERSION 3.7.2)

project(sel4test-driver C ASM)

# Find required seL4 packages
find_package(seL4 REQUIRED)
find_package(elfloader-tool REQUIRED)
find_package(musllibc REQUIRED)
find_package(sel4muslcsys REQUIRED)
find_package(sel4platsupport REQUIRED)
find_package(sel4utils REQUIRED)
find_package(sel4debug REQUIRED)

# Path to KaaL Rust static library
set(KAAL_RUST_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../target/aarch64-unknown-none/release/libmy_kaal_system.a")

# Create wrapper executable
add_executable(sel4test-driver
    main.c
)

target_link_libraries(sel4test-driver
    PRIVATE
    ${KAAL_RUST_LIB}
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

# Generate simulation script
include(simulation)
GenerateSimulateScript()
