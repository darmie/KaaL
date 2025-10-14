//! Device Manager
//!
//! Manages device resource allocation (MMIO regions, IRQs, DMA buffers).

use crate::{BrokerError, Result};

/// Device identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceId {
    /// UART device (port number)
    Uart(usize),
    /// Timer device
    Timer,
    /// GPIO controller
    Gpio,
    /// Custom device (vendor_id, device_id)
    Custom(u32, u32),
}

/// Device resource bundle
///
/// Contains all resources allocated for a device.
#[derive(Debug)]
pub struct DeviceResource {
    /// MMIO base address
    pub mmio_base: usize,
    /// MMIO size in bytes
    pub mmio_size: usize,
    /// IRQ capability slot (if applicable)
    pub irq_cap: Option<usize>,
    /// DMA buffer capability slot (if applicable)
    pub dma_cap: Option<usize>,
}

/// Device Manager
///
/// Tracks allocated devices and provides device resource allocation.
pub struct DeviceManager {
    // TODO: Track allocated devices to prevent double-allocation
}

impl DeviceManager {
    /// Create a new Device Manager
    pub(crate) fn new() -> Self {
        Self {}
    }

    /// Request a device
    ///
    /// Allocates all resources needed for the specified device.
    pub(crate) fn request_device(
        &mut self,
        device_id: DeviceId,
        irq_cap: Option<usize>,
    ) -> Result<DeviceResource> {
        // TODO: Query kernel for device information from device tree
        // TODO: Track allocated device to prevent double-allocation

        match device_id {
            DeviceId::Uart(port) => {
                // QEMU virt platform UART0 base address
                let mmio_base = 0x0900_0000 + (port * 0x1000);
                let mmio_size = 0x1000; // 4KB

                Ok(DeviceResource {
                    mmio_base,
                    mmio_size,
                    irq_cap,
                    dma_cap: None,
                })
            }
            DeviceId::Timer => {
                // TODO: Implement timer device allocation
                Err(BrokerError::DeviceNotFound)
            }
            DeviceId::Gpio => {
                // TODO: Implement GPIO device allocation
                Err(BrokerError::DeviceNotFound)
            }
            DeviceId::Custom(_, _) => {
                // TODO: Implement custom device allocation
                Err(BrokerError::DeviceNotFound)
            }
        }
    }
}
