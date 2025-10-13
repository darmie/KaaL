//! IPC Message Structure
//!
//! This module defines the message format for inter-process communication.
//! Messages consist of:
//! - Message registers (data words)
//! - Message label (metadata + capability count)
//! - Capability transfer information
//!
//! ## Register Usage
//!
//! **Fast path (in CPU registers)**:
//! - x0-x3: First 4 message words (arguments)
//! - x4-x7: Additional 4 words (optional)
//!
//! **Extended path (in IPC buffer)**:
//! - Words 8-63: Additional message data
//!
//! Following seL4's design for compatibility and performance.

use crate::objects::{Capability, CapError};
use core::fmt;

/// Maximum number of message registers
///
/// seL4 supports up to 120 message registers on some architectures.
/// We'll use 64 for simplicity (8 in fast path + 56 in IPC buffer).
pub const MAX_MSG_REGS: usize = 64;

/// Number of message registers in fast path (CPU registers)
pub const FAST_PATH_REGS: usize = 8;

/// Maximum capabilities that can be transferred in one message
pub const MAX_CAPS: usize = 3;

/// IPC Message - data transferred during IPC
///
/// Messages are the fundamental unit of communication between threads.
/// They consist of data words (registers) and optionally capabilities.
#[derive(Clone)]
pub struct Message {
    /// Message label
    ///
    /// Contains user-defined data (typically syscall number or message type)
    /// and capability count in the lower bits.
    label: u64,

    /// Message registers (data payload)
    ///
    /// First FAST_PATH_REGS are passed in CPU registers (x0-x7),
    /// rest are transferred via IPC buffer.
    regs: [u64; MAX_MSG_REGS],

    /// Number of valid registers (0-MAX_MSG_REGS)
    len: usize,

    /// Capabilities to transfer
    caps: [Option<Capability>; MAX_CAPS],

    /// Number of capabilities (0-MAX_CAPS)
    num_caps: usize,
}

impl Message {
    /// Create a new empty message
    pub const fn new() -> Self {
        Self {
            label: 0,
            regs: [0; MAX_MSG_REGS],
            len: 0,
            caps: [None; MAX_CAPS],
            num_caps: 0,
        }
    }

    /// Create an empty message (alias for new())
    pub const fn empty() -> Self {
        Self::new()
    }

    /// Create a message with a label
    pub const fn with_label(label: u64) -> Self {
        Self {
            label,
            regs: [0; MAX_MSG_REGS],
            len: 0,
            caps: [None; MAX_CAPS],
            num_caps: 0,
        }
    }

    /// Create a message with label and registers
    pub fn with_regs(label: u64, regs: &[u64]) -> Self {
        let len = regs.len().min(MAX_MSG_REGS);
        let mut msg = Self::with_label(label);
        msg.regs[..len].copy_from_slice(&regs[..len]);
        msg.len = len;
        msg
    }

    /// Get the message label
    #[inline]
    pub fn label(&self) -> u64 {
        self.label
    }

    /// Set the message label
    #[inline]
    pub fn set_label(&mut self, label: u64) {
        self.label = label;
    }

    /// Get message length (number of registers)
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if message is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get a message register
    #[inline]
    pub fn get_reg(&self, index: usize) -> Option<u64> {
        if index < self.len {
            Some(self.regs[index])
        } else {
            None
        }
    }

    /// Set a message register
    pub fn set_reg(&mut self, index: usize, value: u64) -> Result<(), IpcError> {
        if index >= MAX_MSG_REGS {
            return Err(IpcError::MessageTooLarge);
        }

        self.regs[index] = value;
        if index >= self.len {
            self.len = index + 1;
        }

        Ok(())
    }

    /// Get all registers as a slice
    #[inline]
    pub fn regs(&self) -> &[u64] {
        &self.regs[..self.len]
    }

    /// Get mutable access to registers
    #[inline]
    pub fn regs_mut(&mut self) -> &mut [u64] {
        &mut self.regs[..self.len]
    }

    /// Set message length
    pub fn set_len(&mut self, len: usize) -> Result<(), IpcError> {
        if len > MAX_MSG_REGS {
            return Err(IpcError::MessageTooLarge);
        }
        self.len = len;
        Ok(())
    }

    /// Add a capability to the message
    pub fn add_cap(&mut self, cap: Capability) -> Result<(), IpcError> {
        if self.num_caps >= MAX_CAPS {
            return Err(IpcError::TooManyCaps);
        }

        self.caps[self.num_caps] = Some(cap);
        self.num_caps += 1;

        Ok(())
    }

    /// Get capability at index
    #[inline]
    pub fn get_cap(&self, index: usize) -> Option<&Capability> {
        if index < self.num_caps {
            self.caps[index].as_ref()
        } else {
            None
        }
    }

    /// Get capability at index (alias for get_cap)
    #[inline]
    pub fn cap(&self, index: usize) -> Option<Capability> {
        if index < self.num_caps {
            self.caps[index]
        } else {
            None
        }
    }

    /// Push a register value to the end of the message
    pub fn push(&mut self, value: u64) -> Result<(), IpcError> {
        if self.len >= MAX_MSG_REGS {
            return Err(IpcError::MessageTooLarge);
        }
        self.regs[self.len] = value;
        self.len += 1;
        Ok(())
    }

    /// Get number of capabilities
    #[inline]
    pub fn num_caps(&self) -> usize {
        self.num_caps
    }

    /// Get all capabilities as a slice
    pub fn caps(&self) -> &[Option<Capability>] {
        &self.caps[..self.num_caps]
    }

    /// Clear the message
    pub fn clear(&mut self) {
        self.label = 0;
        self.regs = [0; MAX_MSG_REGS];
        self.len = 0;
        self.caps = [None; MAX_CAPS];
        self.num_caps = 0;
    }

    /// Check if this message uses only the fast path
    ///
    /// Returns true if message fits in CPU registers only.
    #[inline]
    pub fn is_fast_path(&self) -> bool {
        self.len <= FAST_PATH_REGS && self.num_caps == 0
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Message")
            .field("label", &format_args!("{:#x}", self.label))
            .field("len", &self.len)
            .field("num_caps", &self.num_caps)
            .field("regs", &&self.regs[..self.len.min(8)])
            .finish()
    }
}

/// IPC Buffer - user-accessible memory for extended message data
///
/// Each thread has an IPC buffer page in its address space.
/// The kernel can read/write to it during IPC.
#[repr(C)]
pub struct IpcBuffer {
    /// Extended message registers (beyond fast path)
    ///
    /// Registers 8-63 are stored here (56 words).
    pub msg: [u64; MAX_MSG_REGS - FAST_PATH_REGS],

    /// Capability transfer metadata
    pub caps_unwrapped: u64,

    /// Capability receive slots
    pub caps: [u64; MAX_CAPS],

    /// Badge value (for endpoint identification)
    pub badge: u64,

    /// Reserved for future use
    pub _reserved: [u64; 8],
}

impl IpcBuffer {
    /// Create a new zeroed IPC buffer
    pub const fn new() -> Self {
        Self {
            msg: [0; MAX_MSG_REGS - FAST_PATH_REGS],
            caps_unwrapped: 0,
            caps: [0; MAX_CAPS],
            badge: 0,
            _reserved: [0; 8],
        }
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.msg = [0; MAX_MSG_REGS - FAST_PATH_REGS];
        self.caps_unwrapped = 0;
        self.caps = [0; MAX_CAPS];
        self.badge = 0;
    }
}

impl Default for IpcBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// IPC error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    /// Message too large (exceeds MAX_MSG_REGS)
    MessageTooLarge,

    /// Too many capabilities (exceeds MAX_CAPS)
    TooManyCaps,

    /// Capability error
    CapError(CapError),

    /// Endpoint not found
    EndpointNotFound,

    /// Insufficient rights
    InsufficientRights,

    /// Thread not blocked
    ThreadNotBlocked,

    /// Invalid IPC buffer
    InvalidIpcBuffer,

    /// IPC cancelled
    Cancelled,

    /// Invalid capability
    InvalidCapability,

    /// Null pointer encountered
    NullPointer,

    /// Queue corrupted
    QueueCorrupted,

    /// No IPC buffer configured
    NoIpcBuffer,
}

impl From<CapError> for IpcError {
    fn from(err: CapError) -> Self {
        IpcError::CapError(err)
    }
}

// Compile-time assertions
const _: () = {
    assert!(
        MAX_MSG_REGS >= FAST_PATH_REGS,
        "MAX_MSG_REGS must be >= FAST_PATH_REGS"
    );
    assert!(MAX_CAPS <= 8, "MAX_CAPS must be <= 8 for reasonable size");
    assert!(
        core::mem::size_of::<IpcBuffer>() <= 4096,
        "IpcBuffer must fit in a page"
    );
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_creation() {
        let msg = Message::new();
        assert_eq!(msg.label(), 0);
        assert_eq!(msg.len(), 0);
        assert_eq!(msg.num_caps(), 0);
        assert!(msg.is_empty());
    }

    #[test]
    fn message_with_label() {
        let msg = Message::with_label(0x42);
        assert_eq!(msg.label(), 0x42);
    }

    #[test]
    fn message_with_regs() {
        let regs = [1, 2, 3, 4, 5];
        let msg = Message::with_regs(0x100, &regs);

        assert_eq!(msg.label(), 0x100);
        assert_eq!(msg.len(), 5);
        assert_eq!(msg.get_reg(0), Some(1));
        assert_eq!(msg.get_reg(4), Some(5));
        assert_eq!(msg.get_reg(5), None);
    }

    #[test]
    fn message_set_reg() {
        let mut msg = Message::new();

        msg.set_reg(0, 100).unwrap();
        msg.set_reg(1, 200).unwrap();

        assert_eq!(msg.len(), 2);
        assert_eq!(msg.get_reg(0), Some(100));
        assert_eq!(msg.get_reg(1), Some(200));
    }

    #[test]
    fn message_add_cap() {
        use crate::objects::{Capability, CapType};

        let mut msg = Message::new();
        let cap = Capability::new(CapType::Endpoint, 0x1000);

        msg.add_cap(cap).unwrap();
        assert_eq!(msg.num_caps(), 1);
        assert!(msg.get_cap(0).is_some());
    }

    #[test]
    fn message_too_many_caps() {
        use crate::objects::{Capability, CapType};

        let mut msg = Message::new();
        let cap = Capability::new(CapType::Endpoint, 0x1000);

        // Add MAX_CAPS capabilities
        for _ in 0..MAX_CAPS {
            msg.add_cap(cap).unwrap();
        }

        // Next one should fail
        assert!(matches!(msg.add_cap(cap), Err(IpcError::TooManyCaps)));
    }

    #[test]
    fn message_fast_path() {
        let msg = Message::with_regs(0x42, &[1, 2, 3, 4]);
        assert!(msg.is_fast_path());

        let msg2 = Message::with_regs(0x42, &[1; 10]);
        assert!(!msg2.is_fast_path());
    }

    #[test]
    fn message_clear() {
        let mut msg = Message::with_regs(0x42, &[1, 2, 3]);
        assert_eq!(msg.len(), 3);

        msg.clear();
        assert_eq!(msg.label(), 0);
        assert_eq!(msg.len(), 0);
        assert_eq!(msg.num_caps(), 0);
    }

    #[test]
    fn ipc_buffer_size() {
        // IPC buffer should fit in a 4KB page
        assert!(core::mem::size_of::<IpcBuffer>() <= 4096);
    }

    #[test]
    fn ipc_buffer_creation() {
        let buf = IpcBuffer::new();
        assert_eq!(buf.badge, 0);
        assert_eq!(buf.caps_unwrapped, 0);
    }
}
