//! KaaL Capability Broker
//!
//! The Capability Broker is a userspace runtime service that provides a clean API
//! for managing kernel capabilities. It hides the complexity of the KaaL microkernel's
//! capability system from application developers.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │      Application / Driver Code          │
//! └──────────────┬──────────────────────────┘
//!                │ Clean API
//! ┌──────────────▼──────────────────────────┐
//! │     Capability Broker (this crate)      │
//! │  • Device Manager                       │
//! │  • Memory Manager                       │
//! │  • Endpoint Manager                     │
//! └──────────────┬──────────────────────────┘
//!                │ Syscalls
//! ┌──────────────▼──────────────────────────┐
//! │      KaaL Microkernel (EL1)             │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - **Device Management**: Allocate MMIO regions, IRQs, DMA buffers
//! - **Memory Management**: Request physical/virtual memory from kernel
//! - **Endpoint Management**: Create IPC endpoints for communication
//! - **Capability Tracking**: Track and manage capability slots
//!
//! # Usage
//!
//! ```rust,no_run
//! use capability_broker::CapabilityBroker;
//!
//! // Initialize the broker (typically done in root task)
//! let mut broker = CapabilityBroker::init()?;
//!
//! // Request a device (e.g., UART)
//! let uart_device = broker.request_device(DeviceId::Uart(0))?;
//! // uart_device now contains MMIO region, IRQ capability, etc.
//!
//! // Allocate memory
//! let mem_region = broker.allocate_memory(4096)?;
//!
//! // Create IPC endpoint
//! let endpoint = broker.create_endpoint()?;
//! ```

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod boot_info;

pub mod device_manager;
pub mod endpoint_manager;
pub mod memory_manager;

pub use device_manager::{DeviceId, DeviceResource};
pub use endpoint_manager::Endpoint;
pub use memory_manager::MemoryRegion;

/// Errors that can occur in the Capability Broker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerError {
    /// Capability slot allocation failed (out of slots)
    OutOfCapabilitySlots,
    /// Requested device not found or already allocated
    DeviceNotFound,
    /// Memory allocation failed (out of memory)
    OutOfMemory,
    /// Invalid capability operation
    InvalidCapability,
    /// Syscall failed
    SyscallFailed(usize),
    /// Resource already in use
    ResourceInUse,
}

/// Result type for Capability Broker operations
pub type Result<T> = core::result::Result<T, BrokerError>;

/// The Capability Broker
///
/// This is the main entry point for managing kernel capabilities in userspace.
/// It provides a clean API for device allocation, memory management, and IPC.
pub struct CapabilityBroker {
    /// Next free capability slot
    next_cap_slot: usize,
    /// Maximum capability slot
    max_cap_slot: usize,
    /// Device manager
    device_manager: device_manager::DeviceManager,
    /// Memory manager
    memory_manager: memory_manager::MemoryManager,
    /// Endpoint manager
    endpoint_manager: endpoint_manager::EndpointManager,
}

impl CapabilityBroker {
    /// Initialize the Capability Broker
    ///
    /// This should be called early in the root task initialization.
    /// It queries the kernel for available resources and sets up internal state.
    ///
    /// # Returns
    ///
    /// Returns a new `CapabilityBroker` instance, or an error if initialization fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use capability_broker::CapabilityBroker;
    ///
    /// let mut broker = CapabilityBroker::init()?;
    /// ```
    pub fn init() -> Result<Self> {
        // TODO: Query kernel for boot info
        // - Available capability slots
        // - Memory regions
        // - Device tree

        // For now, use hardcoded values
        let next_cap_slot = 100; // Start after kernel-reserved caps
        let max_cap_slot = 4096; // Arbitrary limit

        Ok(Self {
            next_cap_slot,
            max_cap_slot,
            device_manager: device_manager::DeviceManager::new(),
            memory_manager: memory_manager::MemoryManager::new(),
            endpoint_manager: endpoint_manager::EndpointManager::new(),
        })
    }

    /// Allocate a new capability slot
    ///
    /// Returns the next available capability slot number, or an error if no slots are available.
    fn allocate_cap_slot(&mut self) -> Result<usize> {
        if self.next_cap_slot >= self.max_cap_slot {
            return Err(BrokerError::OutOfCapabilitySlots);
        }

        let slot = self.next_cap_slot;
        self.next_cap_slot += 1;
        Ok(slot)
    }

    /// Request a device resource
    ///
    /// Allocates all resources needed for the specified device (MMIO, IRQ, DMA).
    ///
    /// # Arguments
    ///
    /// * `device_id` - Identifier for the device to allocate
    ///
    /// # Returns
    ///
    /// Returns a `DeviceResource` containing all allocated resources, or an error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use capability_broker::{CapabilityBroker, DeviceId};
    ///
    /// let mut broker = CapabilityBroker::init()?;
    /// let uart = broker.request_device(DeviceId::Uart(0))?;
    /// // Use uart.mmio_base, uart.irq_cap, etc.
    /// ```
    pub fn request_device(&mut self, device_id: DeviceId) -> Result<DeviceResource> {
        // Allocate IRQ capability slot if needed
        let irq_cap = self.allocate_cap_slot().ok();
        self.device_manager.request_device(device_id, irq_cap)
    }

    /// Allocate a memory region
    ///
    /// Requests the specified amount of physical memory from the kernel.
    ///
    /// # Arguments
    ///
    /// * `size` - Size in bytes (will be rounded up to page size)
    ///
    /// # Returns
    ///
    /// Returns a `MemoryRegion` describing the allocated memory, or an error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use capability_broker::CapabilityBroker;
    ///
    /// let mut broker = CapabilityBroker::init()?;
    /// let mem = broker.allocate_memory(4096)?; // Allocate 4KB
    /// ```
    pub fn allocate_memory(&mut self, size: usize) -> Result<MemoryRegion> {
        let cap_slot = self.allocate_cap_slot()?;
        self.memory_manager.allocate(size, cap_slot)
    }

    /// Create an IPC endpoint
    ///
    /// Creates a new IPC endpoint for communication between components.
    ///
    /// # Returns
    ///
    /// Returns an `Endpoint` that can be used for IPC, or an error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use capability_broker::CapabilityBroker;
    ///
    /// let mut broker = CapabilityBroker::init()?;
    /// let endpoint = broker.create_endpoint()?;
    /// // Use endpoint for send/recv operations
    /// ```
    pub fn create_endpoint(&mut self) -> Result<Endpoint> {
        let cap_slot = self.allocate_cap_slot()?;
        self.endpoint_manager.create_endpoint(cap_slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_cap_slot() {
        let mut broker = CapabilityBroker::init().unwrap();

        let slot1 = broker.allocate_cap_slot().unwrap();
        let slot2 = broker.allocate_cap_slot().unwrap();

        assert_eq!(slot1, 100);
        assert_eq!(slot2, 101);
    }
}
