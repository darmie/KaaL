//! Service Registry
//!
//! Manages service registration and discovery for IPC.
//! Allows producers (servers) to register services by name,
//! and consumers (clients) to discover them.

use crate::{Endpoint, Result, BrokerError};

/// Maximum number of registered services
const MAX_SERVICES: usize = 32;

/// Maximum service name length
const MAX_NAME_LEN: usize = 32;

/// A registered service
#[derive(Debug, Clone, Copy)]
pub struct ServiceRecord {
    /// Service name (null-terminated)
    name: [u8; MAX_NAME_LEN],
    /// Actual name length
    name_len: usize,
    /// Endpoint for this service
    endpoint: Endpoint,
    /// Process ID that registered this service
    owner_pid: usize,
    /// Is this slot allocated?
    allocated: bool,
}

impl ServiceRecord {
    fn new() -> Self {
        Self {
            name: [0; MAX_NAME_LEN],
            name_len: 0,
            endpoint: Endpoint { cap_slot: 0, id: 0 },
            owner_pid: 0,
            allocated: false,
        }
    }

    fn matches(&self, name: &str) -> bool {
        if !self.allocated || name.len() != self.name_len {
            return false;
        }
        &self.name[..self.name_len] == name.as_bytes()
    }

    fn name_str(&self) -> Option<&str> {
        if self.allocated {
            core::str::from_utf8(&self.name[..self.name_len]).ok()
        } else {
            None
        }
    }
}

/// Service Registry
///
/// Manages service registration and discovery.
pub struct ServiceRegistry {
    /// Registered services
    services: [ServiceRecord; MAX_SERVICES],
    /// Number of registered services
    num_services: usize,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub(crate) fn new() -> Self {
        Self {
            services: [ServiceRecord::new(); MAX_SERVICES],
            num_services: 0,
        }
    }

    /// Register a service
    ///
    /// # Arguments
    ///
    /// * `name` - Service name (must be unique)
    /// * `endpoint` - IPC endpoint for the service
    /// * `owner_pid` - Process ID of the service provider
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if:
    /// - Service name is too long
    /// - Service already registered
    /// - Registry is full
    pub(crate) fn register_service(
        &mut self,
        name: &str,
        endpoint: Endpoint,
        owner_pid: usize,
    ) -> Result<()> {
        // Validate name length
        if name.is_empty() || name.len() > MAX_NAME_LEN {
            return Err(BrokerError::InvalidCapability);
        }

        // Check if service already exists
        for service in &self.services {
            if service.matches(name) {
                return Err(BrokerError::ResourceInUse);
            }
        }

        // Find free slot
        for service in &mut self.services {
            if !service.allocated {
                service.name[..name.len()].copy_from_slice(name.as_bytes());
                service.name_len = name.len();
                service.endpoint = endpoint;
                service.owner_pid = owner_pid;
                service.allocated = true;
                self.num_services += 1;
                return Ok(());
            }
        }

        Err(BrokerError::OutOfCapabilitySlots)
    }

    /// Lookup a service by name
    ///
    /// # Arguments
    ///
    /// * `name` - Service name to lookup
    ///
    /// # Returns
    ///
    /// The service's endpoint, or an error if not found.
    pub(crate) fn lookup_service(&self, name: &str) -> Result<Endpoint> {
        for service in &self.services {
            if service.matches(name) {
                return Ok(service.endpoint);
            }
        }
        Err(BrokerError::DeviceNotFound)
    }

    /// Unregister a service
    ///
    /// # Arguments
    ///
    /// * `name` - Service name to unregister
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if service not found.
    pub(crate) fn unregister_service(&mut self, name: &str) -> Result<()> {
        for service in &mut self.services {
            if service.matches(name) {
                service.allocated = false;
                self.num_services -= 1;
                return Ok(());
            }
        }
        Err(BrokerError::DeviceNotFound)
    }

    /// Get number of registered services
    pub(crate) fn num_services(&self) -> usize {
        self.num_services
    }

    /// List all registered services
    ///
    /// Returns an iterator over (name, endpoint) pairs.
    pub(crate) fn list_services(&self) -> impl Iterator<Item = (&str, Endpoint)> {
        self.services
            .iter()
            .filter(|s| s.allocated)
            .filter_map(|s| s.name_str().map(|name| (name, s.endpoint)))
    }
}
