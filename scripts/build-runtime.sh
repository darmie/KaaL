#!/bin/bash
# Build KaaL with Rust seL4 Runtime (Advanced Mode)

set -e

echo "🚀 Building KaaL with Rust seL4 Runtime..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check if Rust seL4 runtime is available
if ! cargo tree --no-default-features --features sel4-runtime-mode 2>/dev/null | grep -q "sel4-runtime"; then
    echo "⚠️  Rust seL4 Runtime not found. Setting up..."
    echo ""
    echo "The Rust seL4 Runtime requires:"
    echo "  1. seL4 kernel build"
    echo "  2. Custom target specification"
    echo "  3. seL4 runtime library"
    echo ""
    echo "This is an advanced mode. For most users, we recommend:"
    echo "  - Microkit mode: ./scripts/build-microkit.sh"
    echo ""
    read -p "Continue with runtime setup? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Build with runtime feature
echo "Building with Rust seL4 Runtime..."
cargo build --no-default-features --features sel4-runtime-mode "$@"

echo ""
echo "✅ Build complete!"
echo "   Mode: Rust seL4 Runtime"
echo "   Target: $(rustc -vV | grep host | cut -d' ' -f2)"
echo ""
echo "Next steps:"
echo "  1. Configure seL4 kernel build"
echo "  2. Create root task entry point"
echo "  3. Link with seL4 runtime"
echo "  4. Run: qemu-system-<arch> -kernel kaal-image"
