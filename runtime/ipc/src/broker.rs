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
}

impl ChannelBroker {
    /// Create a new channel broker
    pub fn new(max_channels: usize) -> Self {
        Self {
            channels: BTreeMap::new(),
            component_channels: BTreeMap::new(),
            next_channel_id: AtomicUsize::new(1),
            max_channels,
        }
    }

    /// Establish a channel between two components
    ///
    /// This is called by root-task or kernel when components request IPC.
    /// It handles all the privileged operations:
    /// 1. Allocates shared memory
    /// 2. Maps it into both components
    /// 3. Creates notification capabilities
    /// 4. Transfers capabilities to components
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

        // Here we would call privileged operations:
        // 1. Allocate shared memory (sys_memory_allocate)
        // 2. Map into producer (sys_memory_map_into)
        // 3. Map into consumer (sys_memory_map_into)
        // 4. Create notifications (sys_notification_create)
        // 5. Transfer caps (sys_cap_insert_into)

        // For now, create placeholder channel
        let channel = Channel {
            id: channel_id,
            producer_id,
            consumer_id,
            state: ChannelState::Establishing,
            shared_memory_phys: 0,  // Would be allocated
            shared_memory_size: buffer_size,
            producer_vaddr: 0,      // Would be mapped
            consumer_vaddr: 0,      // Would be mapped
            producer_notify: 0,     // Would be created
            consumer_notify: 0,     // Would be created
        };

        // Register channel
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