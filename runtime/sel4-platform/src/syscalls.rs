//! Unified seL4 syscall interface
//!
//! This module provides platform-agnostic syscalls that work across:
//! - Mock mode (for testing)
//! - Runtime mode (for advanced use)

/// Mock mode - use our mock implementations
#[cfg(feature = "mock")]
pub use sel4_mock_sys::*;

/// Runtime mode - use real seL4 runtime bindings (use high-level sel4 crate)
#[cfg(feature = "runtime")]
pub use sel4::sys::*;

// Ensure at compile-time that exactly one mode is selected
#[cfg(not(any(feature = "mock", feature = "microkit", feature = "runtime")))]
compile_error!("Must select exactly one seL4 mode: mock, microkit, or runtime");

#[cfg(all(feature = "mock", feature = "microkit"))]
compile_error!("Cannot enable both 'mock' and 'microkit' features");

#[cfg(all(feature = "mock", feature = "runtime"))]
compile_error!("Cannot enable both 'mock' and 'runtime' features");

#[cfg(all(feature = "microkit", feature = "runtime"))]
compile_error!("Cannot enable both 'microkit' and 'runtime' features");
