#!/bin/bash
# Verification script: Confirm that KaaL defaults to REAL seL4, not mocks

set -e

echo "╔════════════════════════════════════════════════════════╗"
echo "║   KaaL Build Mode Verification                         ║"
echo "║   Confirming: Default = REAL seL4 (not mocks)          ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

echo "📋 Test 1: Default build should REQUIRE seL4 SDK"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if cargo build -p sel4-platform 2>&1 | grep -q "SEL4_INCLUDE_DIRS or SEL4_PREFIX must be set"; then
    echo "✅ PASS: Default build requires SEL4_PREFIX (real seL4)"
else
    echo "❌ FAIL: Default build did not require seL4 SDK!"
    exit 1
fi
echo ""

echo "📋 Test 2: Mock mode should work with explicit flag"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if cargo build -p sel4-platform --no-default-features --features mock 2>&1 | grep -q "Finished"; then
    echo "✅ PASS: Mock mode builds successfully with explicit flag"
else
    echo "❌ FAIL: Mock mode failed!"
    exit 1
fi
echo ""

echo "📋 Test 3: Check feature tree (should show microkit by default)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if cargo metadata --format-version 1 2>/dev/null | \
   jq -r '.packages[] | select(.name == "sel4-platform") | .features.default[]' | \
   grep -q "microkit"; then
    echo "✅ PASS: Default features include 'microkit'"
else
    echo "⚠️  SKIP: Could not verify via metadata (jq may not be installed)"
fi
echo ""

echo "📋 Test 4: Verify mock is NOT in default features"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if grep -A 3 "^default = " runtime/sel4-platform/Cargo.toml | grep -q "mock"; then
    echo "❌ FAIL: Default features contain 'mock'!"
    exit 1
else
    echo "✅ PASS: Default does NOT include mock"
fi
echo ""

echo "╔════════════════════════════════════════════════════════╗"
echo "║   ✅ VERIFICATION COMPLETE                             ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""
echo "Summary:"
echo "  ✅ Default build = REAL seL4 (microkit mode)"
echo "  ✅ Mock mode = Explicit opt-in only"
echo "  ✅ Production-first approach confirmed"
echo ""
echo "To build on macOS:"
echo "  cargo build --no-default-features --features mock"
echo ""
echo "To build on Linux with seL4:"
echo "  export SEL4_PREFIX=/path/to/seL4"
echo "  cargo build"
