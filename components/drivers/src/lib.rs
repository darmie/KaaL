//! Device Drivers - Hardware device drivers
//!
//! # Purpose
//! Collection of device drivers for common hardware
//! (serial, block devices, network cards, etc.)
//!
//! # Integration Points
//! - Depends on: DDDK, Capability Broker
//! - Provides to: VFS, Network stack
//! - Capabilities required: Device-specific (MMIO, IRQ, DMA)
//!
//! # Architecture
//! - Modular driver framework
//! - Common driver traits
//! - Device discovery and initialization
//! - Interrupt handling
//!
//! # Testing Strategy
//! - Unit tests: Driver logic
//! - Integration tests: With real/simulated hardware
//! - Hardware sim tests: QEMU device models

use thiserror::Error;

/// Driver error types
#[derive(Debug, Error)]
pub enum DriverError {
    #[error("Device not found")]
    DeviceNotFound,

    #[error("Hardware error: {0}")]
    HardwareError(String),

    #[error("Initialization failed: {0}")]
    InitFailed(String),
}

pub type Result<T> = core::result::Result<T, DriverError>;

/// Generic driver trait
pub trait Driver {
    /// Initialize the driver
    fn init(&mut self) -> Result<()>;

    /// Handle device interrupt
    fn handle_interrupt(&mut self);
}

/// Block device trait
pub trait BlockDevice {
    /// Read blocks
    fn read_blocks(&mut self, start: u64, count: usize, buf: &mut [u8]) -> Result<usize>;

    /// Write blocks
    fn write_blocks(&mut self, start: u64, buf: &[u8]) -> Result<usize>;

    /// Get block size
    fn block_size(&self) -> usize;
}

/// Network device trait
pub trait NetworkDevice {
    /// Transmit packet
    fn transmit(&mut self, packet: &[u8]) -> Result<()>;

    /// Receive packet
    fn receive(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Get MAC address
    fn mac_address(&self) -> [u8; 6];
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBlockDevice;

    impl BlockDevice for MockBlockDevice {
        fn read_blocks(&mut self, _start: u64, _count: usize, _buf: &mut [u8]) -> Result<usize> {
            Ok(0)
        }

        fn write_blocks(&mut self, _start: u64, _buf: &[u8]) -> Result<usize> {
            Ok(0)
        }

        fn block_size(&self) -> usize {
            512
        }
    }

    #[test]
    fn test_mock_block_device() {
        let mut dev = MockBlockDevice;
        assert_eq!(dev.block_size(), 512);
    }
}
