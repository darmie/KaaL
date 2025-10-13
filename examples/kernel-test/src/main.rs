#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;
use kaal_kernel::kprintln;

// ARM64 boot entry - setup stack and BSS, then call _start
global_asm!(
    ".section .text._boot_start",
    ".global _boot_start",
    "_boot_start:",
    "    ldr x30, =_stack_top",
    "    mov sp, x30",
    "    ldr x0, =_bss_start",
    "    ldr x1, =_bss_end",
    "    sub x1, x1, x0",
    "1:",
    "    cbz x1, 2f",
    "    str xzr, [x0], #8",
    "    sub x1, x1, #8",
    "    b 1b",
    "2:",
    "    b _start",
    "3:",
    "    wfi",
    "    b 3b",
);

/// Test runner entry point
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize console for output
    kaal_kernel::config::init_console();

    kprintln!("KaaL Kernel Object Model Test Suite");
    kprintln!("(No heap allocator - seL4 design: static allocation only)");
    kprintln!("");

    // Initialize exception handlers to catch faults
    kprintln!("[init] Installing exception handlers...");
    unsafe {
        kaal_kernel::arch::aarch64::exception::init();
    }
    kprintln!("[init] Exception handlers installed");
    kprintln!("");

    // ========================================================================
    // HEAP ALLOCATOR TESTS COMMENTED OUT FOR OBJECT MODEL TEST ISOLATION
    // ========================================================================
    /*
    kprintln!("═══════════════════════════════════════════════════════════");
    kprintln!("  KaaL Kernel Heap Allocator Unit Tests");
    kprintln!("═══════════════════════════════════════════════════════════");
    kprintln!("");

    let mut passed = 0;
    let mut failed = 0;
    let total = 8;

    // Test 1: Allocator initialization
    kprintln!("[1/{}] test_allocator_init...", total);
    if kaal_kernel::memory::heap::tests::test_allocator_init() {
        kprintln!("    ✓ PASS");
        passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        failed += 1;
    }

    // Test 2: Simple allocation
    kprintln!("[2/{}] test_simple_allocation...", total);
    if kaal_kernel::memory::heap::tests::test_simple_allocation() {
        kprintln!("    ✓ PASS");
        passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        failed += 1;
    }

    // Test 3: Multiple allocations
    kprintln!("[3/{}] test_multiple_allocations...", total);
    if kaal_kernel::memory::heap::tests::test_multiple_allocations() {
        kprintln!("    ✓ PASS");
        passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        failed += 1;
    }

    // Test 4: Allocation and deallocation
    kprintln!("[4/{}] test_allocation_and_deallocation...", total);
    if kaal_kernel::memory::heap::tests::test_allocation_and_deallocation() {
        kprintln!("    ✓ PASS");
        passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        failed += 1;
    }

    // Test 5: Out of memory handling
    kprintln!("[5/{}] test_out_of_memory...", total);
    if kaal_kernel::memory::heap::tests::test_out_of_memory() {
        kprintln!("    ✓ PASS");
        passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        failed += 1;
    }

    // Test 6: Alignment requirements
    kprintln!("[6/{}] test_alignment...", total);
    if kaal_kernel::memory::heap::tests::test_alignment() {
        kprintln!("    ✓ PASS");
        passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        failed += 1;
    }

    // Test 7: Fragmentation handling
    kprintln!("[7/{}] test_fragmentation...", total);
    if kaal_kernel::memory::heap::tests::test_fragmentation() {
        kprintln!("    ✓ PASS");
        passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        failed += 1;
    }

    // Test 8: Zero-size allocation
    kprintln!("[8/{}] test_zero_size_allocation...", total);
    if kaal_kernel::memory::heap::tests::test_zero_size_allocation() {
        kprintln!("    ✓ PASS");
        passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        failed += 1;
    }

    kprintln!("");
    kprintln!("═══════════════════════════════════════════════════════════");
    kprintln!("  Heap Test Results: {} passed, {} failed", passed, failed);
    kprintln!("═══════════════════════════════════════════════════════════");
    kprintln!("");
    */

    // Initialize variables for test tracking (heap tests skipped)
    let passed = 0;
    let failed = 0;
    let total = 0;

    // ========================================================================
    // Object Model Tests
    // ========================================================================

    kprintln!("═══════════════════════════════════════════════════════════");
    kprintln!("  KaaL Kernel Object Model Unit Tests");
    kprintln!("═══════════════════════════════════════════════════════════");
    kprintln!("");

    let mut obj_passed = 0;
    let mut obj_failed = 0;
    let obj_total = 18;

    // Capability Tests
    kprintln!("[1/{}] test_capability_creation...", obj_total);
    if kaal_kernel::objects::test_runner::test_capability_creation() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("[2/{}] test_capability_derivation...", obj_total);
    if kaal_kernel::objects::test_runner::test_capability_derivation() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("[3/{}] test_capability_minting...", obj_total);
    if kaal_kernel::objects::test_runner::test_capability_minting() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("[4/{}] test_capability_rights...", obj_total);
    if kaal_kernel::objects::test_runner::test_capability_rights() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    // CNode Tests
    kprintln!("[5/{}] test_cnode_creation...", obj_total);
    if kaal_kernel::objects::test_runner::test_cnode_creation() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("[6/{}] test_cnode_insert_lookup...", obj_total);
    if kaal_kernel::objects::test_runner::test_cnode_insert_lookup() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("[7/{}] test_cnode_copy_move...", obj_total);
    if kaal_kernel::objects::test_runner::test_cnode_copy_move() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    // TCB Tests
    kprintln!("[8/{}] test_tcb_creation...", obj_total);
    if kaal_kernel::objects::test_runner::test_tcb_creation() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("[9/{}] test_tcb_state_transitions...", obj_total);
    if kaal_kernel::objects::test_runner::test_tcb_state_transitions() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("[10/{}] test_tcb_priority...", obj_total);
    if kaal_kernel::objects::test_runner::test_tcb_priority() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    // Endpoint Tests
    kprintln!("[11/{}] test_endpoint_creation...", obj_total);
    if kaal_kernel::objects::test_runner::test_endpoint_creation() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("[12/{}] test_endpoint_queue_operations...", obj_total);
    if kaal_kernel::objects::test_runner::test_endpoint_queue_operations() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    // Untyped Tests
    kprintln!("[13/{}] test_untyped_creation...", obj_total);
    if kaal_kernel::objects::test_runner::test_untyped_creation() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("[14/{}] test_untyped_retype...", obj_total);
    if kaal_kernel::objects::test_runner::test_untyped_retype() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("[15/{}] test_untyped_revoke...", obj_total);
    if kaal_kernel::objects::test_runner::test_untyped_revoke() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    // Invocation Tests
    kprintln!("[16/{}] test_tcb_invocation_priority...", obj_total);
    if kaal_kernel::objects::test_runner::test_tcb_invocation_priority() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("[17/{}] test_invocation_rights_enforcement...", obj_total);
    if kaal_kernel::objects::test_runner::test_invocation_rights_enforcement() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    // Integration Tests
    kprintln!("[18/{}] test_capability_delegation_chain...", obj_total);
    if kaal_kernel::objects::test_runner::test_capability_delegation_chain() {
        kprintln!("    ✓ PASS");
        obj_passed += 1;
    } else {
        kprintln!("    ✗ FAIL");
        obj_failed += 1;
    }

    kprintln!("");
    kprintln!("═══════════════════════════════════════════════════════════");
    kprintln!("  Object Model Test Results: {} passed, {} failed", obj_passed, obj_failed);
    kprintln!("═══════════════════════════════════════════════════════════");
    kprintln!("");

    // Overall results
    let total_passed = passed + obj_passed;
    let total_failed = failed + obj_failed;
    let total_tests = total + obj_total;

    kprintln!("═══════════════════════════════════════════════════════════");
    kprintln!("  Overall Test Results: {} passed, {} failed out of {}",
             total_passed, total_failed, total_tests);
    kprintln!("═══════════════════════════════════════════════════════════");

    if total_failed == 0 {
        kprintln!("");
        kprintln!("All tests passed! ✓");
        kprintln!("");
    } else {
        kprintln!("");
        kprintln!("Some tests failed! ✗");
        kprintln!("");
    }

    // Halt
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprintln!("PANIC: {}", info);
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
