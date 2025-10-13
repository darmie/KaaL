//! Endpoint Object Implementation
//!
//! Endpoints are rendezvous points for synchronous IPC (Inter-Process Communication).
//! They implement the fundamental mechanism for threads to communicate and transfer
//! capabilities between protection domains.
//!
//! ## Design
//!
//! Endpoints use a rendezvous model:
//! - Threads block on send/receive until a partner arrives
//! - When both sender and receiver are present, IPC occurs
//! - Message data is transferred directly (no buffering)
//! - Capabilities can be transferred alongside messages
//!
//! ## Queue Structure
//!
//! ```
//! Endpoint
//!   ├─ Send Queue: [TCB1] → [TCB2] → [TCB3]
//!   └─ Receive Queue: [TCB4] → [TCB5]
//! ```
//!
//! When a sender arrives and receivers are queued (or vice versa),
//! the IPC happens immediately and both threads are unblocked.

use super::TCB;

/// Endpoint - rendezvous point for synchronous IPC
///
/// Endpoints maintain two queues: one for threads waiting to send,
/// and one for threads waiting to receive. When both queues have
/// threads, IPC occurs immediately.
pub struct Endpoint {
    /// Queue of threads blocked waiting to send
    ///
    /// Threads in this queue have a message ready and are waiting
    /// for a receiver to arrive.
    send_queue: ThreadQueue,

    /// Queue of threads blocked waiting to receive
    ///
    /// Threads in this queue are waiting for a sender to arrive
    /// with a message.
    recv_queue: ThreadQueue,

    /// Badge value for this endpoint (optional)
    ///
    /// When capabilities to this endpoint are minted with different
    /// badges, the receiver can distinguish which capability was used
    /// to send the message.
    badge: u64,
}

impl Endpoint {
    /// Create a new endpoint
    pub fn new() -> Self {
        Self {
            send_queue: ThreadQueue::new(),
            recv_queue: ThreadQueue::new(),
            badge: 0,
        }
    }

    /// Create a new endpoint with a specific badge
    pub fn with_badge(badge: u64) -> Self {
        Self {
            send_queue: ThreadQueue::new(),
            recv_queue: ThreadQueue::new(),
            badge,
        }
    }

    /// Get the badge value
    #[inline]
    pub fn badge(&self) -> u64 {
        self.badge
    }

    /// Set the badge value
    #[inline]
    pub fn set_badge(&mut self, badge: u64) {
        self.badge = badge;
    }

    /// Check if there are threads waiting to send
    #[inline]
    pub fn has_senders(&self) -> bool {
        !self.send_queue.is_empty()
    }

    /// Check if there are threads waiting to receive
    #[inline]
    pub fn has_receivers(&self) -> bool {
        !self.recv_queue.is_empty()
    }

    /// Get the number of threads waiting to send
    #[inline]
    pub fn send_queue_len(&self) -> usize {
        self.send_queue.len()
    }

    /// Get the number of threads waiting to receive
    #[inline]
    pub fn recv_queue_len(&self) -> usize {
        self.recv_queue.len()
    }

    /// Queue a thread for send
    ///
    /// The thread will be blocked waiting for a receiver.
    /// If a receiver is already waiting, they can be matched immediately.
    ///
    /// # Safety
    /// - `tcb` must be a valid pointer to a TCB
    /// - The TCB must remain valid until unqueued
    pub unsafe fn queue_send(&mut self, tcb: *mut TCB) {
        debug_assert!(!tcb.is_null(), "Cannot queue null TCB");
        self.send_queue.enqueue(tcb);

        // Update thread state
        let endpoint_addr = self as *const _ as usize;
        (*tcb).block_on_send(endpoint_addr);
    }

    /// Queue a thread for receive
    ///
    /// The thread will be blocked waiting for a sender.
    /// If a sender is already waiting, they can be matched immediately.
    ///
    /// # Safety
    /// - `tcb` must be a valid pointer to a TCB
    /// - The TCB must remain valid until unqueued
    pub unsafe fn queue_receive(&mut self, tcb: *mut TCB) {
        debug_assert!(!tcb.is_null(), "Cannot queue null TCB");
        self.recv_queue.enqueue(tcb);

        // Update thread state
        let endpoint_addr = self as *const _ as usize;
        (*tcb).block_on_receive(endpoint_addr);
    }

    /// Try to match a sender and receiver for IPC
    ///
    /// Returns `Some((sender, receiver))` if a match is possible,
    /// or `None` if either queue is empty.
    ///
    /// This is the core rendezvous operation: if both queues have
    /// threads waiting, pop one from each and return them for IPC.
    pub fn try_match(&mut self) -> Option<(*mut TCB, *mut TCB)> {
        if self.has_senders() && self.has_receivers() {
            let sender = self.send_queue.dequeue().unwrap();
            let receiver = self.recv_queue.dequeue().unwrap();
            Some((sender, receiver))
        } else {
            None
        }
    }

    /// Dequeue the first thread from the send queue
    ///
    /// Returns the TCB pointer if a sender is waiting, or None if the queue is empty.
    pub fn dequeue_sender(&mut self) -> Option<*mut TCB> {
        self.send_queue.dequeue()
    }

    /// Dequeue the first thread from the receive queue
    ///
    /// Returns the TCB pointer if a receiver is waiting, or None if the queue is empty.
    pub fn dequeue_receiver(&mut self) -> Option<*mut TCB> {
        self.recv_queue.dequeue()
    }

    /// Dequeue a specific thread from the send queue
    ///
    /// Used for cancellation or timeout scenarios.
    ///
    /// # Safety
    /// - `tcb` must be a valid pointer that was previously queued
    pub unsafe fn dequeue_specific_sender(&mut self, tcb: *mut TCB) -> bool {
        self.send_queue.remove(tcb)
    }

    /// Dequeue a specific thread from the receive queue
    ///
    /// Used for cancellation or timeout scenarios.
    ///
    /// # Safety
    /// - `tcb` must be a valid pointer that was previously queued
    pub unsafe fn dequeue_specific_receiver(&mut self, tcb: *mut TCB) -> bool {
        self.recv_queue.remove(tcb)
    }

    /// Cancel all waiting threads
    ///
    /// Removes all threads from both queues and marks them as runnable.
    /// Used when an endpoint is destroyed or reset.
    ///
    /// # Safety
    /// - All TCB pointers in the queues must be valid
    pub unsafe fn cancel_all(&mut self) {
        // Cancel all senders
        while let Some(tcb) = self.send_queue.dequeue() {
            (*tcb).unblock();
        }

        // Cancel all receivers
        while let Some(tcb) = self.recv_queue.dequeue() {
            (*tcb).unblock();
        }
    }

    /// Check if the endpoint is idle (no queued threads)
    #[inline]
    pub fn is_idle(&self) -> bool {
        self.send_queue.is_empty() && self.recv_queue.is_empty()
    }
}

impl Default for Endpoint {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Endpoint")
            .field("badge", &self.badge)
            .field("send_queue_len", &self.send_queue.len())
            .field("recv_queue_len", &self.recv_queue.len())
            .finish()
    }
}

/// Thread queue - FIFO queue of TCB pointers
///
/// Used for maintaining waiting threads in endpoints.
/// Maximum number of threads that can be queued on an endpoint
const MAX_QUEUE_SIZE: usize = 256;

/// Threads are queued in FIFO order to ensure fairness.
///
/// Uses a fixed-size array to avoid heap allocation issues with spinlocks.
struct ThreadQueue {
    /// Fixed-size array of TCB pointers
    threads: [*mut TCB; MAX_QUEUE_SIZE],
    /// Number of threads currently in the queue
    count: usize,
}

impl ThreadQueue {
    /// Create a new empty thread queue
    fn new() -> Self {
        Self {
            threads: [core::ptr::null_mut(); MAX_QUEUE_SIZE],
            count: 0,
        }
    }

    /// Check if the queue is empty
    #[inline]
    fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get the number of threads in the queue
    #[inline]
    fn len(&self) -> usize {
        self.count
    }

    /// Add a thread to the back of the queue (FIFO)
    fn enqueue(&mut self, tcb: *mut TCB) {
        debug_assert!(self.count < MAX_QUEUE_SIZE, "Thread queue overflow");
        if self.count < MAX_QUEUE_SIZE {
            self.threads[self.count] = tcb;
            self.count += 1;
        }
    }

    /// Remove and return the thread at the front of the queue
    fn dequeue(&mut self) -> Option<*mut TCB> {
        if self.count == 0 {
            None
        } else {
            let tcb = self.threads[0];
            // Shift elements forward
            for i in 0..self.count - 1 {
                self.threads[i] = self.threads[i + 1];
            }
            self.threads[self.count - 1] = core::ptr::null_mut();
            self.count -= 1;
            Some(tcb)
        }
    }

    /// Remove a specific thread from the queue
    ///
    /// Returns true if the thread was found and removed.
    fn remove(&mut self, tcb: *mut TCB) -> bool {
        for i in 0..self.count {
            if self.threads[i] == tcb {
                // Shift elements forward
                for j in i..self.count - 1 {
                    self.threads[j] = self.threads[j + 1];
                }
                self.threads[self.count - 1] = core::ptr::null_mut();
                self.count -= 1;
                return true;
            }
        }
        false
    }

    /// Peek at the front thread without removing it
    #[allow(dead_code)]
    fn peek(&self) -> Option<*mut TCB> {
        if self.count > 0 {
            Some(self.threads[0])
        } else {
            None
        }
    }
}

// Thread-safe marker - Endpoints are managed by the kernel
// and access is synchronized through kernel entry/exit
unsafe impl Send for Endpoint {}
unsafe impl Sync for Endpoint {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::VirtAddr;
    use crate::objects::CNode;

    #[test]
    fn endpoint_creation() {
        let ep = Endpoint::new();
        assert_eq!(ep.badge(), 0);
        assert!(!ep.has_senders());
        assert!(!ep.has_receivers());
        assert!(ep.is_idle());
    }

    #[test]
    fn endpoint_with_badge() {
        let ep = Endpoint::with_badge(0x1234);
        assert_eq!(ep.badge(), 0x1234);
    }

    #[test]
    fn endpoint_queue_operations() {
        let mut ep = Endpoint::new();

        // Create dummy TCBs for testing
        let mut cnode_memory = [crate::objects::Capability::null(); 16];
        let cnode_ptr = &mut cnode_memory[0] as *mut _ as *mut CNode;

        unsafe {
            let mut sender = TCB::new(
                1,
                cnode_ptr,
                0x40000000,
                VirtAddr::new(0x10000000),
                0x200000,
                0x300000,
            );
            let sender_ptr = &mut sender as *mut TCB;

            let mut receiver = TCB::new(
                2,
                cnode_ptr,
                0x40000000,
                VirtAddr::new(0x10000000),
                0x200000,
                0x300000,
            );
            let receiver_ptr = &mut receiver as *mut TCB;

            // Queue sender
            ep.queue_send(sender_ptr);
            assert!(ep.has_senders());
            assert_eq!(ep.send_queue_len(), 1);
            assert_eq!(sender.state(), ThreadState::BlockedOnSend { endpoint: &ep as *const _ as usize });

            // Queue receiver
            ep.queue_receive(receiver_ptr);
            assert!(ep.has_receivers());
            assert_eq!(ep.recv_queue_len(), 1);
            assert_eq!(receiver.state(), ThreadState::BlockedOnReceive { endpoint: &ep as *const _ as usize });

            // Try to match
            let matched = ep.try_match();
            assert!(matched.is_some());
            let (s, r) = matched.unwrap();
            assert_eq!(s, sender_ptr);
            assert_eq!(r, receiver_ptr);

            // Queues should now be empty
            assert!(!ep.has_senders());
            assert!(!ep.has_receivers());
            assert!(ep.is_idle());
        }
    }

    #[test]
    fn endpoint_remove_thread() {
        let mut ep = Endpoint::new();
        let mut cnode_memory = [crate::objects::Capability::null(); 16];
        let cnode_ptr = &mut cnode_memory[0] as *mut _ as *mut CNode;

        unsafe {
            let mut tcb1 = TCB::new(1, cnode_ptr, 0x40000000, VirtAddr::new(0x10000000), 0x200000, 0x300000);
            let mut tcb2 = TCB::new(2, cnode_ptr, 0x40000000, VirtAddr::new(0x10000000), 0x200000, 0x300000);
            let tcb1_ptr = &mut tcb1 as *mut TCB;
            let tcb2_ptr = &mut tcb2 as *mut TCB;

            // Queue two senders
            ep.queue_send(tcb1_ptr);
            ep.queue_send(tcb2_ptr);
            assert_eq!(ep.send_queue_len(), 2);

            // Remove first sender
            assert!(ep.dequeue_sender(tcb1_ptr));
            assert_eq!(ep.send_queue_len(), 1);

            // Try to remove again (should fail)
            assert!(!ep.dequeue_sender(tcb1_ptr));

            // Remove second sender
            assert!(ep.dequeue_sender(tcb2_ptr));
            assert_eq!(ep.send_queue_len(), 0);
            assert!(ep.is_idle());
        }
    }

    #[test]
    fn endpoint_cancel_all() {
        let mut ep = Endpoint::new();
        let mut cnode_memory = [crate::objects::Capability::null(); 16];
        let cnode_ptr = &mut cnode_memory[0] as *mut _ as *mut CNode;

        unsafe {
            let mut sender = TCB::new(1, cnode_ptr, 0x40000000, VirtAddr::new(0x10000000), 0x200000, 0x300000);
            let mut receiver = TCB::new(2, cnode_ptr, 0x40000000, VirtAddr::new(0x10000000), 0x200000, 0x300000);

            ep.queue_send(&mut sender as *mut TCB);
            ep.queue_receive(&mut receiver as *mut TCB);

            assert!(!ep.is_idle());

            // Cancel all
            ep.cancel_all();

            assert!(ep.is_idle());
            assert_eq!(sender.state(), ThreadState::Runnable);
            assert_eq!(receiver.state(), ThreadState::Runnable);
        }
    }
}
