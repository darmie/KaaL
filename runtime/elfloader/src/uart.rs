// UART driver for debug output

use core::fmt::{self, Write};
use spin::Mutex;

/// PL011 UART base address for QEMU ARM virt platform
const UART_BASE: usize = 0x0900_0000;

/// PL011 UART registers
#[repr(C)]
struct Pl011Regs {
    dr: u32,      // Data register
    _rsrecr: u32,
    _reserved1: [u32; 4],
    fr: u32,      // Flag register
    _reserved2: [u32; 2],
    ibrd: u32,    // Integer baud rate divisor
    fbrd: u32,    // Fractional baud rate divisor
    lcrh: u32,    // Line control register
    cr: u32,      // Control register
}

impl Pl011Regs {
    fn is_tx_full(&self) -> bool {
        (self.fr & (1 << 5)) != 0
    }

    fn putc(&mut self, c: u8) {
        while self.is_tx_full() {
            core::hint::spin_loop();
        }
        self.dr = c as u32;
    }
}

struct UartWriter {
    regs: &'static mut Pl011Regs,
}

impl UartWriter {
    fn new() -> Self {
        let regs = unsafe { &mut *(UART_BASE as *mut Pl011Regs) };
        Self { regs }
    }

    fn write_byte(&mut self, byte: u8) {
        if byte == b'\n' {
            self.regs.putc(b'\r');
        }
        self.regs.putc(byte);
    }
}

impl Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

static UART: Mutex<Option<UartWriter>> = Mutex::new(None);

/// Initialize UART
pub fn init() {
    let mut uart = UART.lock();
    *uart = Some(UartWriter::new());
}

/// Print to UART
pub fn print(args: fmt::Arguments) {
    if let Some(ref mut uart) = *UART.lock() {
        let _ = uart.write_fmt(args);
    }
}

#[macro_export]
macro_rules! uart_print {
    ($($arg:tt)*) => ($crate::uart::print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! uart_println {
    () => ($crate::uart_print!("\n"));
    ($($arg:tt)*) => ($crate::uart_print!("{}\n", format_args!($($arg)*)));
}

// Re-export macros
pub use uart_print as print_macro;
pub use uart_println as println;
