//! Console component trait
//!
//! Provides a minimal console interface for kernel debug output.
//! This is NOT a full UART driver - just enough for `kprintln!` to work.
//!
//! Full UART drivers with interrupts, DMA, and buffering live in user-space
//! as framework components (see runtime/components/drivers/uart_pl011).

use core::fmt;

/// Console trait for kernel debug output
///
/// Kernel components must implement this trait to provide debug console
/// functionality. This is minimal by design - only `putc()` for character
/// output.
///
/// # Design Philosophy
/// Following seL4 principles: kernel components are MINIMAL. Full-featured
/// drivers with interrupts, DMA, buffering, etc. belong in user-space.
pub trait Console: Send + Sync {
    /// Write a single character to the console
    ///
    /// This is a blocking operation. The implementation should wait for
    /// the hardware to be ready before writing.
    ///
    /// # Arguments
    /// * `c` - Character to write
    fn putc(&self, c: u8);

    /// Write a string to the console
    ///
    /// Default implementation writes character by character.
    /// Can be overridden for more efficient implementations.
    fn puts(&self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.putc(b'\r'); // CRLF for terminals
            }
            self.putc(byte);
        }
    }
}

/// Wrapper for using Console with core::fmt::Write
pub struct ConsoleWriter<C: Console + 'static> {
    console: &'static C,
}

impl<C: Console + 'static> ConsoleWriter<C> {
    pub const fn new(console: &'static C) -> Self {
        Self { console }
    }
}

impl<C: Console + 'static> fmt::Write for ConsoleWriter<C> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.console.puts(s);
        Ok(())
    }
}

// Component implementations
pub mod pl011;
pub mod null;
