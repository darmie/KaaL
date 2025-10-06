#!/bin/bash
# Build KaaL with real seL4 (microkit mode)

set -e

# Point to our local seL4 kernel sources
export SEL4_PREFIX=/Users/amaterasu/Vibranium/kaal/external/seL4

echo "=== Building with REAL seL4 (Microkit Mode) ==="
echo "SEL4_PREFIX: $SEL4_PREFIX"
echo ""

# Build sel4-platform with microkit backend
cargo build -p sel4-platform \
    --no-default-features \
    --features "microkit,board-qemu-virt-aarch64" \
    "$@"
