//! Kernel configuration and component composition
//!
//! This module handles compile-time kernel configuration and component
//! composition based on cargo features.

use crate::components::console::{Console, pl011::{Pl011Console, Pl011Config}, null::{NullConsole, NullConfig}};

/// Console component selection (compile-time)
///
/// This uses cargo features to select which console implementation to use:
/// - `console-pl011`: PL011 UART console (default for QEMU virt)
/// - `console-null`: No console output (production builds)
///
/// This mirrors the framework's runtime component spawning, but at compile-time.
#[cfg(feature = "console-pl011")]
pub static CONSOLE: Pl011Console = Pl011Console::new(Pl011Config {
    mmio_base: 0x9000000, // QEMU virt PL011 UART base address
});

#[cfg(feature = "console-null")]
pub static CONSOLE: NullConsole = NullConsole::new(NullConfig);

// Default to PL011 if no console feature is specified
#[cfg(not(any(feature = "console-pl011", feature = "console-null")))]
pub static CONSOLE: Pl011Console = Pl011Console::new(Pl011Config {
    mmio_base: 0x9000000,
});

/// Initialize kernel console component
///
/// Must be called early in boot sequence before any debug output.
pub fn init_console() {
    #[cfg(feature = "console-pl011")]
    CONSOLE.init();

    #[cfg(feature = "console-null")]
    CONSOLE.init();

    #[cfg(not(any(feature = "console-pl011", feature = "console-null")))]
    CONSOLE.init();
}

/// Get reference to the global console
///
/// This provides a typed reference to the console component for use
/// by the debug output system.
#[cfg(feature = "console-pl011")]
pub fn console() -> &'static impl Console {
    &CONSOLE
}

#[cfg(feature = "console-null")]
pub fn console() -> &'static impl Console {
    &CONSOLE
}

#[cfg(not(any(feature = "console-pl011", feature = "console-null")))]
pub fn console() -> &'static impl Console {
    &CONSOLE
}
