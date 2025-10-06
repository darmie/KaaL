#!/bin/bash
# Run seL4 kernel in QEMU

set -e

echo "╔════════════════════════════════════════════════════════╗"
echo "║   seL4 Kernel QEMU Test                                ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Check for kernel
if [ ! -f build/kernel.elf ]; then
    echo "❌ Kernel not found at build/kernel.elf"
    echo "   Extract it from Docker first:"
    echo "   docker run --rm kaal-dev cat /opt/seL4/build/kernel.elf > build/kernel.elf"
    exit 1
fi

echo "✅ Kernel found: build/kernel.elf"
echo "   Size: $(ls -lh build/kernel.elf | awk '{print $5}')"
echo ""
echo "🚀 Starting seL4 kernel in QEMU..."
echo "   Platform: qemu-arm-virt (ARM Cortex-A53)"
echo "   Press Ctrl+A then X to exit"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Run QEMU
qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a53 \
    -nographic \
    -m 512M \
    -kernel build/kernel.elf

echo ""
echo "✅ QEMU session ended"
