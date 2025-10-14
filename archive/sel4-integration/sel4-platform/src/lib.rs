//! # seL4 Platform Abstraction Layer
//!
//! This module provides a unified API across different seL4 deployment modes:
//! - **Mock Mode**: Fast unit testing
//! - **Microkit Mode**: seL4 Microkit deployment (default)
//! - **Runtime Mode**: Direct Rust seL4 runtime (advanced)
//!
//! ## Usage
//!
//! ```rust
//! use sel4_platform::adapter as sel4;
//!
//! // Unified API works across all modes
//! unsafe {
//!     let bootinfo = sel4::get_boot_info();
//!     let error = sel4::untyped_retype(...);
//!     if sel4::is_ok(error) {
//!         // Success
//!     }
//! }
//! ```
//!
//! ## Build Modes
//!
//! ```bash
//! # Microkit (default - real seL4)
//! cargo build
//!
//! # Mock (testing only)
//! cargo build --features mock
//!
//! # Runtime (advanced)
//! cargo build --features runtime
//! ```

#![no_std]

/// Unified seL4 adapter - provides consistent API across all modes
///
/// This is the main module KaaL crates should use.
/// It provides the same function signatures regardless of backend.
pub mod adapter;

/// Low-level syscall re-exports (for advanced use)
pub mod syscalls;

/// Platform configuration and detection
pub mod config {
    /// Detect which seL4 mode is active at compile time
    pub fn platform_mode() -> &'static str {
        #[cfg(feature = "mock")]
        return "mock";

        #[cfg(feature = "runtime")]
        return "runtime";

        #[cfg(not(any(feature = "mock", feature = "runtime")))]
        compile_error!("No seL4 platform mode selected. Use either 'mock' or 'runtime' feature.");
    }

    /// Check if we're in mock mode (testing)
    pub const fn is_mock() -> bool {
        cfg!(feature = "mock")
    }

    /// Check if we're in runtime mode (real seL4)
    pub const fn is_runtime() -> bool {
        cfg!(feature = "runtime")
    }
}

// Re-export adapter as the primary interface
pub use adapter as sel4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let mode = config::platform_mode();
        assert!(mode == "mock" || mode == "runtime");
    }

    #[test]
    #[cfg(feature = "mock")]
    fn test_mock_mode() {
        assert!(config::is_mock());
        assert!(!config::is_runtime());
    }

    #[test]
    fn test_adapter_api() {
        // Test that adapter functions are accessible
        use sel4::*;

        // These should compile regardless of mode
        let _ = NO_ERROR;
        let _ = INVALID_ARGUMENT;
        let _ = CAN_READ;
        let _ = TCB_OBJECT;
    }
}
