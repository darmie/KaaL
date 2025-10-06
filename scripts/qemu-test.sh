#!/bin/bash
# Run KaaL in QEMU (native macOS)

set -e

echo "╔════════════════════════════════════════════════════════╗"
echo "║   KaaL QEMU Test                                       ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Find binary (check both release and debug builds)
BINARY=""
if [ -f target/release/kaal-root-task ]; then
    BINARY="target/release/kaal-root-task"
elif [ -f target/debug/kaal-root-task ]; then
    BINARY="target/debug/kaal-root-task"
else
    echo "❌ Binary not found in target/release or target/debug"
    echo "   Build KaaL first using any method:"
    echo "   - Docker: ./scripts/docker-build.sh"
    echo "   - Native: cargo build --release"
    exit 1
fi

# Check if QEMU is installed natively
if ! command -v qemu-system-aarch64 &> /dev/null; then
    echo "📦 Installing QEMU with Homebrew..."
    brew install qemu
fi

echo "✅ Binary ready: $BINARY"
echo "✅ QEMU installed"
echo ""
echo "🖥️  Starting QEMU (press Ctrl+A then X to exit)..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Run in QEMU (native macOS)
qemu-system-aarch64 \
    -machine virt,virtualization=on \
    -cpu cortex-a53 \
    -nographic \
    -m 2G \
    -kernel "$BINARY"

echo ""
echo "✅ QEMU session ended"
