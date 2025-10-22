//! ARM PL011 UART Hardware Interface
//!
//! This module provides low-level access to the PL011 UART hardware.
//! Reference: ARM PrimeCell UART (PL011) Technical Reference Manual

use core::ptr::{read_volatile, write_volatile};

/// PL011 UART Register offsets
const UARTDR: usize = 0x000;     // Data Register
const UARTFR: usize = 0x018;     // Flag Register
const UARTIBRD: usize = 0x024;   // Integer Baud Rate Divisor
const UARTFBRD: usize = 0x028;   // Fractional Baud Rate Divisor
const UARTLCR_H: usize = 0x02C;  // Line Control Register
const UARTCR: usize = 0x030;     // Control Register
const UARTIMSC: usize = 0x038;   // Interrupt Mask Set/Clear
const UARTRIS: usize = 0x03C;    // Raw Interrupt Status
const UARTMIS: usize = 0x040;    // Masked Interrupt Status
const UARTICR: usize = 0x044;    // Interrupt Clear Register

/// Flag Register bits
const FR_TXFF: u32 = 1 << 5;     // Transmit FIFO full
const FR_RXFE: u32 = 1 << 4;     // Receive FIFO empty
const FR_BUSY: u32 = 1 << 3;     // UART busy

/// Line Control Register bits
const LCR_H_WLEN_8: u32 = 0x60;  // 8-bit word length
const LCR_H_FEN: u32 = 1 << 4;   // Enable FIFOs

/// Control Register bits
const CR_UARTEN: u32 = 1 << 0;   // UART enable
const CR_TXE: u32 = 1 << 8;      // Transmit enable
const CR_RXE: u32 = 1 << 9;      // Receive enable

/// Interrupt bits
const INT_RX: u32 = 1 << 4;      // Receive interrupt
const INT_TX: u32 = 1 << 5;      // Transmit interrupt
const INT_RT: u32 = 1 << 6;      // Receive timeout
const INT_OE: u32 = 1 << 10;     // Overrun error

/// PL011 UART driver
pub struct Pl011 {
    base: usize,
}

impl Pl011 {
    /// Create a new PL011 UART driver
    ///
    /// # Safety
    /// The caller must ensure that `base` points to valid PL011 UART MMIO registers
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }

    /// Initialize the UART
    ///
    /// Configures:
    /// - 115200 baud (assuming 24MHz UARTCLK)
    /// - 8 data bits, no parity, 1 stop bit
    /// - FIFOs enabled
    /// - RX interrupts enabled
    ///
    /// # Safety
    /// Must be called before any other UART operations
    pub unsafe fn init(&mut self) {
        // Disable UART
        self.write_reg(UARTCR, 0);

        // Wait for end of transmission
        while self.read_reg(UARTFR) & FR_BUSY != 0 {}

        // Flush FIFOs by disabling them
        self.write_reg(UARTLCR_H, 0);

        // Set baud rate to 115200
        // UARTCLK = 24 MHz (QEMU virt default)
        // Baud rate divisor = UARTCLK / (16 * baud_rate)
        //                   = 24000000 / (16 * 115200)
        //                   = 13.02 = 13 + 0.02
        // Integer part: 13
        // Fractional part: 0.02 * 64 = 1
        self.write_reg(UARTIBRD, 13);   // Integer baud rate divisor
        self.write_reg(UARTFBRD, 1);    // Fractional baud rate divisor

        // Configure line control: 8N1, enable FIFOs
        self.write_reg(UARTLCR_H, LCR_H_WLEN_8 | LCR_H_FEN);

        // Enable RX interrupts (and receive timeout)
        self.write_reg(UARTIMSC, INT_RX | INT_RT | INT_OE);

        // Enable UART, TX, and RX
        self.write_reg(UARTCR, CR_UARTEN | CR_TXE | CR_RXE);
    }

    /// Read a register
    #[inline]
    unsafe fn read_reg(&self, offset: usize) -> u32 {
        read_volatile((self.base + offset) as *const u32)
    }

    /// Write a register
    #[inline]
    unsafe fn write_reg(&mut self, offset: usize, value: u32) {
        write_volatile((self.base + offset) as *mut u32, value);
    }

    /// Check if transmit FIFO is full
    pub fn tx_full(&self) -> bool {
        unsafe { self.read_reg(UARTFR) & FR_TXFF != 0 }
    }

    /// Check if receive FIFO is empty
    pub fn rx_empty(&self) -> bool {
        unsafe { self.read_reg(UARTFR) & FR_RXFE != 0 }
    }

    /// Write a byte to the UART (blocking)
    pub fn write_byte(&mut self, byte: u8) {
        // Wait until TX FIFO has space
        while self.tx_full() {}

        unsafe {
            self.write_reg(UARTDR, byte as u32);
        }
    }

    /// Read a byte from the UART (non-blocking)
    ///
    /// Returns `Some(byte)` if data is available, `None` otherwise
    pub fn read_byte(&mut self) -> Option<u8> {
        if self.rx_empty() {
            None
        } else {
            unsafe {
                Some(self.read_reg(UARTDR) as u8)
            }
        }
    }

    /// Write a string to the UART
    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            // Convert LF to CRLF
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }

    /// Get raw interrupt status
    pub fn interrupt_status(&self) -> u32 {
        unsafe { self.read_reg(UARTRIS) }
    }

    /// Get masked interrupt status
    pub fn masked_interrupt_status(&self) -> u32 {
        unsafe { self.read_reg(UARTMIS) }
    }

    /// Clear interrupts
    pub fn clear_interrupts(&mut self, mask: u32) {
        unsafe {
            self.write_reg(UARTICR, mask);
        }
    }

    /// Check if RX interrupt is pending
    pub fn has_rx_interrupt(&self) -> bool {
        self.masked_interrupt_status() & (INT_RX | INT_RT) != 0
    }

    /// Clear RX interrupts
    pub fn clear_rx_interrupts(&mut self) {
        self.clear_interrupts(INT_RX | INT_RT | INT_OE);
    }
}

// PL011 is safe to send between threads (no shared mutable state)
unsafe impl Send for Pl011 {}
