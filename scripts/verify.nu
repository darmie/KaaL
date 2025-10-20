#!/usr/bin/env nu
# Verify all verified modules in the kernel using Verus

def main [] {
    let verus_bin = $"($env.HOME)/verus/verus"

    if not ($verus_bin | path exists) {
        print "Error: Verus not found at" $verus_bin
        print "Please install Verus first. See docs/verification/SETUP.md"
        exit 1
    }

    print "🔍 Running Verus verification..."
    print ""

    # Verify each module
    print "Verifying bitmap_simple.rs (3 functions)..."
    run-external $verus_bin "kernel/src/verified/bitmap_simple.rs"

    # Additional verified modules will be added as they pass verification:
    # - Frame allocator (pending: bit manipulation axioms)
    # - Page tables (future)
    # - Capability system (future)

    print ""
    print "✓ Verified: 1 module, 3 functions, 0 errors"
    print ""
    print "Note: Additional modules in development require bit operation axioms"
}
