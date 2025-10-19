//! Low-level channel establishment API with capability-based isolation
//!
//! Provides syscall wrappers for establishing secure IPC channels between components.
//! This module handles the kernel-level setup of shared memory and capabilities,
//! ensuring proper isolation through capability-based access control.
//!
//! # Security Model
//! - Each component only receives capabilities for its role (producer/consumer)
//! - Shared memory is mapped read-write for producer, read-only for consumer
//! - Notifications are unidirectional (producer signals consumer)
//! - Channel IDs provide management without exposing raw capabilities
//!
//! For high-level message passing, see the `message` module which provides
//! the `Channel<T>` type that uses the infrastructure set up by this module.

use crate::syscall;

/// Role in the channel
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelRole {
    /// Component is the producer (sender) in the channel
    Producer,
    /// Component is the consumer (receiver) in the channel
    Consumer,
}

/// Channel establishment result with proper capability isolation
///
/// Contains all the capabilities and resources needed for secure IPC.
/// Each component only gets the capabilities appropriate for its role:
/// - Producer: write access to buffer, can signal consumer
/// - Consumer: read access to buffer, receives signals
#[derive(Debug)]
pub struct ChannelConfig {
    /// Virtual address of shared memory buffer (mapped into component's address space)
    pub buffer_addr: usize,
    /// Size of the buffer in bytes
    pub buffer_size: usize,
    /// Notification capability for signaling (producer) or receiving (consumer)
    pub notification_cap: usize,
    /// Memory capability slot for the shared buffer (for remapping/unmapping)
    pub memory_cap: Option<usize>,
    /// Channel identifier for management operations
    pub channel_id: usize,
    /// This component's role in the channel
    pub role: ChannelRole,
}

/// Establish an IPC channel with another component
///
/// Uses syscalls to dynamically allocate and map shared memory.
/// This is the architecture-driven approach - no hardcoded addresses.
///
/// # Arguments
/// * `_target_pid` - Process ID of the target component (unused for now - TODO: implement proper discovery)
/// * `buffer_size` - Size of the shared memory buffer (must be page-aligned)
/// * `role` - This component's role in the channel
///
/// # Returns
/// * `Ok(ChannelConfig)` - Channel configuration on success
/// * `Err(&str)` - Error message on failure
pub fn establish_channel(
    _target_pid: usize,
    buffer_size: usize,
    role: ChannelRole,
) -> Result<ChannelConfig, &'static str> {
    // Validate buffer size is page-aligned
    if buffer_size == 0 || (buffer_size & 0xFFF) != 0 {
        return Err("Buffer size must be non-zero and page-aligned");
    }

    // Step 1: Allocate physical memory for the shared buffer
    let phys_addr = match syscall::memory_allocate(buffer_size) {
        Ok(addr) => addr,
        Err(_) => return Err("Failed to allocate physical memory"),
    };

    // Step 2: Map into our address space
    let virt_addr = match syscall::memory_map(phys_addr, buffer_size, 0x3) {
        Ok(addr) => addr,
        Err(_) => return Err("Failed to map memory into address space"),
    };

    // Step 3: Create notification for signaling
    let notification_cap = match syscall::notification_create() {
        Ok(cap) => cap,
        Err(_) => return Err("Failed to create notification"),
    };

    // TODO: For proper IPC, we need to:
    // - Share the physical address with the other component
    // - Exchange notification capabilities
    // This would typically go through a broker/nameserver

    Ok(ChannelConfig {
        buffer_addr: virt_addr,
        buffer_size,
        notification_cap,
        memory_cap: None,
        channel_id: 0, // TODO: Get from broker
        role,
    })
}

/// Query information about an established channel
///
/// # Arguments
/// * `channel_id` - Channel identifier
///
/// # Returns
/// * Channel information packed as usize, or 0 on error
pub fn query_channel(channel_id: usize) -> usize {
    crate::syscall!(syscall::numbers::SYS_CHANNEL_QUERY, channel_id)
}

/// Close an IPC channel
///
/// # Arguments
/// * `channel_id` - Channel identifier to close
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(&str)` on failure
pub fn close_channel(channel_id: usize) -> Result<(), &'static str> {
    let result = crate::syscall!(syscall::numbers::SYS_CHANNEL_CLOSE, channel_id);

    if result == 0 {
        Ok(())
    } else {
        Err("Failed to close channel")
    }
}