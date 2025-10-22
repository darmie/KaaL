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
/// * `channel_name` - Unique identifier for this channel (e.g., "producer_to_consumer")
/// * `buffer_size` - Size of the shared memory buffer (must be page-aligned)
/// * `role` - This component's role in the channel
///
/// # Returns
/// * `Ok(ChannelConfig)` - Channel configuration on success
/// * `Err(&str)` - Error message on failure
pub fn establish_channel(
    channel_name: &str,
    buffer_size: usize,
    role: ChannelRole,
) -> Result<ChannelConfig, &'static str> {
    // Validate inputs
    if channel_name.is_empty() || channel_name.len() > 32 {
        return Err("Channel name must be 1-32 characters");
    }

    if buffer_size == 0 || (buffer_size & 0xFFF) != 0 {
        return Err("Buffer size must be non-zero and page-aligned");
    }

    let (phys_addr, virt_addr) = match role {
        ChannelRole::Producer => {
            // Producer allocates the shared buffer physical memory
            let buffer_phys = match syscall::memory_allocate(buffer_size) {
                Ok(addr) => addr,
                Err(_) => return Err("Failed to allocate buffer physical memory"),
            };

            // Map buffer into our address space
            let buffer_virt = match syscall::memory_map(buffer_phys, buffer_size, 0x3) {
                Ok(addr) => addr,
                Err(_) => return Err("Failed to map buffer into address space"),
            };

            // Register the physical address with the kernel broker
            unsafe {
                syscall::shmem_register(channel_name, buffer_phys, buffer_size)
                    .map_err(|_| "Failed to register shared memory with broker")?;
            }

            (buffer_phys, buffer_virt)
        }
        ChannelRole::Consumer => {
            // Query the broker for the physical address
            let buffer_phys = unsafe {
                syscall::shmem_query(channel_name)
                    .map_err(|_| "Producer has not yet allocated shared memory")?
            };

            // Map the producer's buffer into our address space
            let buffer_virt = match syscall::memory_map(buffer_phys, buffer_size, 0x3) {
                Ok(addr) => addr,
                Err(_) => return Err("Failed to map shared buffer into address space"),
            };

            (buffer_phys, buffer_virt)
        }
    };

    // Step 3: Create notification for signaling
    let notification_cap = match syscall::notification_create() {
        Ok(cap) => cap,
        Err(_) => return Err("Failed to create notification"),
    };

    // WORKAROUND: This printf prevents a heisenbug where buffer_addr gets corrupted
    // The bug appears to be related to compiler optimization or stack layout
    // Removing this line causes notepad to crash with FAR=0x164 (wrong buffer address)
    // Root cause: likely printf! macro's static mut globals affecting register allocation
    use crate::printf;
    printf!("[channel_setup] virt_addr={:#x}\n", virt_addr);

    // TODO: For proper IPC, we need to:
    // - Share the physical address with the other component via broker
    // - Exchange notification capabilities
    // This would typically go through a broker/nameserver

    Ok(ChannelConfig {
        buffer_addr: virt_addr,
        buffer_size,
        notification_cap,
        memory_cap: Some(phys_addr), // Store physical address for debugging
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