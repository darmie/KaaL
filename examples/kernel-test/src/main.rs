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

    // Initialize heap allocator
    unsafe {
        kaal_kernel::memory::heap::init();
    }
    kprintln!("Heap initialized with {} bytes free", kaal_kernel::memory::heap::free_memory());
    kprintln!("");

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
    kprintln!("  Test Results: {} passed, {} failed", passed, failed);
    kprintln!("═══════════════════════════════════════════════════════════");

    if failed == 0 {
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
