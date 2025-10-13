//! System call numbers
//!
//! Syscall numbering follows seL4 conventions where possible.
//! Debug syscalls are in the 0x1000+ range.

/// Debug: Print a single character to console
pub const SYS_DEBUG_PUTCHAR: u64 = 0x1000;

/// Debug: Print a string to console (ptr, len)
pub const SYS_DEBUG_PRINT: u64 = 0x1001;

/// Yield the CPU to the scheduler
pub const SYS_YIELD: u64 = 0x01;

/// Send a message on an IPC endpoint (not yet implemented)
pub const SYS_SEND: u64 = 0x02;

/// Receive a message on an IPC endpoint (not yet implemented)
pub const SYS_RECV: u64 = 0x03;

/// Call: Combined send + receive (not yet implemented)
pub const SYS_CALL: u64 = 0x04;

/// Reply: Reply to a call (not yet implemented)
pub const SYS_REPLY: u64 = 0x05;
