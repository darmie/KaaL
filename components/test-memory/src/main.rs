//! Memory Management and IRQ Syscall Tests
//!
//! Tests for:
//! - SYS_MEMORY_REMAP and SYS_MEMORY_SHARE syscalls
//! - SYS_IRQ_HANDLER_GET and SYS_IRQ_HANDLER_ACK syscalls
//!
//! Test Plan:
//! 1. Memory Remap - Change permissions on existing mappings
//! 2. Memory Share - Share memory between processes (requires additional process)
//! 3. IRQ Handler Get - Allocate IRQ handler capability (requires IRQControl)
//! 4. IRQ Handler Ack - Acknowledge IRQ and re-enable

#![no_std]
#![no_main]

use kaal_sdk::{printf, syscall};

const PAGE_SIZE: usize = 4096;

// Syscall numbers (for memory tests that need raw syscalls)
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

    // Test 2: IRQ Capabilities
    test_irq_capabilities();

    printf!("\n");
    printf!("===========================================\n");
    printf!("  All Tests Complete\n");
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

/// Test 2: IRQ Capability Tests
/// Note: This test requires IRQControl capability to be passed to this component
/// For now, it tests error handling when IRQControl is not available
fn test_irq_capabilities() {
    printf!("\n");
    printf!("Test 2: IRQ Capability System\n");
    printf!("------------------------------------------\n");

    // Test 2a: Create notification for IRQ signaling
    printf!("Test 2a: Create notification capability\n");
    let notification_cap = match syscall::notification_create() {
        Ok(cap) => cap,
        Err(_) => {
            printf!("  ✗ FAIL: notification_create failed\n");
            return;
        }
    };
    printf!("  ✓ PASS: Notification created at slot {}\n", notification_cap);

    // Test 2b: Allocate slot for IRQHandler
    printf!("Test 2b: Allocate capability slot for IRQHandler\n");
    let irq_handler_slot = match syscall::cap_allocate() {
        Ok(slot) => slot,
        Err(_) => {
            printf!("  ✗ FAIL: cap_allocate failed\n");
            return;
        }
    };
    printf!("  ✓ PASS: Allocated slot {}\n", irq_handler_slot);

    // Test 2c: Try to allocate IRQ handler WITHOUT IRQControl capability
    // This should fail since test-memory doesn't have IRQControl
    printf!("Test 2c: Try IRQ handler allocation without IRQControl (should fail)\n");

    // IRQControl would be in slot 0 if we had it, but we don't
    let fake_irq_control_slot = 0;
    let test_irq = 100; // Some arbitrary IRQ number

    let result = syscall::irq_handler_get(
        fake_irq_control_slot,
        test_irq,
        notification_cap,
        irq_handler_slot
    );

    if result.is_err() {
        printf!("  ✓ PASS: Correctly rejected (no IRQControl capability)\n");
        printf!("  This is expected - only root-task has IRQControl\n");
    } else {
        printf!("  ✗ FAIL: Should have rejected request\n");
        return;
    }

    // Test 2d: Try to ACK non-existent IRQ handler
    printf!("Test 2d: Try to ACK non-existent IRQ handler (should fail)\n");
    let result = syscall::irq_handler_ack(irq_handler_slot);
    if result.is_err() {
        printf!("  ✓ PASS: Correctly rejected (no IRQHandler in slot)\n");
    } else {
        printf!("  ✗ FAIL: Should have rejected ACK\n");
        return;
    }

    printf!("\n");
    printf!("✓ Test 2: IRQ Capability System - ALL TESTS PASSED\n");
    printf!("\n");
    printf!("Note: Full IRQ functionality test would require:\n");
    printf!("  1. IRQControl capability (root-task only)\n");
    printf!("  2. Valid hardware IRQ to bind\n");
    printf!("  3. Device driver to trigger interrupt\n");
    printf!("  These tests verify the syscall interface works correctly.\n");
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
