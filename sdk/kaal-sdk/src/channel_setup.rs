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
/// # Arguments
/// * `target_pid` - Process ID of the target component
/// * `buffer_size` - Size of the shared memory buffer (must be page-aligned)
/// * `role` - This component's role in the channel
///
/// # Returns
/// * `Ok(ChannelConfig)` - Channel configuration on success
/// * `Err(&str)` - Error message on failure
pub fn establish_channel(
    target_pid: usize,
    buffer_size: usize,
    role: ChannelRole,
) -> Result<ChannelConfig, &'static str> {
    // Validate buffer size is page-aligned
    if buffer_size == 0 || (buffer_size & 0xFFF) != 0 {
        return Err("Buffer size must be non-zero and page-aligned");
    }

    // Convert role to syscall parameter
    let role_param = match role {
        ChannelRole::Producer => 0,
        ChannelRole::Consumer => 1,
    };

    // Call kernel to establish channel
    let result = crate::syscall!(
        syscall::numbers::SYS_CHANNEL_ESTABLISH,
        target_pid,
        buffer_size,
        role_param
    );

    // Check for error
    if result == 0 {
        return Err("Failed to establish channel");
    }

    // Unpack the result
    // Format: buffer_addr in high 32 bits, notification_cap in low 16 bits, channel_id in next 16 bits
    let buffer_addr = (result >> 32) as usize;
    let notification_cap = (result & 0xFFFF) as usize;
    let channel_id = ((result >> 16) & 0xFFFF) as usize;

    Ok(ChannelConfig {
        buffer_addr,
        buffer_size,
        notification_cap,
        memory_cap: None, // TODO: Return memory cap from kernel
        channel_id,
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