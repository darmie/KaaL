//! Echo Server Component
//!
//! This is the first independently spawned process in the KaaL microkernel!
//!
//! It demonstrates:
//! - Process creation with full isolation
//! - Separate address space (TTBR0)
//! - Separate capability space (CSpace/CNode)
//! - Independent execution in EL0

#![no_std]
#![no_main]

use core::panic::PanicInfo;

/// Syscall number for debug print
const SYS_DEBUG_PRINT: usize = 0x1001;

/// Make a syscall to print a message
unsafe fn sys_print(msg: &str) {
    let msg_ptr = msg.as_ptr() as usize;
    let msg_len = msg.len();

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

/// Entry point for the echo-server process
///
/// This function is called by the kernel after the process is created
/// and added to the scheduler.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  🎉 Echo Server Process Started!\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");
        sys_print("[echo-server] Running in separate process!\n");
        sys_print("[echo-server] Own address space (TTBR0)\n");
        sys_print("[echo-server] Own capability space (CSpace)\n");
        sys_print("[echo-server] Full isolation from root-task\n");
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  ✓ Multi-Process Microkernel System Active!\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");
    }

    // Idle loop - yield CPU to other processes
    loop {
        unsafe {
            // Use WFI to yield and save power
            core::arch::asm!("wfi");
        }
    }
}

/// Panic handler for the echo-server process
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
