#!/bin/bash
# Run KaaL Chapter 1 kernel in QEMU
# This directly boots the kernel without elfloader for testing

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT_DIR="$( cd "$SCRIPT_DIR/../.." && pwd )"
KERNEL_ELF="$ROOT_DIR/kernel/target/aarch64-unknown-none/release/kaal-kernel"

# Check if kernel exists
if [ ! -f "$KERNEL_ELF" ]; then
    echo "ERROR: Kernel not found at $KERNEL_ELF"
    echo "Please build the kernel first:"
    echo "  cd kernel && cargo build --release --target aarch64-unknown-none -Z build-std=core,alloc"
    exit 1
fi

echo "═══════════════════════════════════════════════════════════"
echo "  KaaL Chapter 1 Kernel - QEMU Test"
echo "═══════════════════════════════════════════════════════════"
echo "Kernel: $KERNEL_ELF"
echo ""

# Check if QEMU is available
if ! command -v qemu-system-aarch64 &> /dev/null; then
    echo "ERROR: qemu-system-aarch64 not found"
    echo "Please install QEMU for ARM64"
    exit 1
fi

echo "Starting QEMU..."
echo "Press Ctrl-A then X to exit QEMU"
echo ""

# Run QEMU with the kernel
# - Machine: virt (QEMU ARM Virtual Machine)
# - CPU: cortex-a53
# - RAM: 128M
# - Serial: stdio (output to terminal)
# - Kernel: our ELF binary
# - No graphics
qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a53 \
    -m 128M \
    -serial stdio \
    -kernel "$KERNEL_ELF" \
    -nographic \
    -d guest_errors \
    -D /tmp/qemu-kaal.log
