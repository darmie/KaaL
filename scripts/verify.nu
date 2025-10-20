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
    print "Verifying bitmap_simple.rs..."
    run-external $verus_bin "kernel/src/verified/bitmap_simple.rs"

    print ""
    print "Verifying frame_allocator.rs..."
    run-external $verus_bin "kernel/src/verified/frame_allocator.rs"

    # Add more modules as they're created:
    # print ""
    # print "Verifying page tables..."
    # run-external $verus_bin "kernel/src/verified/pagetable.rs"

    print ""
    print "✓ All verifications passed!"
}
