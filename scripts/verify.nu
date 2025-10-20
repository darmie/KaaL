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
    let result = (run-external $verus_bin "--crate-type=lib" $file | complete)
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

    let virt_addr = (verify-module $verus_bin "kernel/src/verified/virt_addr.rs" "virt_addr" 10
        --details "Functions: new, as_usize, is_aligned, align_down, align_up, page_number, is_null")

    let pfn = (verify-module $verus_bin "kernel/src/verified/page_frame_number.rs" "page_frame_number" 5
        --details "Functions: new, as_usize, phys_addr, from_phys_addr")

    let cap_rights = (verify-module $verus_bin "kernel/src/verified/cap_rights.rs" "cap_rights" 4
        --details "Functions: empty, contains, get_bits; Constants: READ, WRITE, GRANT, ALL")

    let bitmap_prod = (verify-module $verus_bin "kernel/src/verified/bitmap_prod.rs" "bitmap_prod" 12
        --details "Functions: new, is_set, set, clear, find_first_unset; Frame conditions with old(); Loop invariants")

    let tcb = (verify-module $verus_bin "kernel/src/verified/tcb.rs" "tcb" 29
        --details "TCB state machine, capability checking, time slice management; State transitions verified")

    let cnode_ops = (verify-module $verus_bin "kernel/src/verified/cnode_ops.rs" "cnode_ops" 6
        --details "CNode slot operations: num_slots, is_valid_index, size validation")

    let capability_ops = (verify-module $verus_bin "kernel/src/verified/capability_ops.rs" "capability_ops" 10
        --details "Capability derivation, rights checking, union/intersection operations")

    let page_table_ops = (verify-module $verus_bin "kernel/src/verified/page_table_ops.rs" "page_table_ops" 7
        --details "PageTableLevel operations: shift, block_size, index, supports_blocks, next")

    let thread_queue_ops = (verify-module $verus_bin "kernel/src/verified/thread_queue_ops.rs" "thread_queue_ops" 19
        --details "ThreadQueue and Endpoint operations: enqueue, dequeue, queue state, FIFO properties")

    let scheduler_ops = (verify-module $verus_bin "kernel/src/verified/scheduler_ops.rs" "scheduler_ops" 21
        --details "Scheduler operations: priority bitmap, O(1) priority lookup, leading_zeros optimization")

    let invocation_ops = (verify-module $verus_bin "kernel/src/verified/invocation_ops.rs" "invocation_ops" 53
        --details "Invocation operations: argument validation, rights checking, label parsing for TCB/CNode/Endpoint")

    let frame_allocator_ops = (verify-module $verus_bin "kernel/src/verified/frame_allocator_ops.rs" "frame_allocator_ops" 15
        --details "Frame allocator operations: alloc, dealloc, add_region, reserve_region, free count tracking")

    let untyped_ops = (verify-module $verus_bin "kernel/src/verified/untyped_ops.rs" "untyped_ops" 11
        --details "Untyped memory operations: new, allocate, revoke, watermark allocator, child tracking")

    let vspace_ops = (verify-module $verus_bin "kernel/src/verified/vspace_ops.rs" "vspace_ops" 19
        --details "VSpace operations: page table walking (L0-L3), map/unmap pages (4KB/2MB/1GB), alignment checks")

    # Calculate summary
    let results = [$bitmap, $phys_addr, $virt_addr, $pfn, $cap_rights, $bitmap_prod, $tcb, $cnode_ops, $capability_ops, $page_table_ops, $thread_queue_ops, $scheduler_ops, $invocation_ops, $frame_allocator_ops, $untyped_ops, $vspace_ops]
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
    print "   - virt_addr.rs (virtual address operations)"
    print "   - page_frame_number.rs (page frame number operations)"
    print "   - cap_rights.rs (capability rights bit operations)"
    print "   - bitmap_prod.rs (production bitmap with advanced features)"
    print "   - tcb.rs (thread control block state machine)"
    print "   - cnode_ops.rs (CNode slot operations)"
    print "   - capability_ops.rs (capability derivation and rights)"
    print "   - page_table_ops.rs (page table level operations)"
    print "   - thread_queue_ops.rs (thread queue and endpoint operations)"
    print "   - scheduler_ops.rs (priority-based scheduler operations)"
    print "   - invocation_ops.rs (syscall invocation validation)"
    print "   - frame_allocator_ops.rs (frame allocator operations)"
    print "   - untyped_ops.rs (untyped memory watermark allocator)"
    print "   - vspace_ops.rs (virtual address space operations)"
}
