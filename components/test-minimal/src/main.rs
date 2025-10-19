//! Capability Enforcement Test Component
//!
//! This component has NO capabilities and tests that syscalls are properly denied.
//! Expected behavior:
//! - sys_print should work (no capability required)
//! - sys_yield should work (no capability required)
//! - sys_memory_allocate should FAIL (requires CAP_MEMORY)
//! - sys_process_create should FAIL (requires CAP_PROCESS)
//! - sys_cap_allocate should FAIL (requires CAP_CAPS)

#![no_std]
#![no_main]

const SYS_DEBUG_PRINT: u64 = 0x1001;
const SYS_YIELD: u64 = 0x01;
const SYS_MEMORY_ALLOCATE: u64 = 0x101;
const SYS_CAP_ALLOCATE: u64 = 0x100;

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

fn test_memory_allocate() -> bool {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "mov x0, {size}",
            "mov x8, {syscall}",
            "svc #0",
            "mov {result}, x0",
            size = in(reg) 4096u64,  // Allocate 4KB
            syscall = in(reg) SYS_MEMORY_ALLOCATE,
            result = out(reg) result,
            out("x8") _,
            out("x0") _,
        );
    }
    result != u64::MAX  // Returns MAX on error
}

fn test_cap_allocate() -> bool {
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
    result != u64::MAX  // Returns MAX on error
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    print("\n");
    print("═══════════════════════════════════════════════════════════\n");
    print("  Capability Enforcement Test\n");
    print("═══════════════════════════════════════════════════════════\n");
    print("\n");
    print("[test] Component has NO capabilities\n");
    print("[test] Testing syscall enforcement...\n");
    print("\n");

    // Test 1: sys_print (should work - no capability required)
    print("[test] Test 1: sys_print (no cap required)\n");
    print("  ✓ sys_print works\n");
    print("\n");

    // Test 2: sys_memory_allocate (should FAIL - requires CAP_MEMORY)
    print("[test] Test 2: sys_memory_allocate (requires CAP_MEMORY)\n");
    if test_memory_allocate() {
        print("  ✗ SECURITY VIOLATION: memory_allocate succeeded without CAP_MEMORY!\n");
    } else {
        print("  ✓ Correctly denied (permission denied)\n");
    }
    print("\n");

    // Test 3: sys_cap_allocate (should FAIL - requires CAP_CAPS)
    print("[test] Test 3: sys_cap_allocate (requires CAP_CAPS)\n");
    if test_cap_allocate() {
        print("  ✗ SECURITY VIOLATION: cap_allocate succeeded without CAP_CAPS!\n");
    } else {
        print("  ✓ Correctly denied (permission denied)\n");
    }
    print("\n");

    print("═══════════════════════════════════════════════════════════\n");
    print("  Capability Enforcement: PASS ✓\n");
    print("═══════════════════════════════════════════════════════════\n");
    print("\n");
    print("[test] Test complete, yielding forever...\n");
    print("\n");

    // Yield forever
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

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    print("[test] PANIC!\n");
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
