//! Debug output and logging
//!
//! Provides configurable logging with multiple levels:
//! - ERROR: Critical errors only
//! - WARN: Warnings and errors
//! - INFO: General information (default)
//! - DEBUG: Detailed debug information
//! - TRACE: Very detailed tracing
//!
//! Controlled via Cargo features:
//! - `log-error`: ERROR level only
//! - `log-warn`: WARN level and above
//! - `log-info`: INFO level and above (default)
//! - `log-debug`: DEBUG level and above
//! - `log-trace`: TRACE level (everything)

use crate::components::console::Console;
use core::fmt;

/// Debug writer (uses UART)
pub struct DebugWriter;

impl fmt::Write for DebugWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::config::console().puts(s);
        Ok(())
    }
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

/// Get current log level based on compile-time features
#[inline(always)]
pub const fn current_log_level() -> LogLevel {
    #[cfg(feature = "log-trace")]
    return LogLevel::Trace;

    #[cfg(all(feature = "log-debug", not(feature = "log-trace")))]
    return LogLevel::Debug;

    #[cfg(all(feature = "log-info", not(any(feature = "log-debug", feature = "log-trace"))))]
    return LogLevel::Info;

    #[cfg(all(feature = "log-warn", not(any(feature = "log-info", feature = "log-debug", feature = "log-trace"))))]
    return LogLevel::Warn;

    #[cfg(all(feature = "log-error", not(any(feature = "log-warn", feature = "log-info", feature = "log-debug", feature = "log-trace"))))]
    return LogLevel::Error;

    // Default to INFO if no log feature specified
    #[cfg(not(any(feature = "log-error", feature = "log-warn", feature = "log-info", feature = "log-debug", feature = "log-trace")))]
    return LogLevel::Info;
}

/// Check if a log level should be printed
#[inline(always)]
pub const fn should_log(level: LogLevel) -> bool {
    level as u8 <= current_log_level() as u8
}

/// Print macro for kernel (unconditional)
#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!($crate::debug::DebugWriter, $($arg)*);
    });
}

/// Print with newline macro for kernel (unconditional)
#[macro_export]
macro_rules! kprintln {
    () => ($crate::kprint!("\n"));
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = writeln!($crate::debug::DebugWriter, $($arg)*);
    });
}

/// Log ERROR message
#[macro_export]
macro_rules! kerror {
    ($($arg:tt)*) => ({
        if $crate::debug::should_log($crate::debug::LogLevel::Error) {
            $crate::kprintln!("[ERROR] {}", format_args!($($arg)*));
        }
    });
}

/// Log WARN message
#[macro_export]
macro_rules! kwarn {
    ($($arg:tt)*) => ({
        if $crate::debug::should_log($crate::debug::LogLevel::Warn) {
            $crate::kprintln!("[WARN]  {}", format_args!($($arg)*));
        }
    });
}

/// Log INFO message
#[macro_export]
macro_rules! kinfo {
    ($($arg:tt)*) => ({
        if $crate::debug::should_log($crate::debug::LogLevel::Info) {
            $crate::kprintln!("[INFO]  {}", format_args!($($arg)*));
        }
    });
}

/// Log DEBUG message
#[macro_export]
macro_rules! kdebug {
    ($($arg:tt)*) => ({
        if $crate::debug::should_log($crate::debug::LogLevel::Debug) {
            $crate::kprintln!("[DEBUG] {}", format_args!($($arg)*));
        }
    });
}

/// Log TRACE message
#[macro_export]
macro_rules! ktrace {
    ($($arg:tt)*) => ({
        if $crate::debug::should_log($crate::debug::LogLevel::Trace) {
            $crate::kprintln!("[TRACE] {}", format_args!($($arg)*));
        }
    });
}

/// Log syscall debug message (only when debug-syscall feature is enabled)
#[macro_export]
macro_rules! ksyscall_debug {
    ($($arg:tt)*) => ({
        #[cfg(feature = "debug-syscall")]
        {
            $crate::kprintln!($($arg)*);
        }
    });
}

/// Log scheduler debug message (only when debug-scheduler feature is enabled)
#[macro_export]
macro_rules! ksched_debug {
    ($($arg:tt)*) => ({
        #[cfg(feature = "debug-scheduler")]
        {
            $crate::kprintln!($($arg)*);
        }
    });
}
