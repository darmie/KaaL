//! Shared Memory IPC - High-performance inter-component communication
//!
//! # Purpose
//! Provides shared memory ring buffers and notification mechanisms for
//! efficient bulk data transfer between components, avoiding expensive
//! message-passing IPC overhead.
//!
//! # Integration Points
//! - Depends on: KaaL kernel (notifications), Capability Broker (memory allocation)
//! - Provides to: All system components requiring high-throughput IPC
//! - IPC endpoints: Uses notification objects for signaling
//! - Capabilities required: Shared memory regions, notification capabilities
//!
//! # Architecture
//! Lock-free ring buffer using atomic operations with notification-based
//! signaling. Supports single-producer/single-consumer pattern with zero-copy
//! semantics.
//!
//! # Design
//! Based on Chapter 9 Phase 2 shared memory IPC architecture:
//! - Lock-free ring buffers for bulk data transfer
//! - Notification objects for lightweight signaling
//! - Zero-copy communication (data stays in shared memory)
//! - Target latency: < 500 CPU cycles

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "alloc")]
pub mod broker;

/// IPC error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    /// Ring buffer is full
    BufferFull { capacity: usize },
    /// Ring buffer is empty
    BufferEmpty,
    /// Invalid buffer size (must be power of 2)
    InvalidSize,
    /// Notification operation failed
    NotificationFailed,
    /// Invalid notification capability
    InvalidNotification,
}

pub type Result<T> = core::result::Result<T, IpcError>;

/// Notification capability slot (indexes into CSpace)
pub type NotificationCap = u64;

/// Shared memory ring buffer for high-performance IPC
///
/// # Type Parameters
/// * `T` - Element type (must be `Copy` for zero-copy semantics)
/// * `N` - Ring buffer capacity (must be power of 2)
///
/// # Safety
/// This structure uses lock-free atomic operations. The ring buffer must be
/// placed in shared memory accessible to both producer and consumer processes.
///
/// # Memory Layout
/// The ring buffer uses a single allocation containing:
/// - Array of T elements (N items)
/// - Atomic head pointer (producer writes here)
/// - Atomic tail pointer (consumer reads here)
/// - Notification capability slots for signaling
///
/// # Lock-Free Guarantees
/// - Single producer, single consumer (SPSC)
/// - Wait-free for producer (if space available)
/// - Wait-free for consumer (if data available)
/// - Uses atomic operations with proper memory ordering
#[repr(C)]
pub struct SharedRing<T: Copy, const N: usize> {
    /// Ring buffer storage
    buffer: [T; N],
    /// Head index (producer writes here)
    head: AtomicUsize,
    /// Tail index (consumer reads here)
    tail: AtomicUsize,
    /// Notification capability for signaling consumer
    consumer_notify: Option<NotificationCap>,
    /// Notification capability for signaling producer
    producer_notify: Option<NotificationCap>,
}

impl<T: Copy, const N: usize> SharedRing<T, N> {
    /// Create a new shared ring buffer without notifications
    ///
    /// # Panics
    /// Panics if N is not a power of 2 (compile-time check)
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

    /// Create a new shared ring buffer with notification capabilities
    ///
    /// # Arguments
    /// * `consumer_notify` - Notification capability to signal consumer
    /// * `producer_notify` - Notification capability to signal producer
    ///
    /// # Panics
    /// Panics if N is not a power of 2
    pub fn with_notifications(
        consumer_notify: NotificationCap,
        producer_notify: NotificationCap,
    ) -> Self {
        assert!(N.is_power_of_two(), "Ring buffer size must be power of 2");

        Self {
            buffer: unsafe { core::mem::zeroed() },
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            consumer_notify: Some(consumer_notify),
            producer_notify: Some(producer_notify),
        }
    }

    /// Push an item into the ring buffer (producer side)
    ///
    /// # Arguments
    /// * `item` - Item to push
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// Returns `IpcError::BufferFull` if buffer is full
    ///
    /// # Implementation Notes
    /// - Uses Acquire ordering to read head/tail
    /// - Uses Release ordering to update head (ensures item write is visible)
    /// - Signals consumer via notification if configured
    pub fn push(&self, item: T) -> Result<()> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        // Check if buffer is full (leaves one slot empty to distinguish full/empty)
        if (head + 1) % N == tail {
            return Err(IpcError::BufferFull { capacity: N });
        }

        // Write item to buffer
        unsafe {
            core::ptr::write_volatile(self.buffer.as_ptr().add(head) as *mut T, item);
        }

        // Update head with release semantics for visibility
        self.head.store((head + 1) % N, Ordering::Release);

        // Signal consumer via notification
        if let Some(notify_cap) = self.consumer_notify {
            // Badge = 1 indicates data available
            unsafe {
                sys_signal(notify_cap, 1);
            }
        }

        Ok(())
    }

    /// Pop an item from the ring buffer (consumer side)
    ///
    /// # Returns
    /// The popped item on success
    ///
    /// # Errors
    /// Returns `IpcError::BufferEmpty` if buffer is empty
    ///
    /// # Implementation Notes
    /// - Uses Acquire ordering to read head/tail
    /// - Uses Release ordering to update tail (ensures read is complete)
    /// - Signals producer via notification if configured
    pub fn pop(&self) -> Result<T> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        // Check if buffer is empty
        if head == tail {
            return Err(IpcError::BufferEmpty);
        }

        // Read item from buffer
        let item = unsafe { core::ptr::read_volatile(self.buffer.as_ptr().add(tail) as *const T) };

        // Update tail with release semantics
        self.tail.store((tail + 1) % N, Ordering::Release);

        // Signal producer that space is available
        if let Some(notify_cap) = self.producer_notify {
            // Badge = 2 indicates space available
            unsafe {
                sys_signal(notify_cap, 2);
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

    /// Get the consumer notification capability
    ///
    /// Returns the notification capability that the producer signals
    /// when data is available. Used by consumers to extract the
    /// notification from a SharedRing initialized by the producer.
    pub fn get_consumer_notify(&self) -> Option<NotificationCap> {
        self.consumer_notify
    }

    /// Wait for consumer notification (blocking)
    ///
    /// Blocks the current thread until the consumer notification is signaled.
    /// Used by consumer to wait for data availability.
    ///
    /// # Returns
    /// Signal bits from the notification
    ///
    /// # Errors
    /// Returns error if no consumer notification is configured
    pub fn wait_consumer(&self) -> Result<u64> {
        match self.consumer_notify {
            Some(notify_cap) => {
                let signals = unsafe { sys_wait(notify_cap) };
                if signals == u64::MAX {
                    Err(IpcError::NotificationFailed)
                } else {
                    Ok(signals)
                }
            }
            None => Err(IpcError::InvalidNotification),
        }
    }

    /// Wait for producer notification (blocking)
    ///
    /// Blocks the current thread until the producer notification is signaled.
    /// Used by producer to wait for buffer space.
    ///
    /// # Returns
    /// Signal bits from the notification
    ///
    /// # Errors
    /// Returns error if no producer notification is configured
    pub fn wait_producer(&self) -> Result<u64> {
        match self.producer_notify {
            Some(notify_cap) => {
                let signals = unsafe { sys_wait(notify_cap) };
                if signals == u64::MAX {
                    Err(IpcError::NotificationFailed)
                } else {
                    Ok(signals)
                }
            }
            None => Err(IpcError::InvalidNotification),
        }
    }

    /// Poll consumer notification (non-blocking)
    ///
    /// Checks for consumer notification without blocking.
    ///
    /// # Returns
    /// Signal bits (0 if no signals available)
    pub fn poll_consumer(&self) -> u64 {
        match self.consumer_notify {
            Some(notify_cap) => unsafe { sys_poll(notify_cap) },
            None => 0,
        }
    }

    /// Poll producer notification (non-blocking)
    ///
    /// Checks for producer notification without blocking.
    ///
    /// # Returns
    /// Signal bits (0 if no signals available)
    pub fn poll_producer(&self) -> u64 {
        match self.producer_notify {
            Some(notify_cap) => unsafe { sys_poll(notify_cap) },
            None => 0,
        }
    }
}

// Syscall wrappers for notification operations
// These call into kernel notification syscalls (0x17-0x1A)

/// Signal a notification (non-blocking)
unsafe fn sys_signal(notification_cap: u64, badge: u64) {
    let syscall_num: u64 = 0x18; // SYS_SIGNAL
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {cap}",
        "mov x1, {badge}",
        "svc #0",
        syscall_num = in(reg) syscall_num,
        cap = in(reg) notification_cap,
        badge = in(reg) badge,
        out("x8") _,
        out("x0") _,
        out("x1") _,
    );
}

/// Wait for notification (blocking)
unsafe fn sys_wait(notification_cap: u64) -> u64 {
    let syscall_num: u64 = 0x19; // SYS_WAIT
    let result: u64;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {cap}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) syscall_num,
        cap = in(reg) notification_cap,
        result = out(reg) result,
        out("x8") _,
    );
    result
}

/// Poll notification (non-blocking)
unsafe fn sys_poll(notification_cap: u64) -> u64 {
    let syscall_num: u64 = 0x1A; // SYS_POLL
    let result: u64;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {cap}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) syscall_num,
        cap = in(reg) notification_cap,
        result = out(reg) result,
        out("x8") _,
    );
    result
}

/// Producer handle for shared ring buffer
///
/// Provides a type-safe interface for the producer side of the ring buffer.
/// Only allows push operations and producer notifications.
pub struct Producer<'a, T: Copy, const N: usize> {
    ring: &'a SharedRing<T, N>,
}

impl<'a, T: Copy, const N: usize> Producer<'a, T, N> {
    /// Create a producer handle from a shared ring
    pub fn new(ring: &'a SharedRing<T, N>) -> Self {
        Self { ring }
    }

    /// Push an item into the ring buffer
    pub fn push(&self, item: T) -> Result<()> {
        self.ring.push(item)
    }

    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        self.ring.is_full()
    }

    /// Wait for space to become available
    pub fn wait_for_space(&self) -> Result<u64> {
        self.ring.wait_producer()
    }

    /// Poll for space availability notification
    pub fn poll_space(&self) -> u64 {
        self.ring.poll_producer()
    }
}

/// Consumer handle for shared ring buffer
///
/// Provides a type-safe interface for the consumer side of the ring buffer.
/// Only allows pop operations and consumer notifications.
pub struct Consumer<'a, T: Copy, const N: usize> {
    ring: &'a SharedRing<T, N>,
}

impl<'a, T: Copy, const N: usize> Consumer<'a, T, N> {
    /// Create a consumer handle from a shared ring
    pub fn new(ring: &'a SharedRing<T, N>) -> Self {
        Self { ring }
    }

    /// Pop an item from the ring buffer
    pub fn pop(&self) -> Result<T> {
        self.ring.pop()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    /// Get current buffer length
    pub fn len(&self) -> usize {
        self.ring.len()
    }

    /// Wait for data to become available
    pub fn wait_for_data(&self) -> Result<u64> {
        self.ring.wait_consumer()
    }

    /// Poll for data availability notification
    pub fn poll_data(&self) -> u64 {
        self.ring.poll_consumer()
    }
}
