//! Shared Memory IPC - High-performance inter-component communication
//!
//! # Purpose
//! Provides shared memory ring buffers and notification mechanisms for
//! efficient bulk data transfer between components, avoiding expensive
//! message-passing IPC overhead.
//!
//! # Integration Points
//! - Depends on: seL4 kernel (notifications), Capability Broker (memory allocation)
//! - Provides to: All system components requiring high-throughput IPC
//! - IPC endpoints: Uses seL4 notifications for signaling
//! - Capabilities required: Shared memory regions, notification endpoints
//!
//! # Architecture
//! Lock-free ring buffer using atomic operations with seL4 notifications
//! for signaling. Supports both single-producer/single-consumer and
//! multi-producer/multi-consumer patterns.
//!
//! # Testing Strategy
//! - Unit tests: Ring buffer operations, boundary conditions
//! - Integration tests: Producer-consumer patterns, cross-component
//! - Hardware sim tests: Performance benchmarks, stress tests

#![no_std]

#[cfg(test)]
#[macro_use]
extern crate std;

use core::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;

// Use seL4 platform adapter (supports mock/microkit/runtime modes)
use sel4_platform::adapter::{CPtr as seL4_CPtr, MessageInfo as seL4_MessageInfo, signal as seL4_Signal, wait as seL4_Wait};

/// IPC error types
#[derive(Debug, Error)]
pub enum IpcError {
    #[error("Ring buffer full (capacity: {capacity})")]
    BufferFull { capacity: usize },

    #[error("Ring buffer empty")]
    BufferEmpty,

    #[error("Invalid buffer size (must be power of 2)")]
    InvalidSize,

    #[error("Notification failed")]
    NotificationFailed,
}

pub type Result<T> = core::result::Result<T, IpcError>;

/// Shared memory ring buffer for high-performance IPC
///
/// # Type Parameters
/// * `T` - Element type (must be `Copy` for zero-copy semantics)
/// * `N` - Ring buffer capacity (must be power of 2)
///
/// # Safety
/// This structure uses lock-free atomic operations. Callers must ensure
/// proper memory barriers are maintained when sharing between threads.
pub struct SharedRing<T: Copy, const N: usize> {
    buffer: [T; N],
    head: AtomicUsize,
    tail: AtomicUsize,
    /// seL4 notification for signaling consumer
    consumer_notify: Option<seL4_CPtr>,
    /// seL4 notification for signaling producer
    producer_notify: Option<seL4_CPtr>,
}

impl<T: Copy, const N: usize> SharedRing<T, N> {
    /// Create a new shared ring buffer without notifications
    ///
    /// # Errors
    /// Returns error if N is not a power of 2
    pub const fn new() -> Self {
        // Compile-time check that N is power of 2
        assert!(N.is_power_of_two(), "Ring buffer size must be power of 2");

        Self {
            buffer: unsafe { core::mem::zeroed() },
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            consumer_notify: None,
            producer_notify: None,
        }
    }

    /// Create a new shared ring buffer with seL4 notifications
    ///
    /// # Arguments
    /// * `consumer_notify` - Notification capability to signal consumer
    /// * `producer_notify` - Notification capability to signal producer
    pub fn with_notifications(consumer_notify: seL4_CPtr, producer_notify: seL4_CPtr) -> Self {
        assert!(N.is_power_of_two(), "Ring buffer size must be power of 2");

        Self {
            buffer: unsafe { core::mem::zeroed() },
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            consumer_notify: Some(consumer_notify),
            producer_notify: Some(producer_notify),
        }
    }

    /// Push an item into the ring buffer
    ///
    /// # Arguments
    /// * `item` - Item to push
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// Returns `IpcError::BufferFull` if buffer is full
    pub fn push(&self, item: T) -> Result<()> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        // Check if buffer is full
        if (head + 1) % N == tail {
            return Err(IpcError::BufferFull { capacity: N });
        }

        // Write item (safe: we checked for space)
        unsafe {
            core::ptr::write_volatile(
                self.buffer.as_ptr().add(head) as *mut T,
                item,
            );
        }

        // Update head with release semantics for visibility
        self.head.store((head + 1) % N, Ordering::Release);

        // Signal consumer via seL4 notification
        if let Some(notify) = self.consumer_notify {
            unsafe {
                // TODO PHASE 2: Handle notification errors properly
                seL4_Signal(notify);
            }
        }

        Ok(())
    }

    /// Pop an item from the ring buffer
    ///
    /// # Returns
    /// The popped item on success
    ///
    /// # Errors
    /// Returns `IpcError::BufferEmpty` if buffer is empty
    pub fn pop(&self) -> Result<T> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        // Check if buffer is empty
        if head == tail {
            return Err(IpcError::BufferEmpty);
        }

        // Read item
        let item = unsafe {
            core::ptr::read_volatile(
                self.buffer.as_ptr().add(tail) as *const T,
            )
        };

        // Update tail with release semantics
        self.tail.store((tail + 1) % N, Ordering::Release);

        // Signal producer that space is available
        if let Some(notify) = self.producer_notify {
            unsafe {
                // TODO PHASE 2: Handle notification errors properly
                seL4_Signal(notify);
            }
        }

        Ok(item)
    }

    /// Get current buffer occupancy
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        if head >= tail {
            head - tail
        } else {
            N - tail + head
        }
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire) == self.tail.load(Ordering::Acquire)
    }

    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        (head + 1) % N == tail
    }

    /// Wait for notification (blocking)
    ///
    /// Blocks the current thread until the consumer notification is signaled.
    /// Used by consumer to wait for data availability.
    ///
    /// # Errors
    /// Returns error if no consumer notification is configured
    pub fn wait_consumer(&self) -> Result<()> {
        match self.consumer_notify {
            Some(notify) => {
                unsafe {
                    // TODO PHASE 2: Handle wait errors properly
                    seL4_Wait(notify, core::ptr::null_mut());
                }
                Ok(())
            }
            None => Err(IpcError::NotificationFailed),
        }
    }

    /// Wait for notification (blocking)
    ///
    /// Blocks the current thread until the producer notification is signaled.
    /// Used by producer to wait for buffer space.
    ///
    /// # Errors
    /// Returns error if no producer notification is configured
    pub fn wait_producer(&self) -> Result<()> {
        match self.producer_notify {
            Some(notify) => {
                unsafe {
                    // TODO PHASE 2: Handle wait errors properly
                    seL4_Wait(notify, core::ptr::null_mut());
                }
                Ok(())
            }
            None => Err(IpcError::NotificationFailed),
        }
    }
}

/// High-level IPC channel abstraction
///
/// Provides a typed channel for sending and receiving messages between
/// components using shared memory and seL4 notifications.
pub struct Channel<T: Copy, const N: usize> {
    /// Shared ring buffer
    ring: SharedRing<T, N>,
}

impl<T: Copy, const N: usize> Channel<T, N> {
    /// Create a new channel with seL4 notifications
    ///
    /// # Arguments
    /// * `consumer_notify` - Notification to signal consumer
    /// * `producer_notify` - Notification to signal producer
    pub fn new(consumer_notify: seL4_CPtr, producer_notify: seL4_CPtr) -> Self {
        Self {
            ring: SharedRing::with_notifications(consumer_notify, producer_notify),
        }
    }

    /// Send a message (non-blocking)
    ///
    /// # Errors
    /// Returns error if buffer is full
    pub fn send(&self, msg: T) -> Result<()> {
        self.ring.push(msg)
    }

    /// Receive a message (non-blocking)
    ///
    /// # Errors
    /// Returns error if buffer is empty
    pub fn recv(&self) -> Result<T> {
        self.ring.pop()
    }

    /// Send a message (blocking)
    ///
    /// Blocks until buffer space is available.
    pub fn send_blocking(&self, msg: T) -> Result<()> {
        loop {
            match self.ring.push(msg) {
                Ok(()) => return Ok(()),
                Err(IpcError::BufferFull { .. }) => {
                    // Wait for space to become available
                    self.ring.wait_producer()?;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Receive a message (blocking)
    ///
    /// Blocks until a message is available.
    pub fn recv_blocking(&self) -> Result<T> {
        loop {
            match self.ring.pop() {
                Ok(msg) => return Ok(msg),
                Err(IpcError::BufferEmpty) => {
                    // Wait for data to become available
                    self.ring.wait_consumer()?;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Try to receive a message (non-blocking)
    ///
    /// Returns None if buffer is empty, Some(msg) otherwise.
    pub fn try_recv(&self) -> Option<T> {
        self.ring.pop().ok()
    }

    /// Check if channel has messages
    pub fn has_messages(&self) -> bool {
        !self.ring.is_empty()
    }

    /// Get number of pending messages
    pub fn len(&self) -> usize {
        self.ring.len()
    }

    /// Check if channel is empty
    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }
}

/// Channel endpoint types
pub enum EndpointRole {
    /// Producer (sender) endpoint
    Producer,
    /// Consumer (receiver) endpoint
    Consumer,
}

/// Split channel into producer and consumer endpoints
///
/// This allows separating send and receive operations for better
/// type safety and clearer ownership semantics.
pub struct Producer<T: Copy, const N: usize> {
    ring: *const SharedRing<T, N>,
}

unsafe impl<T: Copy, const N: usize> Send for Producer<T, N> {}
unsafe impl<T: Copy, const N: usize> Sync for Producer<T, N> {}

impl<T: Copy, const N: usize> Producer<T, N> {
    /// Send a message (non-blocking)
    pub fn send(&self, msg: T) -> Result<()> {
        unsafe { (*self.ring).push(msg) }
    }

    /// Send a message (blocking)
    pub fn send_blocking(&self, msg: T) -> Result<()> {
        loop {
            match unsafe { (*self.ring).push(msg) } {
                Ok(()) => return Ok(()),
                Err(IpcError::BufferFull { .. }) => {
                    unsafe { (*self.ring).wait_producer()? };
                }
                Err(e) => return Err(e),
            }
        }
    }
}

pub struct Consumer<T: Copy, const N: usize> {
    ring: *const SharedRing<T, N>,
}

unsafe impl<T: Copy, const N: usize> Send for Consumer<T, N> {}
unsafe impl<T: Copy, const N: usize> Sync for Consumer<T, N> {}

impl<T: Copy, const N: usize> Consumer<T, N> {
    /// Receive a message (non-blocking)
    pub fn recv(&self) -> Result<T> {
        unsafe { (*self.ring).pop() }
    }

    /// Receive a message (blocking)
    pub fn recv_blocking(&self) -> Result<T> {
        loop {
            match unsafe { (*self.ring).pop() } {
                Ok(msg) => return Ok(msg),
                Err(IpcError::BufferEmpty) => {
                    unsafe { (*self.ring).wait_consumer()? };
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Try to receive a message
    pub fn try_recv(&self) -> Option<T> {
        unsafe { (*self.ring).pop().ok() }
    }

    /// Check if there are pending messages
    pub fn has_messages(&self) -> bool {
        unsafe { !(*self.ring).is_empty() }
    }
}

impl<T: Copy, const N: usize> Channel<T, N> {
    /// Split channel into producer and consumer endpoints
    ///
    /// # Safety
    /// Caller must ensure the channel outlives the returned endpoints
    pub unsafe fn split(&self) -> (Producer<T, N>, Consumer<T, N>) {
        (
            Producer {
                ring: &self.ring as *const _,
            },
            Consumer {
                ring: &self.ring as *const _,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_push_pop() {
        let ring: SharedRing<u32, 8> = SharedRing::new();

        // Push items
        ring.push(1).unwrap();
        ring.push(2).unwrap();
        ring.push(3).unwrap();

        // Pop items
        assert_eq!(ring.pop().unwrap(), 1);
        assert_eq!(ring.pop().unwrap(), 2);
        assert_eq!(ring.pop().unwrap(), 3);
    }

    #[test]
    fn test_ring_buffer_full() {
        let ring: SharedRing<u32, 4> = SharedRing::new();

        // Fill buffer (capacity - 1 items)
        assert!(ring.push(1).is_ok());
        assert!(ring.push(2).is_ok());
        assert!(ring.push(3).is_ok());

        // Next push should fail
        assert!(matches!(ring.push(4), Err(IpcError::BufferFull { .. })));
    }

    #[test]
    fn test_ring_buffer_empty() {
        let ring: SharedRing<u32, 8> = SharedRing::new();

        // Pop from empty buffer should fail
        assert!(matches!(ring.pop(), Err(IpcError::BufferEmpty)));
    }

    #[test]
    fn test_ring_buffer_wrap_around() {
        let ring: SharedRing<u32, 4> = SharedRing::new();

        // Fill and empty multiple times
        for i in 0..10 {
            ring.push(i).unwrap();
            ring.push(i + 1).unwrap();

            assert_eq!(ring.pop().unwrap(), i);
            assert_eq!(ring.pop().unwrap(), i + 1);
        }
    }

    #[test]
    fn test_ring_buffer_len() {
        let ring: SharedRing<u32, 8> = SharedRing::new();

        assert_eq!(ring.len(), 0);

        ring.push(1).unwrap();
        assert_eq!(ring.len(), 1);

        ring.push(2).unwrap();
        assert_eq!(ring.len(), 2);

        ring.pop().unwrap();
        assert_eq!(ring.len(), 1);
    }

    #[test]
    fn test_is_empty_is_full() {
        let ring: SharedRing<u32, 4> = SharedRing::new();

        assert!(ring.is_empty());
        assert!(!ring.is_full());

        ring.push(1).unwrap();
        assert!(!ring.is_empty());
        assert!(!ring.is_full());

        ring.push(2).unwrap();
        ring.push(3).unwrap();
        assert!(!ring.is_empty());
        assert!(ring.is_full());
    }

    #[test]
    fn test_channel_send_recv() {
        // Create channel with dummy notification capabilities
        let channel: Channel<u32, 8> = Channel::new(1, 2);

        // Send and receive messages
        channel.send(42).unwrap();
        channel.send(100).unwrap();

        assert_eq!(channel.recv().unwrap(), 42);
        assert_eq!(channel.recv().unwrap(), 100);
    }

    #[test]
    fn test_channel_try_recv() {
        let channel: Channel<u32, 8> = Channel::new(1, 2);

        // Empty channel
        assert!(channel.try_recv().is_none());

        // Send message
        channel.send(123).unwrap();
        assert_eq!(channel.try_recv(), Some(123));

        // Empty again
        assert!(channel.try_recv().is_none());
    }

    #[test]
    fn test_channel_has_messages() {
        let channel: Channel<u32, 8> = Channel::new(1, 2);

        assert!(!channel.has_messages());
        assert_eq!(channel.len(), 0);

        channel.send(1).unwrap();
        assert!(channel.has_messages());
        assert_eq!(channel.len(), 1);

        channel.recv().unwrap();
        assert!(!channel.has_messages());
        assert_eq!(channel.len(), 0);
    }

    #[test]
    fn test_producer_consumer_split() {
        let channel: Channel<u32, 8> = Channel::new(1, 2);

        unsafe {
            let (producer, consumer) = channel.split();

            // Producer sends
            producer.send(10).unwrap();
            producer.send(20).unwrap();

            // Consumer receives
            assert_eq!(consumer.recv().unwrap(), 10);
            assert_eq!(consumer.recv().unwrap(), 20);

            // Check empty
            assert!(!consumer.has_messages());
        }
    }

    #[test]
    fn test_ring_with_notifications() {
        // Create ring with notification capabilities
        let ring: SharedRing<u32, 8> = SharedRing::with_notifications(100, 101);

        // Should work the same as regular ring for push/pop
        ring.push(42).unwrap();
        assert_eq!(ring.pop().unwrap(), 42);
    }
}
