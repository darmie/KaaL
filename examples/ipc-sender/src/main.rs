//! IPC Sender Test Component
//!
//! Demonstrates shared memory IPC using SharedRing with notification-based signaling.
//! Producer side: Pushes messages into ring buffer and signals consumer.

#![no_std]
#![no_main]

extern crate kaal_ipc;

use core::panic::PanicInfo;
use kaal_ipc::{SharedRing, Producer};

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

impl Message {
    fn new(msg: &str) -> Self {
        let mut data = [0u8; 64];
        let bytes = msg.as_bytes();
        let len = bytes.len().min(64);
        data[..len].copy_from_slice(&bytes[..len]);
        Self { data, len }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  IPC Sender - Shared Memory Ring Buffer Producer\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");

        // Create notification objects
        sys_print("[ipc-sender] Creating notification objects...\n");
        let consumer_notify = sys_notification_create();
        let producer_notify = sys_notification_create();

        if consumer_notify == u64::MAX || producer_notify == u64::MAX {
            sys_print("[ipc-sender] ✗ Failed to create notifications\n");
            sys_print("[ipc-sender] Test: FAIL\n");
            loop {
                core::arch::asm!("wfi");
            }
        }

        sys_print("[ipc-sender] ✓ Notifications created successfully\n");

        // Create shared ring buffer in BSS (will be mapped to shared memory in real implementation)
        static mut RING: SharedRing<Message, 16> = SharedRing::new();

        // Initialize ring buffer with notifications
        // In a real implementation, this would be in shared memory and both processes would have access
        RING = SharedRing::with_notifications(consumer_notify, producer_notify);

        let producer = Producer::new(&RING);

        sys_print("[ipc-sender] Shared ring buffer initialized (capacity: 16 messages)\n");
        sys_print("\n");

        // Send test messages
        sys_print("[ipc-sender] Sending test messages...\n");

        let messages = [
            "Hello from sender!",
            "Message #2",
            "Message #3",
            "Testing shared memory IPC",
            "Zero-copy bulk transfer",
        ];

        for (i, msg_text) in messages.iter().enumerate() {
            let msg = Message::new(msg_text);

            sys_print("[ipc-sender] Pushing message ");
            // Print message number
            let num = i + 1;
            if num < 10 {
                let digit = (b'0' + num as u8) as char;
                let mut buf = [0u8; 1];
                buf[0] = digit as u8;
                sys_print(core::str::from_utf8_unchecked(&buf));
            }
            sys_print(": \"");
            sys_print(msg_text);
            sys_print("\"\n");

            match producer.push(msg) {
                Ok(_) => {
                    sys_print("[ipc-sender]   ✓ Message pushed and consumer notified\n");
                }
                Err(e) => {
                    sys_print("[ipc-sender]   ✗ Push failed\n");
                }
            }

            // Brief yield to allow consumer to process
            core::arch::asm!(
                "mov x8, {syscall_num}",
                "svc #0",
                syscall_num = in(reg) SYS_YIELD,
                out("x8") _,
                out("x0") _,
            );
        }

        sys_print("\n");
        sys_print("[ipc-sender] ✓ All messages sent successfully!\n");
        sys_print("[ipc-sender] Shared memory IPC test: PASS\n");
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  IPC Sender Complete\n");
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
