//! DDDK Runtime - Runtime support for device driver development
//!
//! This crate provides the runtime types and traits used by the DDDK
//! procedural macros. It includes error types, driver traits, and
//! resource abstractions.

use thiserror::Error;

pub use cap_broker::DeviceId;

/// Driver error types
#[derive(Debug, Error)]
pub enum DriverError {
    #[error("Resource allocation failed: {0}")]
    ResourceAllocation(String),

    #[error("Driver initialization failed: {0}")]
    Initialization(String),

    #[error("IRQ registration failed: {0}")]
    IrqRegistration(String),

    #[error("DMA allocation failed: {0}")]
    DmaAllocation(String),

    #[error("MMIO mapping failed: {0}")]
    MmioMapping(String),

    #[error("Feature not yet implemented")]
    NotImplemented,

    #[error("Device error: {0}")]
    DeviceError(String),
}

pub type Result<T> = core::result::Result<T, DriverError>;

/// Trait for driver metadata
///
/// Automatically implemented by the #[derive(Driver)] macro
pub trait DriverMetadata {
    /// Get the device ID this driver supports
    fn device_id() -> DeviceId;

    /// Get the driver name
    fn driver_name() -> &'static str;

    /// Get driver version (default: "0.1.0")
    fn driver_version() -> &'static str {
        "0.1.0"
    }
}

/// Trait for driver lifecycle
pub trait Driver: DriverMetadata {
    /// Probe and initialize the driver
    fn probe(broker: &mut dyn cap_broker::CapabilityBroker) -> Result<Self>
    where
        Self: Sized;

    /// Start the driver (begin operations)
    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    /// Stop the driver (suspend operations)
    fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// Shutdown the driver (cleanup resources)
    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// MMIO register accessor
///
/// Provides volatile read/write operations to memory-mapped I/O regions
pub struct MmioRegion {
    base: usize,
    size: usize,
}

impl MmioRegion {
    /// Create a new MMIO region
    ///
    /// # Safety
    /// Caller must ensure the memory region is valid MMIO space
    pub unsafe fn new(base: usize, size: usize) -> Self {
        Self { base, size }
    }

    /// Read a 32-bit value from offset
    ///
    /// # Safety
    /// Caller must ensure offset is within region and properly aligned
    pub unsafe fn read_u32(&self, offset: usize) -> u32 {
        debug_assert!(offset + 4 <= self.size);
        debug_assert!(offset % 4 == 0);
        core::ptr::read_volatile((self.base + offset) as *const u32)
    }

    /// Write a 32-bit value to offset
    ///
    /// # Safety
    /// Caller must ensure offset is within region and properly aligned
    pub unsafe fn write_u32(&mut self, offset: usize, value: u32) {
        debug_assert!(offset + 4 <= self.size);
        debug_assert!(offset % 4 == 0);
        core::ptr::write_volatile((self.base + offset) as *mut u32, value);
    }

    /// Read a 16-bit value from offset
    pub unsafe fn read_u16(&self, offset: usize) -> u16 {
        debug_assert!(offset + 2 <= self.size);
        debug_assert!(offset % 2 == 0);
        core::ptr::read_volatile((self.base + offset) as *const u16)
    }

    /// Write a 16-bit value to offset
    pub unsafe fn write_u16(&mut self, offset: usize, value: u16) {
        debug_assert!(offset + 2 <= self.size);
        debug_assert!(offset % 2 == 0);
        core::ptr::write_volatile((self.base + offset) as *mut u16, value);
    }

    /// Read an 8-bit value from offset
    pub unsafe fn read_u8(&self, offset: usize) -> u8 {
        debug_assert!(offset + 1 <= self.size);
        core::ptr::read_volatile((self.base + offset) as *const u8)
    }

    /// Write an 8-bit value to offset
    pub unsafe fn write_u8(&mut self, offset: usize, value: u8) {
        debug_assert!(offset + 1 <= self.size);
        core::ptr::write_volatile((self.base + offset) as *mut u8, value);
    }

    /// Get base address
    pub fn base(&self) -> usize {
        self.base
    }

    /// Get size
    pub fn size(&self) -> usize {
        self.size
    }
}

/// DMA buffer abstraction
pub struct DmaBuffer {
    vaddr: usize,
    paddr: usize,
    size: usize,
}

impl DmaBuffer {
    /// Create a new DMA buffer
    ///
    /// # Safety
    /// Caller must ensure addresses are valid and size is correct
    pub unsafe fn new(vaddr: usize, paddr: usize, size: usize) -> Self {
        Self { vaddr, paddr, size }
    }

    /// Get virtual address
    pub fn vaddr(&self) -> usize {
        self.vaddr
    }

    /// Get physical address (for DMA operations)
    pub fn paddr(&self) -> usize {
        self.paddr
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get buffer as mutable slice
    ///
    /// # Safety
    /// Caller must ensure no aliasing violations
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        core::slice::from_raw_parts_mut(self.vaddr as *mut u8, self.size)
    }

    /// Get buffer as slice
    pub unsafe fn as_slice(&self) -> &[u8] {
        core::slice::from_raw_parts(self.vaddr as *const u8, self.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmio_region_bounds() {
        unsafe {
            let region = MmioRegion::new(0x1000, 256);
            assert_eq!(region.base(), 0x1000);
            assert_eq!(region.size(), 256);
        }
    }

    #[test]
    fn test_dma_buffer_properties() {
        unsafe {
            let buffer = DmaBuffer::new(0x1000, 0x2000, 4096);
            assert_eq!(buffer.vaddr(), 0x1000);
            assert_eq!(buffer.paddr(), 0x2000);
            assert_eq!(buffer.size(), 4096);
        }
    }
}
