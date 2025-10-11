//\! PL011 UART driver for ARM64
//\!
//\! This is a minimal UART driver for serial console output.
//\! Used for debug printing during early boot.

use core::ptr;

/// PL011 UART registers
#[repr(C)]
struct Pl011Regs {
    dr: u32,
    rsr_ecr: u32,
    _reserved1: [u32; 4],
    fr: u32,
    _reserved2: u32,
    ilpr: u32,
    ibrd: u32,
    fbrd: u32,
    lcrh: u32,
    cr: u32,
}

/// UART base address for QEMU virt
const UART_BASE: usize = 0x09000000;

/// Initialize UART for output
pub fn init() {
    let uart = UART_BASE as *mut Pl011Regs;
    unsafe {
        // Disable UART
        ptr::write_volatile(&mut (*uart).cr, 0);

        // Wait for transmit to finish
        while (ptr::read_volatile(&(*uart).fr) & (1 << 3)) \!= 0 {}

        // Configure: 8n1, FIFO enabled
        ptr::write_volatile(&mut (*uart).lcrh, (1 << 4) | (3 << 5));

        // Enable UART (TX and RX)
        ptr::write_volatile(&mut (*uart).cr, (1 << 0) | (1 << 8) | (1 << 9));
    }
}

/// Write a single byte to UART
pub fn putc(c: u8) {
    let uart = UART_BASE as *mut Pl011Regs;
    unsafe {
        // Wait until TX FIFO not full
        while (ptr::read_volatile(&(*uart).fr) & (1 << 5)) \!= 0 {}

        // Write byte
        ptr::write_volatile(&mut (*uart).dr, c as u32);
    }
}

/// Write a string to UART
pub fn puts(s: &str) {
    for byte in s.bytes() {
        if byte == b'\n' {
            putc(b'\r');
        }
        putc(byte);
    }
}
