//! Channel Broker - IPC Channel Management Service
//!
//! This module provides channel establishment and management for IPC.
//! It runs as part of the runtime/root-task with privileged access to:
//! - Memory allocation and mapping
//! - Capability creation and transfer
//! - Component address space management

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

// Imports will be used when fully implementing broker

/// Channel identifier
pub type ChannelId = usize;

/// Component identifier (PID)
pub type ComponentId = usize;

/// Channel establishment errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerError {
    /// No free channels available
    NoFreeChannels,
    /// Channel not found
    ChannelNotFound,
    /// Channel already exists between components
    ChannelExists,
    /// Memory allocation failed
    AllocationFailed,
    /// Memory mapping failed
    MappingFailed,
    /// Capability creation failed
    CapabilityFailed,
    /// Component not found
    ComponentNotFound,
    /// Not authorized for operation
    NotAuthorized,
}

/// Channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    /// Being established
    Establishing,
    /// Active and ready
    Active,
    /// Being closed
    Closing,
    /// Closed
    Closed,
}

/// Channel metadata
#[derive(Debug, Clone)]
pub struct Channel {
    pub id: ChannelId,
    pub producer_id: ComponentId,
    pub consumer_id: ComponentId,
    pub state: ChannelState,
    pub shared_memory_phys: usize,
    pub shared_memory_size: usize,
    pub producer_vaddr: usize,  // Virtual address in producer's space
    pub consumer_vaddr: usize,  // Virtual address in consumer's space
    pub producer_notify: usize,  // Notification capability slot
    pub consumer_notify: usize,  // Notification capability slot
}

/// Channel Broker - manages IPC channels
pub struct ChannelBroker {
    /// All active channels
    channels: BTreeMap<ChannelId, Channel>,
    /// Map component pairs to channel IDs for quick lookup
    component_channels: BTreeMap<(ComponentId, ComponentId), ChannelId>,
    /// Next channel ID
    next_channel_id: AtomicUsize,
    /// Maximum channels
    max_channels: usize,
    /// Shared memory registry for dynamic discovery
    shmem_registry: capability_broker::ShmemRegistry,
}

impl ChannelBroker {
    /// Create a new channel broker
    pub fn new(max_channels: usize) -> Self {
        Self {
            channels: BTreeMap::new(),
            component_channels: BTreeMap::new(),
            next_channel_id: AtomicUsize::new(1),
            max_channels,
            shmem_registry: capability_broker::ShmemRegistry::new(),
        }
    }

    /// Register shared memory with the broker
    ///
    /// Called by producers to publish physical memory for consumers to discover
    pub fn register_shmem(
        &mut self,
        channel_name: alloc::string::String,
        phys_addr: usize,
        size: usize,
        owner_pid: usize,
    ) -> Result<(), BrokerError> {
        self.shmem_registry
            .register(channel_name, phys_addr, size, owner_pid)
            .map_err(|_| BrokerError::ChannelExists)
    }

    /// Query shared memory from the broker
    ///
    /// Called by consumers to discover physical memory published by producers
    pub fn query_shmem(&self, channel_name: &str) -> Option<&capability_broker::ShmemEntry> {
        self.shmem_registry.query(channel_name)
    }

    /// Cleanup shared memory registrations for a terminated process
    pub fn cleanup_shmem(&mut self, pid: usize) {
        self.shmem_registry.cleanup_process(pid);
    }

    /// Establish a channel between two components (centralized orchestration)
    ///
    /// # What This Does
    ///
    /// The **ChannelBroker** (this struct, running in root-task) sets up a complete
    /// IPC channel by performing all privileged operations on behalf of both components:
    /// 1. Allocates shared memory
    /// 2. Maps it into both components' address spaces
    /// 3. Creates notification capabilities
    /// 4. Transfers capabilities to both components
    ///
    /// # Current Status: PLACEHOLDER
    ///
    /// This method currently only does **tracking/bookkeeping**. The actual working
    /// IPC uses **decentralized self-service** (see below).
    ///
    /// # Two Patterns for IPC Channel Establishment
    ///
    /// ## Pattern 1: Decentralized Self-Service (Current - WORKING)
    ///
    /// Components establish channels themselves via `sdk::channel_setup::establish_channel()`:
    ///
    /// ```text
    /// Producer:
    ///   1. Allocates physical memory (SYS_MEMORY_ALLOCATE)
    ///   2. Maps it into own address space (SYS_MEMORY_MAP)
    ///   3. Registers with ChannelBroker (SYS_SHMEM_REGISTER)
    ///   4. Creates own notification (SYS_NOTIFICATION_CREATE)
    ///
    /// Consumer:
    ///   1. Queries ChannelBroker for phys addr (SYS_SHMEM_QUERY)
    ///   2. Maps same physical memory (SYS_MEMORY_MAP)
    ///   3. Creates own notification (SYS_NOTIFICATION_CREATE)
    ///
    /// ChannelBroker Role: Shared memory registry only (discovery service)
    /// ```
    ///
    /// ## Pattern 2: Centralized Orchestration (Future - THIS METHOD)
    ///
    /// ChannelBroker (in root-task) manages entire setup:
    ///
    /// ```ignore
    /// // ChannelBroker has privileged access to all components
    ///
    /// // 1. Allocate shared memory
    /// let phys_addr = sys_memory_allocate(buffer_size)?;
    ///
    /// // 2. Map into producer's address space
    /// let producer_vaddr = 0x90000000; // Chosen by broker
    /// sys_memory_map_into(producer_tcb, phys_addr, buffer_size,
    ///                    producer_vaddr, PERMS_RW)?;
    ///
    /// // 3. Map into consumer's address space
    /// let consumer_vaddr = 0x90000000; // Same vaddr for simplicity
    /// sys_memory_map_into(consumer_tcb, phys_addr, buffer_size,
    ///                    consumer_vaddr, PERMS_RW)?;
    ///
    /// // 4. Create notification for signaling
    /// let notify_cap = sys_notification_create()?;
    ///
    /// // 5. Give notification to both components
    /// sys_cap_insert_into(producer_tcb, SLOT_NOTIFY, CAP_NOTIFICATION, notify_cap)?;
    /// sys_cap_insert_into(consumer_tcb, SLOT_NOTIFY, CAP_NOTIFICATION, notify_cap)?;
    ///
    /// // 6. Return channel info to requestor
    /// Ok(ChannelInfo { producer_vaddr, consumer_vaddr, notify_cap })
    /// ```
    ///
    /// # When to Use Each Pattern
    ///
    /// **Decentralized Self-Service**:
    /// - Components know their own memory layout best
    /// - Flexible, minimal broker involvement
    /// - Simpler implementation (current Phase 6)
    ///
    /// **Centralized Orchestration**:
    /// - Broker enforces security policies
    /// - Centralized audit trail
    /// - Components can't bypass broker
    /// - Better for untrusted components
    pub fn establish_channel(
        &mut self,
        producer_id: ComponentId,
        consumer_id: ComponentId,
        buffer_size: usize,
    ) -> Result<ChannelId, BrokerError> {
        // Check if channel already exists
        let key = self.component_key(producer_id, consumer_id);
        if self.component_channels.contains_key(&key) {
            return Err(BrokerError::ChannelExists);
        }

        // Check capacity
        if self.channels.len() >= self.max_channels {
            return Err(BrokerError::NoFreeChannels);
        }

        // Allocate channel ID
        let channel_id = self.next_channel_id.fetch_add(1, Ordering::SeqCst);

        // TODO: Implement broker-orchestrated IPC (see documentation above)
        // For now, this is a tracking/bookkeeping placeholder.
        // Components use sdk::channel_setup::establish_channel() for actual IPC.

        let channel = Channel {
            id: channel_id,
            producer_id,
            consumer_id,
            state: ChannelState::Establishing,
            shared_memory_phys: 0,  // Would be allocated via sys_memory_allocate
            shared_memory_size: buffer_size,
            producer_vaddr: 0,      // Would be mapped via sys_memory_map_into
            consumer_vaddr: 0,      // Would be mapped via sys_memory_map_into
            producer_notify: 0,     // Would be created via sys_notification_create
            consumer_notify: 0,     // Would be created via sys_notification_create
        };

        // Register channel for tracking
        self.channels.insert(channel_id, channel);
        self.component_channels.insert(key, channel_id);

        Ok(channel_id)
    }

    /// Get channel information
    pub fn get_channel(&self, channel_id: ChannelId) -> Option<&Channel> {
        self.channels.get(&channel_id)
    }

    /// Find channel between components
    pub fn find_channel(
        &self,
        comp1: ComponentId,
        comp2: ComponentId,
    ) -> Option<&Channel> {
        let key = self.component_key(comp1, comp2);
        self.component_channels
            .get(&key)
            .and_then(|id| self.channels.get(id))
    }

    /// Close a channel
    pub fn close_channel(
        &mut self,
        channel_id: ChannelId,
        requester: ComponentId,
    ) -> Result<(), BrokerError> {
        // Get channel
        let channel = self.channels
            .get(&channel_id)
            .ok_or(BrokerError::ChannelNotFound)?;

        // Verify requester is part of channel
        if channel.producer_id != requester && channel.consumer_id != requester {
            return Err(BrokerError::NotAuthorized);
        }

        // Here we would:
        // 1. Unmap memory from both components
        // 2. Revoke notification capabilities
        // 3. Free shared memory

        // Remove from registries
        let key = self.component_key(channel.producer_id, channel.consumer_id);
        self.component_channels.remove(&key);
        self.channels.remove(&channel_id);

        Ok(())
    }

    /// List channels for a component
    pub fn list_channels(&self, component_id: ComponentId) -> Vec<ChannelId> {
        self.channels
            .values()
            .filter(|c| c.producer_id == component_id || c.consumer_id == component_id)
            .map(|c| c.id)
            .collect()
    }

    /// Update channel state
    pub fn set_channel_state(
        &mut self,
        channel_id: ChannelId,
        state: ChannelState,
    ) -> Result<(), BrokerError> {
        self.channels
            .get_mut(&channel_id)
            .map(|c| c.state = state)
            .ok_or(BrokerError::ChannelNotFound)
    }

    /// Helper to create component pair key
    fn component_key(&self, comp1: ComponentId, comp2: ComponentId) -> (ComponentId, ComponentId) {
        if comp1 < comp2 {
            (comp1, comp2)
        } else {
            (comp2, comp1)
        }
    }
}

/// Global channel broker instance (would be in root-task)
static mut CHANNEL_BROKER: Option<ChannelBroker> = None;

/// Initialize the global channel broker
pub fn init_broker(max_channels: usize) {
    unsafe {
        CHANNEL_BROKER = Some(ChannelBroker::new(max_channels));
    }
}

/// Get reference to global broker
pub fn get_broker() -> Option<&'static ChannelBroker> {
    unsafe { CHANNEL_BROKER.as_ref() }
}

/// Get mutable reference to global broker
pub fn get_broker_mut() -> Option<&'static mut ChannelBroker> {
    unsafe { CHANNEL_BROKER.as_mut() }
}