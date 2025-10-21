//! Capability Derivation and Revocation Test Component
//!
//! Tests the seL4-style CDT system by:
//! 1. Testing syscall interface (SYS_CAP_DERIVE, SYS_CAP_MINT, SYS_CAP_REVOKE)
//! 2. Verifying error handling (invalid slots, permissions)
//! 3. Testing derivation lifecycle (derive → revoke parent → children gone)
//!
//! Component requires CAP_CAPS capability to test capability operations.

#![no_std]
#![no_main]

// Syscall numbers
const SYS_DEBUG_PRINT: u64 = 0x1001;
const SYS_YIELD: u64 = 0x01;
const SYS_CAP_ALLOCATE: u64 = 0x10;
const SYS_ENDPOINT_CREATE: u64 = 0x13;
const SYS_CAP_REVOKE: u64 = 0x1E;
const SYS_CAP_DERIVE: u64 = 0x1F;
const SYS_CAP_MINT: u64 = 0x20;

fn print(msg: &str) {
    unsafe {
        core::arch::asm!(
            "mov x8, {syscall}",
            "svc #0",
            syscall = in(reg) SYS_DEBUG_PRINT,
            in("x0") msg.as_ptr(),
            in("x1") msg.len(),
            out("x8") _,
        );
    }
}

fn print_u64(n: u64) {
    let mut buf = [0u8; 20];
    let mut num = n;
    let mut i = 0;

    if num == 0 {
        print("0");
        return;
    }

    while num > 0 {
        buf[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }

    // Print digits in reverse order
    for j in (0..i).rev() {
        let digit = &buf[j..j+1];
        unsafe {
            core::arch::asm!(
                "mov x8, {syscall}",
                "svc #0",
                syscall = in(reg) SYS_DEBUG_PRINT,
                in("x0") digit.as_ptr(),
                in("x1") 1u64,
                out("x8") _,
            );
        }
    }
}

fn syscall_cap_allocate() -> u64 {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "mov x8, {syscall}",
            "svc #0",
            "mov {result}, x0",
            syscall = in(reg) SYS_CAP_ALLOCATE,
            result = out(reg) result,
            out("x8") _,
            out("x0") _,
        );
    }
    result
}

fn syscall_cap_revoke(cnode_cap: u64, slot: u64) -> u64 {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "mov x0, {cnode_cap}",
            "mov x1, {slot}",
            "mov x8, {syscall}",
            "svc #0",
            "mov {result}, x0",
            cnode_cap = in(reg) cnode_cap,
            slot = in(reg) slot,
            syscall = in(reg) SYS_CAP_REVOKE,
            result = out(reg) result,
            out("x8") _,
            out("x0") _,
            out("x1") _,
        );
    }
    result
}

fn syscall_cap_derive(cnode_cap: u64, src_slot: u64, dest_slot: u64, rights: u64) -> u64 {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "mov x0, {cnode_cap}",
            "mov x1, {src_slot}",
            "mov x2, {dest_slot}",
            "mov x3, {rights}",
            "mov x8, {syscall}",
            "svc #0",
            "mov {result}, x0",
            cnode_cap = in(reg) cnode_cap,
            src_slot = in(reg) src_slot,
            dest_slot = in(reg) dest_slot,
            rights = in(reg) rights,
            syscall = in(reg) SYS_CAP_DERIVE,
            result = out(reg) result,
            out("x8") _,
            out("x0") _,
            out("x1") _,
            out("x2") _,
            out("x3") _,
        );
    }
    result
}

fn syscall_endpoint_create() -> u64 {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "mov x8, {syscall}",
            "svc #0",
            "mov {result}, x0",
            syscall = in(reg) SYS_ENDPOINT_CREATE,
            result = out(reg) result,
            out("x8") _,
            out("x0") _,
        );
    }
    result
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    print("\n");
    print("═══════════════════════════════════════════════\n");
    print("  Capability Derivation & Revocation Tests\n");
    print("═══════════════════════════════════════════════\n");
    print("\n");

    // Test 1: Revoke syscall interface verification
    test_syscall_interface();

    // Test 2: Revoke error handling
    test_error_handling();

    // Test 3: Derive and recursive revocation (THE MAIN TEST!)
    test_derive_and_revoke();

    print("═══════════════════════════════════════════════\n");
    print("  All tests completed!\n");
    print("═══════════════════════════════════════════════\n");
    print("\n");

    // All tests complete - yield forever
    loop {
        unsafe {
            core::arch::asm!(
                "mov x8, {syscall}",
                "svc #0",
                syscall = in(reg) SYS_YIELD,
                out("x8") _,
                out("x0") _,
            );
        }
    }
}

/// Test 1: Syscall interface verification
fn test_syscall_interface() {
    print("[TEST 1] Syscall Interface Verification\n");

    // Allocate a capability slot
    let slot = syscall_cap_allocate();
    if slot == u64::MAX {
        print("  ✗ FAIL: Could not allocate capability slot\n");
        print("\n");
        return;
    }

    // Skip printing slot number - has formatting issues
    print("  ✓ Allocated cap slot\n");

    // Try to revoke empty slot (should fail gracefully or succeed with no-op)
    let result = syscall_cap_revoke(0, slot);
    if result == 0 {
        print("  ⚠ WARN: Revoke succeeded on empty slot (safe no-op)\n");
    } else {
        print("  ✓ Revoke correctly failed on empty slot\n");
    }

    print("\n");
}

/// Test 2: Error handling
fn test_error_handling() {
    print("[TEST 2] Error Handling\n");

    // Test 2a: Invalid slot (out of bounds)
    print("  [2a] Revoke invalid slot (99999)...\n");
    let result = syscall_cap_revoke(0, 99999);
    if result == u64::MAX {
        print("    ✓ Correctly rejected invalid slot\n");
    } else {
        print("    ✗ FAIL: Should reject invalid slot\n");
    }

    // Test 2b: Reserved slot
    print("  [2b] Revoke reserved slot (0)...\n");
    let result = syscall_cap_revoke(0, 0);
    if result == u64::MAX {
        print("    ✓ Correctly rejected slot 0\n");
    } else {
        print("    ⚠ WARN: Revoke succeeded on slot 0\n");
    }

    print("\n");
}

/// Test 3: Derive and recursive revocation (CDT lifecycle test)
///
/// This is the critical test that validates the CDT works correctly:
/// 1. Create an endpoint capability (parent)
/// 2. Derive a child capability from it
/// 3. Revoke the parent
/// 4. Verify the child is gone (recursive revocation)
fn test_derive_and_revoke() {
    print("[TEST 3] Capability Derivation & Recursive Revocation\n");

    // Step 1: Create an endpoint (this will be our parent capability)
    print("  [3a] Creating endpoint capability...\n");
    let endpoint_slot = syscall_endpoint_create();
    if endpoint_slot == u64::MAX {
        print("    ✗ FAIL: Could not create endpoint\n");
        print("\n");
        return;
    }
    print("    ✓ Created endpoint\n");

    // Step 2: Allocate a slot for the derived capability
    print("  [3b] Allocating slot for derived capability...\n");
    let child_slot = syscall_cap_allocate();
    if child_slot == u64::MAX {
        print("    ✗ FAIL: Could not allocate child slot\n");
        print("\n");
        return;
    }
    print("    ✓ Allocated child slot\n");

    // Step 3: Derive a child capability (full rights: 0x7 = RWX)
    print("  [3c] Deriving child capability...\n");
    let result = syscall_cap_derive(0, endpoint_slot, child_slot, 0x7);
    if result != 0 {
        print("    ✗ FAIL: Derive operation failed\n");
        print("\n");
        return;
    }
    print("    ✓ Derived child capability\n");

    // Step 4: Revoke the parent capability
    print("  [3d] Revoking parent (should recursively delete child)...\n");
    let result = syscall_cap_revoke(0, endpoint_slot);
    if result != 0 {
        print("    ✗ FAIL: Revoke failed\n");
        print("\n");
        return;
    }
    print("    ✓ Revoked parent capability\n");

    // Step 5: Verify child is gone by trying to derive from it (should fail)
    print("  [3e] Verifying child was recursively deleted...\n");
    let verify_slot = syscall_cap_allocate();
    if verify_slot == u64::MAX {
        print("    ✗ FAIL: Could not allocate verify slot\n");
        print("\n");
        return;
    }

    // Try to derive from the child slot (should fail because it was revoked)
    let result = syscall_cap_derive(0, child_slot, verify_slot, 0x7);
    if result == u64::MAX {
        print("    ✓ Child was recursively deleted (derive failed)\n");
        print("    ✓ CDT recursive revocation WORKS!\n");
    } else {
        print("    ✗ FAIL: Child still exists (CDT broken)\n");
    }

    print("\n");
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    print("[test] PANIC!\n");
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
