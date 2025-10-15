//! IPC Sender Test Component
//!
//! Sends messages to an IPC endpoint to test the kernel's IPC implementation.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

/// Syscall numbers
const SYS_DEBUG_PRINT: usize = 0x1001;
const SYS_YIELD: usize = 0x01;
const SYS_SEND: usize = 0x02;  // IPC Send syscall

/// Print helper
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

/// Send a message via IPC
unsafe fn sys_ipc_send(endpoint_cap: usize, message_ptr: usize, message_len: usize) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {cap}",
        "mov x1, {msg_ptr}",
        "mov x2, {msg_len}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_SEND,
        cap = in(reg) endpoint_cap,
        msg_ptr = in(reg) message_ptr,
        msg_len = in(reg) message_len,
        result = out(reg) result,
        out("x8") _,
        out("x0") _,
        out("x1") _,
        out("x2") _,
    );
    result
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  IPC Sender Test Component\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");
        sys_print("[ipc-sender] Preparing to send IPC message...\n");

        // Message to send
        let message = b"Hello from sender!";

        // Endpoint capability (should be passed by root task, for now use slot 200)
        let endpoint_cap = 200;

        sys_print("[ipc-sender] Sending message: \"Hello from sender!\"\n");
        sys_print("[ipc-sender] Using endpoint capability slot 200\n");

        // Send the message (will block until receiver is ready)
        let result = sys_ipc_send(endpoint_cap, message.as_ptr() as usize, message.len());

        if result == 0 {
            sys_print("[ipc-sender] ✓ Message sent successfully!\n");
            sys_print("[ipc-sender] IPC send test: PASS\n");
        } else {
            sys_print("[ipc-sender] ✗ Message send failed\n");
            sys_print("[ipc-sender] IPC send test: FAIL\n");
        }

        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  IPC Sender Complete\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");

        // Yield back to scheduler
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) SYS_YIELD,
            out("x8") _,
            out("x0") _,
        );
    }

    // Idle loop
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
