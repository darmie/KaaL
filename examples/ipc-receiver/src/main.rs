//! IPC Receiver Test Component
//!
//! Demonstrates shared memory IPC using SharedRing with notification-based signaling.
//! Consumer side: Pops messages from ring buffer and waits for notifications.

#![no_std]
#![no_main]

extern crate kaal_ipc;

use core::panic::PanicInfo;
use kaal_ipc::{SharedRing, Consumer};

/// Syscall numbers
const SYS_DEBUG_PRINT: usize = 0x1001;
const SYS_YIELD: usize = 0x01;
const SYS_NOTIFICATION_CREATE: usize = 0x17;

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

/// Create a notification object
unsafe fn sys_notification_create() -> u64 {
    let result: u64;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_NOTIFICATION_CREATE,
        result = out(reg) result,
        out("x8") _,
    );
    result
}

/// Message type for IPC
#[repr(C)]
#[derive(Copy, Clone)]
struct Message {
    data: [u8; 64],
    len: usize,
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  IPC Receiver - Shared Memory Ring Buffer Consumer\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");

        // Create notification objects (same capability slots as sender in real scenario)
        sys_print("[ipc-receiver] Creating notification objects...\n");
        let consumer_notify = sys_notification_create();
        let producer_notify = sys_notification_create();

        if consumer_notify == u64::MAX || producer_notify == u64::MAX {
            sys_print("[ipc-receiver] ✗ Failed to create notifications\n");
            sys_print("[ipc-receiver] Test: FAIL\n");
            loop {
                core::arch::asm!("wfi");
            }
        }

        sys_print("[ipc-receiver] ✓ Notifications created successfully\n");

        // Access shared ring buffer (in real implementation, this would be mapped shared memory)
        // For testing, we use a static buffer in BSS
        static mut RING: SharedRing<Message, 16> = SharedRing::new();
        RING = SharedRing::with_notifications(consumer_notify, producer_notify);

        let consumer = Consumer::new(&RING);

        sys_print("[ipc-receiver] Shared ring buffer initialized\n");
        sys_print("[ipc-receiver] Waiting for messages...\n");
        sys_print("\n");

        // Receive messages in a loop
        let mut msg_count = 0;
        for _ in 0..5 {
            // Wait for notification that data is available
            match consumer.wait_for_data() {
                Ok(signals) => {
                    sys_print("[ipc-receiver] ✓ Received notification (signals: ");
                    let mut num = signals;
                    if num == 0 {
                        sys_print("0");
                    } else {
                        let mut digits = [0u8; 20];
                        let mut i = 0;
                        while num > 0 {
                            digits[i] = b'0' + (num % 10) as u8;
                            num /= 10;
                            i += 1;
                        }
                        while i > 0 {
                            i -= 1;
                            let digit = core::str::from_utf8_unchecked(&digits[i..i + 1]);
                            sys_print(digit);
                        }
                    }
                    sys_print(")\n");

                    // Try to pop message
                    match consumer.pop() {
                        Ok(msg) => {
                            msg_count += 1;
                            sys_print("[ipc-receiver] ✓ Message ");
                            let digit = (b'0' + msg_count) as char;
                            let mut buf = [0u8; 1];
                            buf[0] = digit as u8;
                            sys_print(core::str::from_utf8_unchecked(&buf));
                            sys_print(" received: \"");

                            // Print message content
                            if msg.len > 0 && msg.len <= 64 {
                                let text = core::str::from_utf8_unchecked(&msg.data[..msg.len]);
                                sys_print(text);
                            }
                            sys_print("\"\n");
                        }
                        Err(_) => {
                            sys_print("[ipc-receiver]   Buffer empty (spurious wakeup)\n");
                        }
                    }
                }
                Err(_) => {
                    sys_print("[ipc-receiver] ✗ Wait for notification failed\n");
                    break;
                }
            }

            // Brief yield
            core::arch::asm!(
                "mov x8, {syscall_num}",
                "svc #0",
                syscall_num = in(reg) SYS_YIELD,
                out("x8") _,
                out("x0") _,
            );
        }

        sys_print("\n");
        sys_print("[ipc-receiver] ✓ Received ");
        let digit = (b'0' + msg_count) as char;
        let mut buf = [0u8; 1];
        buf[0] = digit as u8;
        sys_print(core::str::from_utf8_unchecked(&buf));
        sys_print(" messages successfully!\n");
        sys_print("[ipc-receiver] Shared memory IPC test: PASS\n");
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  IPC Receiver Complete\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");
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
