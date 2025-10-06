#!/bin/bash
# Build KaaL with Mock seL4 (Unit Testing Only)

set -e

echo "🧪 Building KaaL with Mock seL4..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "⚠️  Mock mode is for UNIT TESTING only."
echo "    For development, use:"
echo "      - Microkit mode (default): cargo build"
echo "      - Runtime mode: ./scripts/build-runtime.sh"
echo ""
echo "Mock mode provides:"
echo "  ✓ Fast unit tests"
echo "  ✓ Native host execution"
echo "  ✓ Easy debugging"
echo "  ✗ No real seL4 capabilities"
echo ""

# Build tests with mock features
cargo test --features mock-sel4 --no-run "$@"

echo ""
echo "✅ Test build complete!"
echo "   Mode: Mock seL4 (testing)"
echo ""
echo "Run tests:"
echo "  cargo test --features mock-sel4"
echo ""
echo "For real development:"
echo "  ./scripts/build-microkit.sh    # Production (default)"
echo "  ./scripts/build-runtime.sh     # Advanced"
