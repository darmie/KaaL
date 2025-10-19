//! Shared Memory Registry
//!
//! Coordinates shared memory allocation and discovery between processes.
//! Allows producers to register physical memory regions under a named channel,
//! and consumers to query those registrations.

use alloc::collections::BTreeMap;
use alloc::string::String;

/// Shared memory registration entry
#[derive(Debug, Clone)]
pub struct ShmemEntry {
    /// Physical address of the shared memory region
    pub phys_addr: usize,
    /// Size of the region in bytes
    pub size: usize,
    /// Process ID that registered this memory (for cleanup)
    pub owner_pid: usize,
}

/// Shared Memory Registry
///
/// Maintains a mapping of channel names to physical memory regions.
/// This enables dynamic IPC channel establishment without hardcoded addresses.
pub struct ShmemRegistry {
    /// Map of channel name to shared memory entry
    entries: BTreeMap<String, ShmemEntry>,
}

impl ShmemRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        ShmemRegistry {
            entries: BTreeMap::new(),
        }
    }

    /// Register a shared memory region under a channel name
    ///
    /// # Arguments
    /// * `channel_name` - Unique identifier for the channel
    /// * `phys_addr` - Physical address of the shared memory
    /// * `size` - Size of the region in bytes
    /// * `owner_pid` - Process ID of the registering process
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(&str)` if channel name already exists
    pub fn register(
        &mut self,
        channel_name: String,
        phys_addr: usize,
        size: usize,
        owner_pid: usize,
    ) -> Result<(), &'static str> {
        if self.entries.contains_key(&channel_name) {
            return Err("Channel name already registered");
        }

        self.entries.insert(
            channel_name,
            ShmemEntry {
                phys_addr,
                size,
                owner_pid,
            },
        );

        Ok(())
    }

    /// Query a shared memory region by channel name
    ///
    /// # Arguments
    /// * `channel_name` - Channel identifier to look up
    ///
    /// # Returns
    /// * `Some(&ShmemEntry)` if found
    /// * `None` if not found
    pub fn query(&self, channel_name: &str) -> Option<&ShmemEntry> {
        self.entries.get(channel_name)
    }

    /// Unregister a shared memory region
    ///
    /// # Arguments
    /// * `channel_name` - Channel identifier to remove
    ///
    /// # Returns
    /// * `Ok(())` if removed
    /// * `Err(&str)` if not found
    pub fn unregister(&mut self, channel_name: &str) -> Result<(), &'static str> {
        if self.entries.remove(channel_name).is_some() {
            Ok(())
        } else {
            Err("Channel not found")
        }
    }

    /// Remove all registrations owned by a specific process
    ///
    /// Useful for cleanup when a process terminates
    pub fn cleanup_process(&mut self, pid: usize) {
        self.entries.retain(|_, entry| entry.owner_pid != pid);
    }

    /// Get the number of registered channels
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_query() {
        let mut registry = ShmemRegistry::new();

        // Register a channel
        assert!(registry
            .register(String::from("test_channel"), 0x40000000, 0x1000, 123)
            .is_ok());

        // Query it back
        let entry = registry.query("test_channel").unwrap();
        assert_eq!(entry.phys_addr, 0x40000000);
        assert_eq!(entry.size, 0x1000);
        assert_eq!(entry.owner_pid, 123);
    }

    #[test]
    fn test_duplicate_registration() {
        let mut registry = ShmemRegistry::new();

        assert!(registry
            .register(String::from("test_channel"), 0x40000000, 0x1000, 123)
            .is_ok());

        // Duplicate registration should fail
        assert!(registry
            .register(String::from("test_channel"), 0x40001000, 0x1000, 124)
            .is_err());
    }

    #[test]
    fn test_cleanup_process() {
        let mut registry = ShmemRegistry::new();

        registry
            .register(String::from("channel1"), 0x40000000, 0x1000, 123)
            .unwrap();
        registry
            .register(String::from("channel2"), 0x40001000, 0x1000, 124)
            .unwrap();
        registry
            .register(String::from("channel3"), 0x40002000, 0x1000, 123)
            .unwrap();

        assert_eq!(registry.len(), 3);

        // Cleanup process 123
        registry.cleanup_process(123);

        assert_eq!(registry.len(), 1);
        assert!(registry.query("channel2").is_some());
        assert!(registry.query("channel1").is_none());
        assert!(registry.query("channel3").is_none());
    }
}
