//! Device Manager
//!
//! Manages device resource allocation (MMIO regions, IRQs, DMA buffers).

use crate::{BrokerError, Result, boot_info::BootInfo};

/// Device identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceId {
    /// UART device (port number)
    Uart(usize),
    /// Timer device
    Timer,
    /// RTC device
    Rtc,
    /// Custom device (device_type from boot info)
    Custom(u32),
}

/// Device resource bundle
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
pub struct DeviceManager {
    /// Copy of boot info for device lookups
    boot_info: Option<&'static BootInfo>,
}

impl DeviceManager {
    /// Create a new Device Manager from boot info
    pub(crate) fn new_from_boot_info(boot_info: &'static BootInfo) -> Self {
        Self {
            boot_info: Some(boot_info),
        }
    }

    /// Create a new Device Manager (legacy, for tests)
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self { boot_info: None }
    }

    /// Request a device
    pub(crate) fn request_device(
        &mut self,
        device_id: DeviceId,
        irq_cap: Option<usize>,
    ) -> Result<DeviceResource> {
        let boot_info = self.boot_info.ok_or(BrokerError::DeviceNotFound)?;

        // Map DeviceId to device_type from boot info
        let device_type = match device_id {
            DeviceId::Uart(0) => 0, // DEVICE_UART0
            DeviceId::Uart(1) => 1, // DEVICE_UART1
            DeviceId::Rtc => 2,     // DEVICE_RTC
            DeviceId::Timer => 3,   // DEVICE_TIMER
            DeviceId::Custom(dt) => dt,
            _ => return Err(BrokerError::DeviceNotFound),
        };

        // Find device region in boot info
        let device = boot_info
            .find_device(device_type)
            .ok_or(BrokerError::DeviceNotFound)?;

        Ok(DeviceResource {
            mmio_base: device.paddr as usize,
            mmio_size: device.size as usize,
            irq_cap,
            dma_cap: None, // DMA not implemented yet
        })
    }
}
