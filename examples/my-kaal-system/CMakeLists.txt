cmake_minimum_required(VERSION 3.7.2)

# Standard seL4 project setup
set(CMAKE_MODULE_PATH "${CMAKE_SOURCE_DIR}/../../tools/cmake-tool/helpers" ${CMAKE_MODULE_PATH})

# Include seL4 build system helpers
include(application_settings)

# Project declaration
project(kaal-system C ASM)

# Find seL4 components
find_package(seL4 REQUIRED)
find_package(elfloader-tool REQUIRED)
find_package(musllibc REQUIRED)
find_package(util_libs REQUIRED)
find_package(seL4_libs REQUIRED)

# Set up seL4 environment
sel4_import_kernel()
elfloader_import_project()

# KaaL root task binary (pre-built by Cargo)
set(KAAL_ROOTTASK_BINARY "${CMAKE_CURRENT_SOURCE_DIR}/target/aarch64-unknown-none/release/my-kaal-system")

# Verify the Rust binary exists
if(NOT EXISTS "${KAAL_ROOTTASK_BINARY}")
    message(FATAL_ERROR "KaaL root task binary not found: ${KAAL_ROOTTASK_BINARY}\nRun 'cargo build --release' first")
endif()

# Import the pre-built Rust binary as an executable target
add_executable(roottask IMPORTED GLOBAL)
set_target_properties(roottask PROPERTIES
    IMPORTED_LOCATION "${KAAL_ROOTTASK_BINARY}"
)

# Declare this as the root server
include(rootserver)
DeclareRootserver(roottask)
