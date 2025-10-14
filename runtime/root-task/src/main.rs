//! Minimal root task binary for Chapter 7
//!
//! This is the first userspace program that runs on the KaaL microkernel.
//! It receives boot parameters from the kernel and demonstrates EL0 execution
//! by calling the sys_print syscall.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

/// Syscall number for sys_debug_print
const SYS_DEBUG_PRINT: usize = 0x1001;

/// Make a syscall to print a message
///
/// # Arguments
/// * `msg` - Message to print (pointer to null-terminated string)
/// * `len` - Length of message
///
/// # Safety
/// This function performs a raw syscall using inline assembly.
/// The kernel must implement the sys_print syscall handler.
unsafe fn sys_print(msg: &str) {
    let msg_ptr = msg.as_ptr() as usize;
    let msg_len = msg.len();

    // ARM64 syscall convention:
    // - x8 = syscall number
    // - x0 = arg1 (message pointer)
    // - x1 = arg2 (message length)
    // - svc #0 = trigger supervisor call
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {msg_ptr}",
        "mov x1, {msg_len}",
        "svc #0",
        syscall_num = in(reg) SYS_DEBUG_PRINT,
        msg_ptr = in(reg) msg_ptr,
        msg_len = in(reg) msg_len,
        out("x0") _,
        out("x1") _,
        out("x8") _,
    );
}

/// Root task entry point
///
/// This function is called by the kernel after it sets up the root task's
/// execution context and transitions to EL0 (userspace).
///
/// # Expected behavior
/// 1. Kernel creates root task TCB
/// 2. Kernel configures TCB (PC=_start, SP=stack_top, EL0)
/// 3. Kernel resumes TCB
/// 4. Root task starts executing here
/// 5. Root task calls sys_print to demonstrate userspace execution
/// 6. Root task enters idle loop
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Print hello message from userspace
    unsafe {
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  KaaL Root Task (EL0) v0.1.0\n");
        sys_print("  Chapter 7: Root Task & Boot Protocol\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");
        sys_print("[root_task] Hello from userspace (EL0)!\n");
        sys_print("[root_task] Syscalls working: sys_print functional\n");
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  Chapter 7: COMPLETE ✓\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");
    }

    // Idle loop - wait for interrupts
    loop {
        unsafe {
            core::arch::asm!("wfi"); // Wait for interrupt
        }
    }
}

/// Panic handler
///
/// Called when the root task panics. Since we're in userspace with no
/// infrastructure yet, we just loop forever.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
