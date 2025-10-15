//! Capability management
//!
//! Higher-level abstractions for working with capabilities.

use crate::{Result, syscall};

/// Capability slot type
pub type CapSlot = usize;

/// Notification capability wrapper
///
/// Provides RAII-style management of notification capabilities.
///
/// # Example
/// ```no_run
/// use kaal_sdk::capability::Notification;
///
/// let notification = Notification::create()?;
/// notification.signal(0x1)?;
/// let signals = notification.poll()?;
/// ```
pub struct Notification {
    slot: CapSlot,
}

impl Notification {
    /// Create a new notification object
    pub fn create() -> Result<Self> {
        let slot = syscall::notification_create()?;
        Ok(Self { slot })
    }

    /// Get the capability slot
    pub fn slot(&self) -> CapSlot {
        self.slot
    }

    /// Signal this notification with a badge
    pub fn signal(&self, badge: u64) -> Result<()> {
        syscall::signal(self.slot, badge)
    }

    /// Wait for notification (blocking)
    pub fn wait(&self) -> Result<u64> {
        syscall::wait(self.slot)
    }

    /// Poll notification (non-blocking)
    pub fn poll(&self) -> Result<u64> {
        syscall::poll(self.slot)
    }
}

/// Endpoint capability wrapper
pub struct Endpoint {
    slot: CapSlot,
}

impl Endpoint {
    /// Create a new endpoint
    pub fn create() -> Result<Self> {
        let slot = syscall::endpoint_create()?;
        Ok(Self { slot })
    }

    /// Get the capability slot
    pub fn slot(&self) -> CapSlot {
        self.slot
    }
}

/// Device capability wrapper
pub struct Device {
    slot: CapSlot,
    device_id: usize,
}

impl Device {
    /// Request access to a device
    pub fn request(device_id: usize) -> Result<Self> {
        let slot = syscall::device_request(device_id)?;
        Ok(Self { slot, device_id })
    }

    /// Get the capability slot
    pub fn slot(&self) -> CapSlot {
        self.slot
    }

    /// Get the device ID
    pub fn device_id(&self) -> usize {
        self.device_id
    }
}
