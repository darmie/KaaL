#!/bin/bash
set -e
cd "$(dirname "$0")"

# Build the test binary
./build-test.sh

# Run in QEMU virt machine
echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  Running Heap Allocator Tests in QEMU"
echo "═══════════════════════════════════════════════════════════"
echo ""

qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a53 \
  -m 128M \
  -nographic \
  -kernel target/aarch64-unknown-none/release/kernel-test \
  -serial mon:stdio \
  -d guest_errors
