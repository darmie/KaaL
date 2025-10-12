//! PL011 UART console component (minimal)
//!
//! This is a MINIMAL implementation for kernel debug output only.
//! It provides just enough functionality for `kprintln!` to work.
//!
//! For a FULL PL011 driver with interrupts, DMA, buffering, and flow control,
//! see runtime/components/drivers/uart_pl011 (user-space component).

use super::Console;
use core::ptr;

/// PL011 UART registers (minimal subset)
#[repr(C)]
struct Pl011Regs {
    dr: u32,          // 0x00: Data register
    _rsrecr: [u32; 5], // 0x04-0x14: Status/error registers (unused)
    fr: u32,          // 0x18: Flag register
}

/// PL011 console component configuration
#[derive(Clone, Copy)]
pub struct Pl011Config {
    /// Physical MMIO base address
    pub mmio_base: usize,
}

/// PL011 minimal console (kernel component)
///
/// This is a kernel component (Layer 0) providing MINIMAL console
/// functionality for debug output. It does NOT support:
/// - Interrupts (IRQs handled in user-space)
/// - DMA transfers (user-space driver feature)
/// - Buffering (user-space driver feature)
/// - Flow control (user-space driver feature)
/// - Baud rate configuration (assumes bootloader setup)
///
/// # Safety
/// This component directly accesses MMIO registers. The kernel must ensure
/// the MMIO region is correctly mapped before using this component.
pub struct Pl011Console {
    mmio_base: usize,
}

impl Pl011Console {
    /// Create a new PL011 console from configuration
    ///
    /// # Safety
    /// The caller must ensure the MMIO base address is valid and properly
    /// mapped in the kernel's address space.
    pub const fn new(config: Pl011Config) -> Self {
        Self {
            mmio_base: config.mmio_base,
        }
    }

    /// Initialize the PL011 UART (minimal setup)
    ///
    /// Assumes the bootloader or firmware has already configured:
    /// - Baud rate (usually 115200)
    /// - Word length (8 bits)
    /// - Parity (none)
    /// - Stop bits (1)
    ///
    /// This function just ensures the UART is enabled for TX.
    pub fn init(&self) {
        unsafe {
            let regs = self.mmio_base as *mut Pl011Regs;

            // For minimal console, we assume UART is already initialized
            // by firmware/bootloader. Just verify we can access it.

            // Read flag register to ensure UART is accessible
            let _flags = ptr::read_volatile(&(*regs).fr);
        }
    }

    /// Check if TX FIFO is full
    #[inline]
    fn tx_full(&self) -> bool {
        unsafe {
            let regs = self.mmio_base as *const Pl011Regs;
            let fr = ptr::read_volatile(&(*regs).fr);
            (fr & (1 << 5)) != 0 // TXFF bit
        }
    }
}

impl Console for Pl011Console {
    fn putc(&self, c: u8) {
        unsafe {
            let regs = self.mmio_base as *mut Pl011Regs;

            // Wait until TX FIFO not full
            while self.tx_full() {
                core::hint::spin_loop();
            }

            // Write character
            ptr::write_volatile(&mut (*regs).dr, c as u32);
        }
    }
}
