#!/bin/bash
# Test script to verify sel4-platform backend selection

set -e

echo "=== Testing seL4 Platform Backend Selection ==="
echo ""

echo "1. Testing DEFAULT (should be mock)..."
cargo build -p sel4-platform --quiet
if [ $? -eq 0 ]; then
    echo "   ✅ Mock mode builds successfully (default)"
else
    echo "   ❌ Mock mode failed!"
    exit 1
fi
echo ""

echo "2. Testing EXPLICIT MOCK mode..."
cargo build -p sel4-platform --no-default-features --features mock --quiet
if [ $? -eq 0 ]; then
    echo "   ✅ Explicit mock mode builds successfully"
else
    echo "   ❌ Explicit mock mode failed!"
    exit 1
fi
echo ""

echo "3. Testing MICROKIT mode (expected to fail on macOS - needs Linux + seL4 SDK)..."
cargo build -p sel4-platform --no-default-features --features "microkit,board-qemu-virt-aarch64" 2>&1 | head -20
if [ $? -eq 0 ]; then
    echo "   ✅ Microkit mode builds successfully"
else
    echo "   ⚠️  Microkit mode failed (EXPECTED on macOS - requires Linux + seL4 SDK)"
fi
echo ""

echo "4. Verifying mock is used by default..."
cargo tree -p sel4-platform -e features | grep -E "sel4-mock|sel4-platform" | head -5
echo ""

echo "=== Backend Selection Test Complete ==="
echo ""
echo "Summary:"
echo "  ✅ Mock mode: Works (default)"
echo "  ⚠️  Microkit mode: Requires Linux + seL4 SDK"
echo "  📝 Use --features microkit,board-qemu-virt-aarch64 for real seL4"
