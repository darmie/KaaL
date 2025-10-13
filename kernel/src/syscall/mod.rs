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

    kprintln!("[syscall] Syscall {} with args: [{:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x}]",
        syscall_num, args[0], args[1], args[2], args[3], args[4], args[5]);

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

/// Debug syscall: print a string (stub - would need memory validation)
fn sys_debug_print(_ptr: u64, _len: u64) -> u64 {
    kprintln!("[syscall] debug_print not yet implemented (needs MMU)");
    u64::MAX // Error: not implemented
}

/// Yield syscall: give up CPU time slice
fn sys_yield() -> u64 {
    kprintln!("[syscall] yield (no-op, scheduler not implemented)");
    0 // Success
}
