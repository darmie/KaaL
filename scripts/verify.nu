#!/usr/bin/env nu
# Verify all verified modules in the kernel using Verus

# Verify a single module and return success status
def verify-module [
    verus_bin: string,  # Path to verus binary
    file: string,       # Module file path (relative to project root)
    name: string,       # Display name
    items: int,         # Number of verified items
    --details: string   # Optional details to show on success
] {
    print $"📦 Verifying ($name)..."
    let result = (run-external $verus_bin $file | complete)
    let ok = ($result.exit_code == 0)

    if $ok {
        print $"  ✅ ($name): ($items) verified, 0 errors"
        if ($details | is-not-empty) {
            print $"     ($details)"
        }
    } else {
        print $"  ❌ ($name): verification failed"
        print $result.stderr
    }
    print ""

    {ok: $ok, items: $items}
}

def main [] {
    let verus_bin = $"($env.HOME)/verus/verus"

    if not ($verus_bin | path exists) {
        print "Error: Verus not found at" $verus_bin
        print "Please install Verus first. See docs/verification/SETUP.md"
        exit 1
    }

    print "🔍 Running Verus verification..."
    print ""

    # Verify each module using the factory function
    let bitmap = (verify-module $verus_bin "kernel/src/verified/bitmap_simple.rs" "bitmap_simple" 3)

    let phys_addr = (verify-module $verus_bin "kernel/src/verified/phys_addr.rs" "phys_addr" 10
        --details "Functions: new, as_usize, is_aligned, align_down, align_up, page_number, is_null")

    # Future verified modules:
    # let virt_addr = (verify-module $verus_bin "kernel/src/verified/virt_addr.rs" "virt_addr" 10)
    # let frame_alloc = (verify-module $verus_bin "kernel/src/verified/frame_allocator.rs" "frame_allocator" 15)

    # Calculate summary
    let results = [$bitmap, $phys_addr]
    let all_ok = ($results | all {|r| $r.ok})
    let total_items = ($results | each {|r| $r.items} | math sum)
    let total_modules = ($results | length)
    let passed_modules = ($results | where ok == true | length)

    # Print summary
    print "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    if $all_ok {
        print $"✓ All verification passed: ($total_items) items verified, 0 errors"
    } else {
        print $"✗ Some verification failed: ($passed_modules)/($total_modules) modules passed"
    }
    print ""
    print $"📊 Verified modules: ($passed_modules)/($total_modules)"
    print "   - bitmap_simple.rs (simple bitmap operations)"
    print "   - phys_addr.rs (physical address operations)"
}
