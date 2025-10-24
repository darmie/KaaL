//! Message-Passing IPC Abstraction
//!
//! **This module provides typed message-passing channels, NOT bare notifications.**
//!
//! ## Distinction from Notifications
//!
//! - **Notification** (`syscall::notification_create`, `capability::Notification`):
//!   - Pure synchronization primitive (like eventfd/semaphore)
//!   - Only carries signal bits (badges), no data payload
//!   - Use for: event signaling, interrupts, general synchronization
//!
//! - **Channel<T>** (this module):
//!   - Complete message-passing system for typed data transfer
//!   - Built on: SharedRing (shared memory) + Notifications (for blocking/wakeup)
//!   - Provides semantic `send(msg)` / `receive()` API
//!   - Use for: inter-process communication with typed messages
//!
//! ## Architecture
//!
//! ```text
//! Channel<T>::send(msg)
//!   └─> SharedRing::push(msg)     [writes to shared memory]
//!        └─> sys_signal()          [wakes receiver if blocked]
//!
//! Channel<T>::receive()
//!   └─> SharedRing::pop()          [reads from shared memory]
//!        └─> sys_wait()            [blocks until data available]
//! ```
//!
//! # Design Philosophy
//! - Use semantic terminology: send/receive, not push/pop
//! - Hide ring buffer implementation details
//! - Type-safe message passing
//! - Automatic notification handling
//!
//! # Usage
//! ```no_run
//! use kaal_sdk::message::{Channel, ChannelConfig};
//!
//! // Sender component
//! let channel = Channel::<u32>::sender(config);
//! channel.send(42)?;
//!
//! // Receiver component
//! let channel = Channel::<u32>::receiver(config);
//! let value = channel.receive()?;
//! ```

use crate::ipc::{SharedRing, IpcError};
use crate::syscall;

/// Channel configuration for establishing message-passing connection
#[derive(Debug, Clone, Copy)]
pub struct ChannelConfig {
    /// Virtual address of shared memory containing the channel
    pub shared_memory: usize,
    /// Notification capability for signaling receiver
    pub receiver_notify: u64,
    /// Notification capability for signaling sender
    pub sender_notify: u64,
}

/// Message-passing channel for inter-component communication
///
/// Provides a semantic API for sending and receiving messages between components.
/// Internally uses SharedRing for efficient lock-free communication.
///
/// # Type Parameters
/// * `T` - Message type (must be `Copy` + `'static` for zero-copy semantics)
///
/// # Capacity
/// Currently fixed at 256 messages. Future versions may make this configurable.
pub struct Channel<T: Copy + 'static> {
    ring: &'static SharedRing<T, 256>,
    role: ChannelRole,
    my_notification: u64, // My notification cap for waiting (receiver) or signaling back (sender)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChannelRole {
    Sender,
    Receiver,
}

impl<T: Copy + 'static> Channel<T> {
    /// Create a sender channel endpoint
    ///
    /// The sender can send messages and wait for buffer space.
    ///
    /// # Arguments
    /// * `config` - Channel configuration with shared memory and notification capabilities
    ///
    /// # Safety
    /// - `shared_memory` must point to valid shared memory containing SharedRing
    /// - Notification capabilities must be valid
    /// - Only one sender per channel (single-producer pattern)
    pub unsafe fn sender(config: ChannelConfig) -> Self {
        let ring = &*(config.shared_memory as *const SharedRing<T, 256>);
        Self {
            ring,
            role: ChannelRole::Sender,
            my_notification: config.receiver_notify, // Sender signals the RECEIVER's notification
        }
    }

    /// Create a receiver channel endpoint
    ///
    /// The receiver can receive messages and wait for data availability.
    ///
    /// # Arguments
    /// * `config` - Channel configuration with shared memory and notification capabilities
    ///
    /// # Safety
    /// - `shared_memory` must point to valid shared memory containing SharedRing
    /// - Notification capabilities must be valid
    /// - Only one receiver per channel (single-consumer pattern)
    pub unsafe fn receiver(config: ChannelConfig) -> Self {
        let ring = &*(config.shared_memory as *const SharedRing<T, 256>);
        Self {
            ring,
            role: ChannelRole::Receiver,
            my_notification: config.receiver_notify,
        }
    }

    /// Send a message through the channel
    ///
    /// Blocks if the channel is full, waiting for the receiver to consume messages.
    /// Automatically signals the receiver when message is sent.
    ///
    /// # Arguments
    /// * `message` - Message to send
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// Returns error if notification operations fail
    ///
    /// # Panics
    /// Panics if called on a receiver channel
    pub fn send(&self, message: T) -> Result<(), IpcError> {
        assert_eq!(self.role, ChannelRole::Sender, "send() called on receiver channel");

        loop {
            match self.ring.push(message) {
                Ok(()) => {
                    // Message sent successfully
                    // Receiver is automatically signaled by SharedRing
                    return Ok(());
                }
                Err(IpcError::BufferFull { .. }) => {
                    // Channel full - wait for receiver to make space
                    self.ring.wait_producer()?;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Try to send a message without blocking
    ///
    /// Returns immediately if the channel is full.
    ///
    /// # Arguments
    /// * `message` - Message to send
    ///
    /// # Returns
    /// Ok(()) if sent successfully, Err if channel is full
    pub fn try_send(&self, message: T) -> Result<(), IpcError> {
        assert_eq!(self.role, ChannelRole::Sender, "try_send() called on receiver channel");
        self.ring.push(message)
    }

    /// Receive a message from the channel
    ///
    /// Blocks if the channel is empty, waiting for the sender to produce messages.
    /// Automatically signals the sender when message is received.
    ///
    /// # Returns
    /// The received message on success
    ///
    /// # Errors
    /// Returns error if notification operations fail
    ///
    /// # Panics
    /// Panics if called on a sender channel
    pub fn receive(&self) -> Result<T, IpcError> {
        assert_eq!(self.role, ChannelRole::Receiver, "receive() called on sender channel");

        // Streaming receive model: keep trying to read from ring buffer
        loop {
            match self.ring.pop() {
                Ok(message) => {
                    // Got message from stream
                    return Ok(message);
                }
                Err(IpcError::BufferEmpty) => {
                    // Stream empty - block until producer signals more data available
                    use crate::syscall;
                    match syscall::wait(self.my_notification as usize) {
                        Ok(_signals) => {
                            // Producer signaled - loop back to try reading from stream again
                        }
                        Err(_) => {
                            return Err(IpcError::NotificationFailed);
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Try to receive a message without blocking
    ///
    /// Returns immediately if the channel is empty.
    ///
    /// # Returns
    /// Ok(message) if received successfully, Err if channel is empty
    pub fn try_receive(&self) -> Result<T, IpcError> {
        assert_eq!(self.role, ChannelRole::Receiver, "try_receive() called on sender channel");
        self.ring.pop()
    }

    /// Check if channel has messages available
    ///
    /// Non-blocking check for data availability.
    pub fn has_messages(&self) -> bool {
        !self.ring.is_empty()
    }

    /// Get the number of messages currently in the channel
    pub fn len(&self) -> usize {
        self.ring.len()
    }

    /// Check if channel is empty
    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    /// Check if channel is full
    pub fn is_full(&self) -> bool {
        self.ring.is_full()
    }
}

/// Iterator adapter for receiving messages
///
/// Allows using `for message in channel.iter()` syntax.
pub struct ChannelIter<'a, T: Copy + 'static> {
    channel: &'a Channel<T>,
}

impl<'a, T: Copy + 'static> Iterator for ChannelIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.channel.receive().ok()
    }
}

impl<T: Copy + 'static> Channel<T> {
    /// Create an iterator that receives messages until the channel closes
    ///
    /// # Example
    /// ```no_run
    /// for message in channel.iter() {
    ///     process(message);
    /// }
    /// ```
    pub fn iter(&self) -> ChannelIter<'_, T> {
        ChannelIter { channel: self }
    }
}

/// Helper function to initialize shared memory for a channel
///
/// Must be called before creating channel endpoints.
/// Typically called by the coordinating component (e.g., root-task).
///
/// # Arguments
/// * `shared_memory` - Virtual address of shared memory region
/// * `receiver_notify` - Notification capability for receiver
/// * `sender_notify` - Notification capability for sender
///
/// # Safety
/// - `shared_memory` must point to valid, writable shared memory
/// - Memory must be at least `size_of::<SharedRing<T, 256>>()` bytes
/// - Must be called before any component accesses the channel
pub unsafe fn initialize_channel<T: Copy>(
    shared_memory: usize,
    receiver_notify: u64,
    sender_notify: u64,
) {
    let ring_ptr = shared_memory as *mut SharedRing<T, 256>;
    let ring = SharedRing::<T, 256>::with_notifications(receiver_notify, sender_notify);
    core::ptr::write(ring_ptr, ring);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Real tests would require kernel support
    // These are structural tests to verify API design

    #[test]
    fn channel_config_is_copy() {
        let config = ChannelConfig {
            shared_memory: 0x1000,
            receiver_notify: 100,
            sender_notify: 101,
        };
        let _config2 = config; // Should compile (Copy)
    }
}
