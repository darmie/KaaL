//! KaaL Microkernel SDK
//!
//! Clean, safe API for interacting with the KaaL microkernel.
//!
//! # Modules
//! - [`syscall`]: Low-level syscall wrappers
//! - [`ipc`]: High-level IPC utilities (re-exports from kaal-ipc)
//! - [`capability`]: Capability management
//! - [`memory`]: Memory allocation and mapping
//! - [`process`]: Process creation and management
//!
//! # Example
//! ```no_run
//! use kaal_sdk::syscall;
//!
//! fn main() {
//!     syscall::print("Hello from KaaL SDK!\n");
//!     syscall::yield_now();
//! }
//! ```

#![no_std]

pub mod syscall;
pub mod capability;
pub mod memory;
pub mod process;

// Re-export IPC from kaal-ipc for convenience
pub use kaal_ipc as ipc;

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result type for SDK operations
pub type Result<T> = core::result::Result<T, Error>;

/// SDK error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Syscall returned error
    SyscallFailed,
    /// Invalid parameter
    InvalidParameter,
    /// Out of memory
    OutOfMemory,
    /// Capability not found
    CapabilityNotFound,
    /// Operation not permitted
    PermissionDenied,
    /// Resource busy
    Busy,
    /// Operation would block
    WouldBlock,
}

impl Error {
    /// Convert from syscall return value
    pub fn from_syscall(ret: usize) -> Result<usize> {
        if ret == usize::MAX {
            Err(Error::SyscallFailed)
        } else {
            Ok(ret)
        }
    }
}
