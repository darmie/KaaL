//! Null console component (no output)
//!
//! This is a no-op console implementation for production builds where
//! debug output is not desired. All output operations are compiled away
//! as no-ops, resulting in zero runtime overhead.

use super::Console;

/// Null console configuration (empty - no configuration needed)
#[derive(Clone, Copy)]
pub struct NullConfig;

/// Null console component (no output)
///
/// This console discards all output. Use this in production builds
/// to eliminate debug output overhead completely.
///
/// The compiler will optimize away all calls to this console since
/// they have no side effects.
pub struct NullConsole;

impl NullConsole {
    /// Create a new null console
    pub const fn new(_config: NullConfig) -> Self {
        Self
    }

    /// Initialize null console (no-op)
    pub fn init(&self) {
        // Nothing to initialize
    }
}

impl Console for NullConsole {
    #[inline(always)]
    fn putc(&self, _c: u8) {
        // Discard output - compiler will optimize this away
    }

    #[inline(always)]
    fn puts(&self, _s: &str) {
        // Discard output - compiler will optimize this away
    }
}
