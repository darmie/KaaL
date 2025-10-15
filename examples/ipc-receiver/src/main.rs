//! IPC Receiver Test Component
//!
//! Receives messages from an IPC endpoint to test the kernel's IPC implementation.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

/// Syscall numbers
const SYS_DEBUG_PRINT: usize = 0x1001;
const SYS_YIELD: usize = 0x01;
const SYS_RECV: usize = 0x03;  // IPC Receive syscall

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

/// Receive a message via IPC
unsafe fn sys_ipc_recv(endpoint_cap: usize, buffer_ptr: usize, buffer_len: usize) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {cap}",
        "mov x1, {buf_ptr}",
        "mov x2, {buf_len}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_RECV,
        cap = in(reg) endpoint_cap,
        buf_ptr = in(reg) buffer_ptr,
        buf_len = in(reg) buffer_len,
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
        sys_print("  IPC Receiver Test Component\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");
        sys_print("[ipc-receiver] Waiting for IPC message...\n");

        // Buffer to receive message
        let mut buffer = [0u8; 256];

        // Endpoint capability (should be passed by root task, for now use slot 200)
        let endpoint_cap = 200;

        sys_print("[ipc-receiver] Blocking on endpoint capability slot 200\n");

        // Receive the message (will block until sender is ready)
        let bytes_received = sys_ipc_recv(endpoint_cap, buffer.as_mut_ptr() as usize, buffer.len());

        if bytes_received != usize::MAX {
            sys_print("[ipc-receiver] ✓ Message received successfully!\n");
            sys_print("[ipc-receiver] Received ");

            // Print number of bytes
            let mut num = bytes_received;
            let mut digits = [0u8; 20];
            let mut i = 0;
            if num == 0 {
                sys_print("0");
            } else {
                while num > 0 {
                    digits[i] = b'0' + (num % 10) as u8;
                    num /= 10;
                    i += 1;
                }
                while i > 0 {
                    i -= 1;
                    let digit = core::str::from_utf8_unchecked(&digits[i..i+1]);
                    sys_print(digit);
                }
            }

            sys_print(" bytes\n");

            // Print the message content (if it's valid UTF-8)
            if bytes_received <= buffer.len() {
                if let Ok(msg) = core::str::from_utf8(&buffer[..bytes_received]) {
                    sys_print("[ipc-receiver] Message content: \"");
                    sys_print(msg);
                    sys_print("\"\n");
                }
            }

            sys_print("[ipc-receiver] IPC receive test: PASS\n");
        } else {
            sys_print("[ipc-receiver] ✗ Message receive failed\n");
            sys_print("[ipc-receiver] IPC receive test: FAIL\n");
        }

        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  IPC Receiver Complete\n");
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
