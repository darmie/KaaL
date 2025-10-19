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
pub mod service_registry;

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

/// Capability allocation record
#[derive(Debug, Clone, Copy)]
struct CapabilityRecord {
    /// Capability slot number
    slot: usize,
    /// Type of capability
    cap_type: CapabilityType,
    /// Is this capability currently allocated?
    allocated: bool,
}

/// Type of capability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapabilityType {
    /// Memory capability
    Memory,
    /// Device capability
    Device,
    /// IPC endpoint capability
    Endpoint,
    /// Untyped/free slot
    Untyped,
}

const MAX_CAPABILITY_RECORDS: usize = 256;

/// The Capability Broker
///
/// This is the main entry point for managing kernel capabilities in userspace.
/// It provides a clean API for device allocation, memory management, and IPC.
pub struct CapabilityBroker {
    /// Next free capability slot
    next_cap_slot: usize,
    /// Maximum capability slot
    max_cap_slot: usize,
    /// Capability allocation records
    cap_records: [Option<CapabilityRecord>; MAX_CAPABILITY_RECORDS],
    /// Number of allocated capabilities
    num_allocated_caps: usize,
    /// Device manager
    device_manager: device_manager::DeviceManager,
    /// Memory manager
    memory_manager: memory_manager::MemoryManager,
    /// Endpoint manager
    endpoint_manager: endpoint_manager::EndpointManager,
    /// Service registry for IPC discovery
    service_registry: service_registry::ServiceRegistry,
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
        // Read boot info from kernel-mapped address
        let boot_info =
            unsafe { boot_info::BootInfo::read().ok_or(BrokerError::SyscallFailed(0))? };

        // Start capability slots after initial caps
        let next_cap_slot = if boot_info.num_initial_caps > 0 {
            (boot_info.num_initial_caps as usize) + 100
        } else {
            100
        };
        let max_cap_slot = 4096;

        Ok(Self {
            next_cap_slot,
            max_cap_slot,
            cap_records: [None; MAX_CAPABILITY_RECORDS],
            num_allocated_caps: 0,
            device_manager: device_manager::DeviceManager::new_from_boot_info(boot_info),
            memory_manager: memory_manager::MemoryManager::new_from_boot_info(boot_info),
            endpoint_manager: endpoint_manager::EndpointManager::new(),
            service_registry: service_registry::ServiceRegistry::new(),
        })
    }

    /// Allocate a new capability slot
    ///
    /// Returns the next available capability slot number, or an error if no slots are available.
    fn allocate_cap_slot(&mut self, cap_type: CapabilityType) -> Result<usize> {
        if self.next_cap_slot >= self.max_cap_slot {
            return Err(BrokerError::OutOfCapabilitySlots);
        }

        let slot = self.next_cap_slot;
        self.next_cap_slot += 1;

        // Record the capability allocation
        if self.num_allocated_caps < MAX_CAPABILITY_RECORDS {
            self.cap_records[self.num_allocated_caps] = Some(CapabilityRecord {
                slot,
                cap_type,
                allocated: true,
            });
            self.num_allocated_caps += 1;
        }

        Ok(slot)
    }

    /// Get statistics about capability usage
    ///
    /// Returns (allocated_count, total_capacity)
    pub fn capability_stats(&self) -> (usize, usize) {
        let allocated = self.cap_records[..self.num_allocated_caps]
            .iter()
            .filter(|r| r.map(|rec| rec.allocated).unwrap_or(false))
            .count();
        (allocated, self.max_cap_slot)
    }

    /// Get capability usage by type
    ///
    /// Returns (memory_caps, device_caps, endpoint_caps, untyped_caps)
    pub fn capability_usage_by_type(&self) -> (usize, usize, usize, usize) {
        let mut memory = 0;
        let mut device = 0;
        let mut endpoint = 0;
        let mut untyped = 0;

        for record in &self.cap_records[..self.num_allocated_caps] {
            if let Some(rec) = record {
                if rec.allocated {
                    match rec.cap_type {
                        CapabilityType::Memory => memory += 1,
                        CapabilityType::Device => device += 1,
                        CapabilityType::Endpoint => endpoint += 1,
                        CapabilityType::Untyped => untyped += 1,
                    }
                }
            }
        }

        (memory, device, endpoint, untyped)
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
        let irq_cap = self.allocate_cap_slot(CapabilityType::Device).ok();
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
        let cap_slot = self.allocate_cap_slot(CapabilityType::Memory)?;
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
        let cap_slot = self.allocate_cap_slot(CapabilityType::Endpoint)?;
        self.endpoint_manager.create_endpoint(cap_slot)
    }

    /// Register a service with the broker
    ///
    /// Allows a service provider (server) to register itself by name,
    /// so consumers (clients) can discover it.
    ///
    /// # Arguments
    ///
    /// * `name` - Service name (must be unique, max 32 characters)
    /// * `endpoint` - IPC endpoint for this service
    /// * `owner_pid` - Process ID of the service provider
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if:
    /// - Service name already registered
    /// - Service name too long
    /// - Registry full
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use capability_broker::CapabilityBroker;
    ///
    /// let mut broker = CapabilityBroker::init()?;
    /// let endpoint = broker.create_endpoint()?;
    /// broker.register_service("printer", endpoint, 42)?;
    /// ```
    pub fn register_service(&mut self, name: &str, endpoint: Endpoint, owner_pid: usize) -> Result<()> {
        self.service_registry.register_service(name, endpoint, owner_pid)
    }

    /// Lookup a service by name
    ///
    /// Allows a consumer (client) to discover a service provider by name.
    ///
    /// # Arguments
    ///
    /// * `name` - Service name to lookup
    ///
    /// # Returns
    ///
    /// The service's endpoint on success, or an error if not found.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use capability_broker::CapabilityBroker;
    ///
    /// let broker = CapabilityBroker::init()?;
    /// let printer_endpoint = broker.lookup_service("printer")?;
    /// // Use endpoint to communicate with printer service
    /// ```
    pub fn lookup_service(&self, name: &str) -> Result<Endpoint> {
        self.service_registry.lookup_service(name)
    }

    /// Unregister a service
    ///
    /// Removes a service from the registry.
    ///
    /// # Arguments
    ///
    /// * `name` - Service name to unregister
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if service not found.
    pub fn unregister_service(&mut self, name: &str) -> Result<()> {
        self.service_registry.unregister_service(name)
    }

    /// Get number of registered services
    pub fn num_services(&self) -> usize {
        self.service_registry.num_services()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_cap_slot() {
        let mut broker = CapabilityBroker::init().unwrap();

        let slot1 = broker.allocate_cap_slot(CapabilityType::Device).unwrap();
        let slot2 = broker.allocate_cap_slot(CapabilityType::Memory).unwrap();

        assert_eq!(slot1, 100);
        assert_eq!(slot2, 101);
    }
}
