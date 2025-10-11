//\! Debug output and logging

use core::fmt;

/// Debug writer (uses UART)
pub struct DebugWriter;

impl fmt::Write for DebugWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::arch::aarch64::uart::puts(s);
        Ok(())
    }
}

/// Print macro for kernel
#[macro_export]
macro_rules\! kprint {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write\!($crate::debug::DebugWriter, $($arg)*);
    });
}

/// Print with newline macro for kernel
#[macro_export]
macro_rules\! kprintln {
    () => ($crate::kprint\!("\n"));
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = writeln\!($crate::debug::DebugWriter, $($arg)*);
    });
}
