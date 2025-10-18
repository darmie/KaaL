//! Notification - Lightweight signaling primitive
//!
//! Notifications are seL4-style signaling objects used for asynchronous
//! communication and event notification. They are much lighter than full
//! message-passing IPC endpoints.
//!
//! ## Design
//!
//! A notification contains a bitfield where each bit represents a separate signal.
//! Multiple threads can wait on a notification, and any thread can signal it.
//!
//! ## Operations
//!
//! - **Signal**: Set notification bits (non-blocking)
//! - **Wait**: Block until notification bits are set, then clear and return them
//! - **Poll**: Check notification bits without blocking
//!
//! ## Use Cases
//!
//! - Shared memory ring buffer signaling (producer/consumer)
//! - Interrupt delivery to userspace
//! - Event notification between components
//! - Semaphore-like synchronization

use crate::objects::TCB;
use core::sync::atomic::{AtomicU64, Ordering};

/// Maximum number of threads that can wait on a notification
const MAX_QUEUE_SIZE: usize = 16;

/// Simple FIFO queue for blocked threads
struct ThreadQueue {
    threads: [*mut TCB; MAX_QUEUE_SIZE],
    count: usize,
}

impl ThreadQueue {
    const fn new() -> Self {
        Self {
            threads: [core::ptr::null_mut(); MAX_QUEUE_SIZE],
            count: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn len(&self) -> usize {
        self.count
    }

    fn enqueue(&mut self, tcb: *mut TCB) {
        debug_assert!(self.count < MAX_QUEUE_SIZE, "Thread queue overflow");
        if self.count < MAX_QUEUE_SIZE {
            self.threads[self.count] = tcb;
            self.count += 1;
        }
    }

    fn dequeue(&mut self) -> Option<*mut TCB> {
        if self.count == 0 {
            None
        } else {
            let tcb = self.threads[0];
            for i in 0..self.count - 1 {
                self.threads[i] = self.threads[i + 1];
            }
            self.threads[self.count - 1] = core::ptr::null_mut();
            self.count -= 1;
            Some(tcb)
        }
    }
}

/// Notification object for lightweight signaling
///
/// Each notification has a 64-bit word where each bit represents a separate signal.
/// This allows for efficient batching of signals and multiplexing of events.
#[repr(C)]
pub struct Notification {
    /// Signal word - bits representing pending signals
    /// Uses atomic operations for lock-free signal/poll
    signal_word: AtomicU64,

    /// Queue of threads waiting on this notification
    /// When signaled, all waiting threads are woken
    wait_queue: ThreadQueue,
}

impl Notification {
    /// Create a new notification with no signals pending
    pub const fn new() -> Self {
        Self {
            signal_word: AtomicU64::new(0),
            wait_queue: ThreadQueue::new(),
        }
    }

    /// Signal the notification with a bitmask
    ///
    /// Sets the specified bits in the notification word. If any threads are
    /// waiting, they are woken up with the signal bits.
    ///
    /// # Arguments
    ///
    /// * `badge` - Bitmask of signals to set (OR'd with existing signals)
    ///
    /// # Safety
    ///
    /// Must be called with interrupts disabled to prevent races with scheduler
    pub unsafe fn signal(&mut self, badge: u64) {
        // OR in the new signal bits atomically
        let old_word = self.signal_word.fetch_or(badge, Ordering::Release);

        // If there were waiting threads, wake them all
        if !self.wait_queue.is_empty() {
            // Get the current signal word (includes both old and new bits)
            let current_signals = self.signal_word.swap(0, Ordering::Acquire);

            // Wake all waiting threads with the signal bits
            while let Some(tcb) = self.wait_queue.dequeue() {
                let thread = &mut *tcb;

                // Store signal bits in thread's x0 register (return value)
                thread.context_mut().x0 = current_signals;

                // Make thread runnable
                thread.set_state(crate::objects::ThreadState::Runnable);
                crate::scheduler::enqueue(tcb);
            }
        }
    }

    /// Wait for notification signals (blocking)
    ///
    /// Blocks the current thread until the notification is signaled.
    /// Returns the signal bits that were set, and clears them.
    ///
    /// # Returns
    ///
    /// Signal bitmask (non-zero)
    ///
    /// # Safety
    ///
    /// Must be called with interrupts disabled to prevent races with scheduler
    pub unsafe fn wait(&mut self, current_tcb: *mut TCB) -> Option<u64> {
        // Check if there are already pending signals
        let signals = self.signal_word.swap(0, Ordering::Acquire);

        if signals != 0 {
            // Signals already pending, return immediately
            return Some(signals);
        }

        // No signals pending, block the thread
        let thread = &mut *current_tcb;

        // Block thread on notification
        thread.set_state(crate::objects::ThreadState::BlockedOnNotification {
            notification: self as *const _ as usize,
        });

        // Add to wait queue
        self.wait_queue.enqueue(current_tcb);

        // Return None to indicate the thread should block
        // The syscall handler will perform the actual context switch
        None
    }

    /// Poll for notification signals (non-blocking)
    ///
    /// Checks if any signals are pending without blocking.
    /// If signals are present, clears and returns them.
    ///
    /// # Returns
    ///
    /// Signal bitmask (0 if no signals pending)
    pub fn poll(&self) -> u64 {
        self.signal_word.swap(0, Ordering::Acquire)
    }

    /// Check if any signals are pending without clearing them
    pub fn peek(&self) -> u64 {
        self.signal_word.load(Ordering::Acquire)
    }

    /// Check if notification has waiting threads
    pub fn has_waiters(&self) -> bool {
        !self.wait_queue.is_empty()
    }

    /// Get number of waiting threads
    pub fn waiter_count(&self) -> usize {
        self.wait_queue.len()
    }

    /// Cancel all waiting threads
    ///
    /// Wakes all threads with signal bits set to 0, indicating cancellation.
    ///
    /// # Safety
    ///
    /// Must be called with interrupts disabled
    pub unsafe fn cancel_all(&mut self) {
        while let Some(tcb) = self.wait_queue.dequeue() {
            let thread = &mut *tcb;

            // Return 0 to indicate cancellation
            thread.context_mut().x0 = 0;

            // Make thread runnable
            thread.set_state(crate::objects::ThreadState::Runnable);
            crate::scheduler::enqueue(tcb);
        }

        // Clear any pending signals
        self.signal_word.store(0, Ordering::Release);
    }
}

impl core::fmt::Debug for Notification {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Notification")
            .field("signal_word", &self.signal_word.load(Ordering::Relaxed))
            .field("waiter_count", &self.wait_queue.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_create() {
        let notif = Notification::new();
        assert_eq!(notif.peek(), 0);
        assert!(!notif.has_waiters());
    }

    #[test]
    fn notification_poll() {
        let mut notif = Notification::new();

        // No signals initially
        assert_eq!(notif.poll(), 0);

        // Signal and poll
        unsafe {
            notif.signal(0b1010);
        }
        assert_eq!(notif.poll(), 0b1010);

        // Signals cleared after poll
        assert_eq!(notif.poll(), 0);
    }

    #[test]
    fn notification_accumulate_signals() {
        let mut notif = Notification::new();

        // Multiple signals accumulate
        unsafe {
            notif.signal(0b0001);
            notif.signal(0b0010);
            notif.signal(0b0100);
        }

        assert_eq!(notif.poll(), 0b0111);
    }
}
