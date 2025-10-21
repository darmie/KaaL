//! Memory Management Syscall Tests
//!
//! Tests for SYS_MEMORY_REMAP and SYS_MEMORY_SHARE syscalls.
//!
//! Test Plan:
//! 1. Memory Remap - Change permissions on existing mappings
//! 2. Memory Share - Share memory between processes (requires additional process)

#![no_std]
#![no_main]

use kaal_sdk::{printf, syscall};

const PAGE_SIZE: usize = 4096;

// Syscall numbers
const SYS_MEMORY_ALLOCATE: u64 = 0x11;
const SYS_MEMORY_MAP: u64 = 0x15;
const SYS_MEMORY_REMAP: u64 = 0x24;

// Memory permission flags
const PERM_READ: u64 = 0x1;
const PERM_WRITE: u64 = 0x2;
const PERM_EXEC: u64 = 0x4;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    printf!("\n");
    printf!("===========================================\n");
    printf!("  Memory Management Syscall Tests\n");
    printf!("===========================================\n");
    printf!("\n");

    // Test 1: Memory Remap
    test_memory_remap();

    printf!("\n");
    printf!("===========================================\n");
    printf!("  All Memory Tests Complete\n");
    printf!("===========================================\n");
    printf!("\n");

    // Yield forever
    loop {
        syscall::yield_now();
    }
}

/// Test 1: SYS_MEMORY_REMAP - Change permissions on existing mappings
fn test_memory_remap() {
    printf!("Test 1: Memory Remap (Change Permissions)\n");
    printf!("------------------------------------------\n");

    // Test 1a: Allocate and map a page with read-write permissions
    printf!("Test 1a: Allocate and map page (RW)\n");
    let phys_addr = syscall_memory_allocate(PAGE_SIZE as u64);
    if phys_addr == u64::MAX {
        printf!("  ✗ FAIL: memory_allocate failed\n");
        return;
    }
    printf!("  Allocated phys_addr = 0x{:x}\n", phys_addr);

    let virt_addr = syscall_memory_map(phys_addr, PAGE_SIZE as u64, PERM_READ | PERM_WRITE);
    if virt_addr == u64::MAX {
        printf!("  ✗ FAIL: memory_map failed\n");
        return;
    }
    printf!("  Mapped to virt_addr = 0x{:x}\n", virt_addr);

    // Test 1b: Write to the page to verify write permission
    printf!("Test 1b: Write to page (should succeed)\n");
    unsafe {
        let ptr = virt_addr as *mut u64;
        *ptr = 0xDEADBEEF;
        let value = *ptr;
        if value == 0xDEADBEEF {
            printf!("  ✓ PASS: Write succeeded (value = 0x{:x})\n", value);
        } else {
            printf!("  ✗ FAIL: Write verification failed\n");
            return;
        }
    }

    // Test 1c: Change permissions to read-only (remove write permission)
    printf!("Test 1c: Remap page to read-only\n");
    let result = syscall_memory_remap(virt_addr, PAGE_SIZE as u64, PERM_READ);
    if result == 0 {
        printf!("  ✓ PASS: memory_remap succeeded\n");
    } else {
        printf!("  ✗ FAIL: memory_remap failed (result = 0x{:x})\n", result);
        return;
    }

    // Test 1d: Read from the page (should still work)
    printf!("Test 1d: Read from page (should succeed)\n");
    unsafe {
        let ptr = virt_addr as *const u64;
        let value = *ptr;
        if value == 0xDEADBEEF {
            printf!("  ✓ PASS: Read succeeded (value = 0x{:x})\n", value);
        } else {
            printf!("  ✗ FAIL: Read verification failed\n");
            return;
        }
    }

    // Test 1e: Make page inaccessible (no permissions)
    printf!("Test 1e: Remap page to no permissions\n");
    let result = syscall_memory_remap(virt_addr, PAGE_SIZE as u64, 0);
    if result == 0 {
        printf!("  ✓ PASS: memory_remap to no-access succeeded\n");
    } else {
        printf!("  ✗ FAIL: memory_remap to no-access failed\n");
        return;
    }

    // Note: We can't test that access now fails because that would cause a page fault
    // In a real system, we'd need exception handling to verify this

    // Test 1f: Restore read-write permissions
    printf!("Test 1f: Remap page back to read-write\n");
    let result = syscall_memory_remap(virt_addr, PAGE_SIZE as u64, PERM_READ | PERM_WRITE);
    if result == 0 {
        printf!("  ✓ PASS: memory_remap to RW succeeded\n");
    } else {
        printf!("  ✗ FAIL: memory_remap to RW failed\n");
        return;
    }

    // Test 1g: Verify we can write again
    printf!("Test 1g: Write to page again (should succeed)\n");
    unsafe {
        let ptr = virt_addr as *mut u64;
        *ptr = 0xCAFEBABE;
        let value = *ptr;
        if value == 0xCAFEBABE {
            printf!("  ✓ PASS: Write succeeded after remap (value = 0x{:x})\n", value);
        } else {
            printf!("  ✗ FAIL: Write verification failed after remap\n");
            return;
        }
    }

    printf!("\n");
    printf!("✓ Test 1: Memory Remap - ALL TESTS PASSED\n");
}

// =============================================================================
// Raw Syscall Wrappers (inline assembly)
// =============================================================================

#[inline(always)]
fn syscall_memory_allocate(size: u64) -> u64 {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "mov x0, {size}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) SYS_MEMORY_ALLOCATE,
            size = in(reg) size,
            result = out(reg) result,
            out("x8") _,
            out("x0") _,
        );
    }
    result
}

#[inline(always)]
fn syscall_memory_map(phys_addr: u64, size: u64, permissions: u64) -> u64 {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "mov x0, {phys}",
            "mov x1, {size}",
            "mov x2, {perms}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) SYS_MEMORY_MAP,
            phys = in(reg) phys_addr,
            size = in(reg) size,
            perms = in(reg) permissions,
            result = out(reg) result,
            out("x8") _,
            out("x0") _,
            out("x1") _,
            out("x2") _,
        );
    }
    result
}

#[inline(always)]
fn syscall_memory_remap(virt_addr: u64, size: u64, new_permissions: u64) -> u64 {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "mov x0, {virt}",
            "mov x1, {size}",
            "mov x2, {perms}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) SYS_MEMORY_REMAP,
            virt = in(reg) virt_addr,
            size = in(reg) size,
            perms = in(reg) new_permissions,
            result = out(reg) result,
            out("x8") _,
            out("x0") _,
            out("x1") _,
            out("x2") _,
        );
    }
    result
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    printf!("[test-memory] PANIC: {}\n", _info);
    loop {
        syscall::yield_now();
    }
}
