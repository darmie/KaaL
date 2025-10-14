//! System call interface
//!
//! This module implements the syscall dispatcher for the KaaL microkernel.
//! Syscalls follow the seL4 convention with syscall number in x8 and
//! arguments in x0-x5.

pub mod numbers;

use crate::arch::aarch64::context::TrapFrame;
use crate::kprintln;

/// Syscall dispatcher - called from exception handler
///
/// Decodes the syscall number from the trap frame and dispatches to the
/// appropriate handler. Returns the result in x0.
pub fn handle_syscall(tf: &mut TrapFrame) {
    let syscall_num = tf.syscall_number();
    let args = tf.syscall_args();

    // Dispatch based on syscall number
    let result = match syscall_num {
        numbers::SYS_DEBUG_PUTCHAR => sys_debug_putchar(args[0]),
        numbers::SYS_DEBUG_PRINT => sys_debug_print(args[0], args[1]),
        numbers::SYS_YIELD => sys_yield(),
        _ => {
            kprintln!("[syscall] Unknown syscall number: {}", syscall_num);
            u64::MAX // Error: invalid syscall
        }
    };

    // Set return value
    tf.set_return_value(result);
}

/// Debug syscall: print a single character
fn sys_debug_putchar(ch: u64) -> u64 {
    if ch <= 0x7F {
        crate::kprint!("{}", ch as u8 as char);
        0 // Success
    } else {
        u64::MAX // Error: invalid character
    }
}

/// Debug syscall: print a string
///
/// This is a simple implementation that reads from the user's address space.
/// In a production kernel, this would need proper memory validation and
/// page table walking to ensure the address is valid and mapped.
///
/// For Chapter 7, we assume the root task has identity-mapped memory,
/// so we can directly access the pointer.
fn sys_debug_print(ptr: u64, len: u64) -> u64 {
    // Validate length (prevent abuse)
    if len > 4096 {
        return u64::MAX; // Error: string too long
    }

    // Safety: We're assuming identity-mapped memory for now.
    // TODO Chapter 8: Add proper memory validation via page table walk
    unsafe {
        let slice = core::slice::from_raw_parts(ptr as *const u8, len as usize);

        // Validate UTF-8 (optional, but prevents panic)
        if let Ok(s) = core::str::from_utf8(slice) {
            crate::kprint!("{}", s);
            0 // Success
        } else {
            u64::MAX // Error: invalid UTF-8
        }
    }
}

/// Yield syscall: give up CPU time slice
fn sys_yield() -> u64 {
    kprintln!("[syscall] yield (no-op, scheduler not implemented)");
    0 // Success
}
