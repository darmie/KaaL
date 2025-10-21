//! Scheduler Types
//!
//! Core data structures for the KaaL scheduler.

use crate::objects::TCB;
use core::ptr;

/// Number of priority levels (0 = highest, 255 = lowest)
pub const NUM_PRIORITIES: usize = 256;

/// Scheduler - manages runnable threads
///
/// The scheduler maintains per-priority ready queues and selects
/// the highest-priority thread to run next.
pub struct Scheduler {
    /// Ready queues per priority level
    /// Index 0 = highest priority, 255 = lowest
    ready_queues: [ThreadQueue; NUM_PRIORITIES],

    /// Currently running thread
    current: *mut TCB,

    /// Idle thread (runs when nothing else is ready)
    idle: *mut TCB,

    /// Priority bitmap for O(1) lookup
    ///
    /// Each bit represents whether that priority level has runnable threads.
    /// Divided into 4 x u64 = 256 bits total.
    /// priority_bitmap[0] covers priorities 0-63
    /// priority_bitmap[1] covers priorities 64-127
    /// priority_bitmap[2] covers priorities 128-191
    /// priority_bitmap[3] covers priorities 192-255
    priority_bitmap: [u64; 4],
}

impl Scheduler {
    /// Create a new scheduler
    ///
    /// # Arguments
    ///
    /// * `idle_tcb` - The idle thread (runs when no other threads ready)
    pub fn new(idle_tcb: *mut TCB) -> Self {
        Self {
            ready_queues: [ThreadQueue::new(); NUM_PRIORITIES],
            current: idle_tcb,
            idle: idle_tcb,
            priority_bitmap: [0; 4],
        }
    }

    /// Get the currently running thread
    #[inline]
    pub fn current(&self) -> *mut TCB {
        self.current
    }

    /// Set the currently running thread
    #[inline]
    pub fn set_current(&mut self, tcb: *mut TCB) {
        self.current = tcb;
    }

    /// Add thread to ready queue
    ///
    /// # Safety
    ///
    /// - tcb must be valid
    /// - Thread must not already be in a queue
    pub unsafe fn enqueue(&mut self, tcb: *mut TCB) {
        if tcb.is_null() {
            return;
        }

        let priority = (*tcb).priority() as usize;
        // crate::kprintln!("[sched] enqueue: TCB at {:#x} has priority {}", tcb as usize, priority);
        if priority >= NUM_PRIORITIES {
            // crate::kprintln!("[sched] enqueue: invalid priority {} >= {}", priority, NUM_PRIORITIES);
            return; // Invalid priority
        }

        // Add to priority queue
        self.ready_queues[priority].enqueue(tcb);

        // Set bit in bitmap
        self.set_priority_bit(priority as u8);
    }

    /// Remove thread from ready queue
    ///
    /// # Safety
    ///
    /// - tcb must be valid
    pub unsafe fn dequeue(&mut self, tcb: *mut TCB) {
        if tcb.is_null() {
            return;
        }

        let priority = (*tcb).priority() as usize;
        if priority >= NUM_PRIORITIES {
            return;
        }

        // Remove from priority queue
        self.ready_queues[priority].dequeue(tcb);

        // Clear bit in bitmap if queue now empty
        if self.ready_queues[priority].is_empty() {
            self.clear_priority_bit(priority as u8);
        }
    }

    /// Pick the next thread to run (strict priority-preemptive)
    ///
    /// Returns the highest-priority runnable thread, or the idle thread
    /// if no threads are ready.
    ///
    /// Used for timer-based preemption to enforce strict priority order.
    pub unsafe fn schedule(&mut self) -> *mut TCB {
        // Debug: show what's available to schedule (commented out to reduce noise)
        // crate::kprintln!("[sched] schedule: looking for next thread");

        // Find highest priority with runnable threads
        if let Some(priority) = self.find_highest_priority() {
            // crate::kprintln!("[sched] schedule: highest priority with threads = {}", priority);
            // Dequeue from that priority level
            if let Some(tcb) = self.ready_queues[priority as usize].dequeue_head() {
                // crate::kprintln!("[sched] schedule: picked TCB at {:#x} with priority {}",
                //                  tcb as usize, priority);
                // Update bitmap if queue now empty
                if self.ready_queues[priority as usize].is_empty() {
                    self.clear_priority_bit(priority);
                }
                return tcb;
            }
        }

        crate::ksched_debug!("[sched] schedule: no ready threads, returning idle TCB 0");
        // No runnable threads, return idle
        self.idle
    }

    /// Pick the next thread to run with fair round-robin across all priorities
    ///
    /// This provides cooperative multitasking fairness when threads explicitly yield.
    /// The strategy:
    /// 1. First try round-robin at current priority (same-priority fairness)
    /// 2. Then check higher priorities (preemption by higher priority)
    /// 3. Finally scan lower priorities (allow progress even if lower priority)
    ///
    /// This ensures that yield_now() actually allows ALL threads to make progress,
    /// not just higher-priority ones.
    ///
    /// # Arguments
    /// * `current_priority` - Priority of the thread that is yielding
    pub unsafe fn schedule_fair(&mut self, current_priority: u8) -> *mut TCB {
        // Step 1: Try same priority first (true round-robin at this level)
        if !self.ready_queues[current_priority as usize].is_empty() {
            if let Some(tcb) = self.ready_queues[current_priority as usize].dequeue_head() {
                if self.ready_queues[current_priority as usize].is_empty() {
                    self.clear_priority_bit(current_priority);
                }
                return tcb;
            }
        }

        // Step 2: Check higher priority (0 to current_priority - 1)
        // Higher priority should still preempt even on yield
        for priority in 0..current_priority {
            if !self.ready_queues[priority as usize].is_empty() {
                if let Some(tcb) = self.ready_queues[priority as usize].dequeue_head() {
                    if self.ready_queues[priority as usize].is_empty() {
                        self.clear_priority_bit(priority);
                    }
                    return tcb;
                }
            }
        }

        // Step 3: Check lower priorities (current_priority + 1 to 255)
        // This is the KEY difference - allow lower priority threads to run when yielding
        for priority in (current_priority + 1)..=255 {
            if !self.ready_queues[priority as usize].is_empty() {
                if let Some(tcb) = self.ready_queues[priority as usize].dequeue_head() {
                    if self.ready_queues[priority as usize].is_empty() {
                        self.clear_priority_bit(priority);
                    }
                    return tcb;
                }
            }
        }

        // No runnable threads, return idle
        self.idle
    }

    /// Find the highest priority level with runnable threads
    ///
    /// Returns None if no threads are ready.
    fn find_highest_priority(&self) -> Option<u8> {
        // Check each u64 in the bitmap (highest priority first)
        for (chunk_idx, &chunk) in self.priority_bitmap.iter().enumerate() {
            if chunk != 0 {
                // Found non-empty chunk, find highest bit (lowest priority number)
                let leading_zeros = chunk.leading_zeros() as usize;
                // We stored priority P at bit_idx = 63 - (P % 64)
                // So bit 63 (MSB) = priority 0, bit 0 (LSB) = priority 63
                // leading_zeros tells us how many zeros before first 1
                // For bit 63 (MSB): leading_zeros = 0 → priority_in_chunk = 0
                // For bit 62: leading_zeros = 1 → priority_in_chunk = 1
                // So priority_in_chunk = leading_zeros
                let priority_in_chunk = leading_zeros;
                let priority = (chunk_idx * 64) + priority_in_chunk;
                return Some(priority as u8);
            }
        }

        None
    }

    /// Set a bit in the priority bitmap
    fn set_priority_bit(&mut self, priority: u8) {
        let priority = priority as usize;
        let chunk_idx = priority / 64;
        let bit_idx = 63 - (priority % 64); // Reverse bit order for leading_zeros
        self.priority_bitmap[chunk_idx] |= 1u64 << bit_idx;
    }

    /// Clear a bit in the priority bitmap
    fn clear_priority_bit(&mut self, priority: u8) {
        let priority = priority as usize;
        let chunk_idx = priority / 64;
        let bit_idx = 63 - (priority % 64); // Reverse bit order for leading_zeros
        self.priority_bitmap[chunk_idx] &= !(1u64 << bit_idx);
    }
}

/// Thread queue - linked list of TCBs
///
/// Each priority level has its own queue. Threads are added to the tail
/// and removed from the head (FIFO/round-robin within priority).
///
/// For Phase 1, using a simple array-based implementation.
/// Future: Could optimize with intrusive linked list using TCB fields.
#[derive(Clone, Copy)]
pub struct ThreadQueue {
    /// Array of TCB pointers
    threads: [*mut TCB; MAX_QUEUE_SIZE],

    /// Number of threads in queue
    count: usize,
}

/// Maximum threads per priority queue
///
/// This is a compile-time limit. In practice, we won't hit this
/// unless many threads have the same priority.
const MAX_QUEUE_SIZE: usize = 64;

impl ThreadQueue {
    /// Create an empty thread queue
    pub const fn new() -> Self {
        Self {
            threads: [ptr::null_mut(); MAX_QUEUE_SIZE],
            count: 0,
        }
    }

    /// Check if queue is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get number of threads in queue
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Add thread to tail of queue
    ///
    /// # Safety
    ///
    /// - tcb must be valid
    /// - Thread must not already be in the queue
    pub unsafe fn enqueue(&mut self, tcb: *mut TCB) {
        if self.count >= MAX_QUEUE_SIZE {
            // Queue full (shouldn't happen with reasonable thread counts)
            crate::ksched_debug!("[sched] WARNING: Thread queue full, dropping enqueue");
            return;
        }

        self.threads[self.count] = tcb;
        self.count += 1;
    }

    /// Remove thread from queue
    ///
    /// Searches for the thread and removes it, preserving order.
    ///
    /// # Safety
    ///
    /// - tcb must be valid
    pub unsafe fn dequeue(&mut self, tcb: *mut TCB) -> bool {
        // Find the thread
        for i in 0..self.count {
            if self.threads[i] == tcb {
                // Found it, shift everything after it down
                for j in i..self.count - 1 {
                    self.threads[j] = self.threads[j + 1];
                }
                self.threads[self.count - 1] = ptr::null_mut();
                self.count -= 1;
                return true;
            }
        }

        false // Not found
    }

    /// Dequeue from head (for scheduling)
    ///
    /// Returns the thread at the head of the queue and removes it.
    pub unsafe fn dequeue_head(&mut self) -> Option<*mut TCB> {
        if self.count == 0 {
            return None;
        }

        let head = self.threads[0];

        // Shift everything down
        for i in 0..self.count - 1 {
            self.threads[i] = self.threads[i + 1];
        }
        self.threads[self.count - 1] = ptr::null_mut();
        self.count -= 1;

        Some(head)
    }
}

/// Scheduler errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    /// Scheduler not initialized
    NotInitialized,

    /// Invalid priority
    InvalidPriority,

    /// Queue full
    QueueFull,

    /// Thread not found
    ThreadNotFound,
}
