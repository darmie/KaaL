//! Scheduler - Thread Scheduling & Context Switching
//!
//! This module implements the KaaL scheduler which manages:
//! - Thread ready queues (per-priority)
//! - Context switching
//! - Yielding and preemption
//! - Integration with IPC blocking
//!
//! ## Architecture
//!
//! The scheduler uses **fixed-priority preemptive scheduling** with **round-robin**
//! within each priority level. Key features:
//!
//! - 256 priority levels (0 = highest, 255 = lowest)
//! - O(1) scheduling via priority bitmap
//! - Deterministic behavior
//! - Explicit yield points (no automatic preemption yet)
//!
//! ## Thread States
//!
//! - **Running**: Currently executing on CPU
//! - **Runnable**: Ready to run, in scheduler queue
//! - **BlockedOnSend**: Waiting to send IPC message
//! - **BlockedOnReceive**: Waiting to receive IPC message
//! - **BlockedOnReply**: Waiting for IPC reply
//! - **Inactive**: Not scheduled
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Initialize scheduler
//! scheduler::init(idle_tcb);
//!
//! // Add thread to ready queue
//! scheduler::enqueue(tcb);
//!
//! // Yield to next thread
//! scheduler::yield_current();
//!
//! // Block current thread (called by IPC)
//! scheduler::block_current();
//! ```

use crate::objects::TCB;
use core::ptr;
use crate::ksched_debug;

mod types;
pub mod timer;

pub use types::{Scheduler, ThreadQueue, SchedulerError};

/// Global scheduler instance
///
/// This is initialized during boot and accessed by all scheduling operations.
/// Safety: Only accessed from kernel code with interrupts disabled.
static mut SCHEDULER: Option<Scheduler> = None;

/// Initialize the scheduler
///
/// This must be called once during boot before any scheduling operations.
///
/// # Arguments
///
/// * `idle_tcb` - The idle thread that runs when no other threads are ready
///
/// # Safety
///
/// - Must be called exactly once during boot
/// - Must be called with interrupts disabled
/// - idle_tcb must be valid for the lifetime of the kernel
pub unsafe fn init(idle_tcb: *mut TCB) {
    SCHEDULER = Some(Scheduler::new(idle_tcb));
}

/// Get a reference to the global scheduler
///
/// # Safety
///
/// - Scheduler must be initialized (init() called)
/// - Interrupts should be disabled when calling
unsafe fn scheduler() -> &'static mut Scheduler {
    SCHEDULER.as_mut().expect("Scheduler not initialized")
}

/// Get the currently running thread
///
/// Returns a pointer to the TCB of the thread currently executing on the CPU.
///
/// # Safety
///
/// - Scheduler must be initialized
pub unsafe fn current_thread() -> *mut TCB {
    scheduler().current()
}

/// Set the current running thread
///
/// This is called by context switcher to update the current thread pointer.
///
/// # Safety
///
/// - Scheduler must be initialized
/// - tcb must be valid
unsafe fn set_current_thread(tcb: *mut TCB) {
    scheduler().set_current(tcb);
}

/// Set the current thread (public for testing)
///
/// **FOR TESTING ONLY** - Sets the scheduler's current thread pointer.
/// In normal operation, this is managed internally by the scheduler.
///
/// # Safety
///
/// - Scheduler must be initialized
/// - tcb must be valid
/// - Thread should be in Running state
pub unsafe fn test_set_current_thread(tcb: *mut TCB) {
    if !tcb.is_null() {
        (*tcb).set_state(crate::objects::ThreadState::Running);
        set_current_thread(tcb);
    }
}

/// Add a thread to the ready queue
///
/// The thread is added to the tail of its priority's queue and becomes
/// eligible for scheduling.
///
/// # Arguments
///
/// * `tcb` - Thread to enqueue (must be in Runnable state)
///
/// # Safety
///
/// - tcb must be valid
/// - Thread must not already be in a queue
///
/// # Note
///
/// If scheduler is not initialized, this is a no-op. This allows early
/// boot code to create processes before scheduler init.
pub unsafe fn enqueue(tcb: *mut TCB) {
    if tcb.is_null() {
        crate::ksched_debug!("[sched] enqueue: null TCB, skipping");
        return;
    }

    // Check if scheduler is initialized
    if SCHEDULER.is_none() {
        crate::kprintln!("[sched] enqueue: scheduler not initialized, skipping TCB at {:#x}", tcb as usize);
        return; // Silently skip if not initialized
    }

    // crate::kprintln!("[sched] enqueue: adding TCB at {:#x} to scheduler", tcb as usize);
    scheduler().enqueue(tcb);
}

/// Remove a thread from the ready queue
///
/// Removes the thread from whichever priority queue it's in.
///
/// # Arguments
///
/// * `tcb` - Thread to dequeue
///
/// # Safety
///
/// - Scheduler must be initialized
/// - tcb must be valid
pub unsafe fn dequeue(tcb: *mut TCB) {
    if tcb.is_null() {
        return;
    }

    scheduler().dequeue(tcb);
}

/// Pick the next thread to run
///
/// Selects the highest-priority runnable thread. If no threads are ready,
/// returns the idle thread.
///
/// # Returns
///
/// Pointer to the TCB of the next thread to run.
///
/// # Safety
///
/// - Scheduler must be initialized
pub unsafe fn schedule() -> *mut TCB {
    scheduler().schedule()
}

/// Yield the current thread
///
/// Saves the current thread's context, picks the next thread, and switches to it.
/// If no other threads are ready, continues running the current thread.
///
/// # Safety
///
/// - Scheduler must be initialized
/// - Must be called with interrupts disabled
/// - Current thread must be valid
pub unsafe fn yield_current() {
    let current = current_thread();

    if current.is_null() {
        // No current thread (shouldn't happen), just schedule
        let next = schedule();
        if !next.is_null() {
            set_current_thread(next);
        }
        return;
    }

    // Get current thread reference
    let current_tcb = &mut *current;

    // If current thread is still runnable, re-enqueue it
    if current_tcb.state() == crate::objects::ThreadState::Running {
        current_tcb.set_state(crate::objects::ThreadState::Runnable);
        enqueue(current);
    }

    // Pick next thread
    let next = schedule();

    if next == current {
        // Same thread, just keep running
        current_tcb.set_state(crate::objects::ThreadState::Running);
        return;
    }

    // Different thread, need to context switch
    let next_tcb = &mut *next;
    next_tcb.set_state(crate::objects::ThreadState::Running);
    set_current_thread(next);

    // Perform context switch (assembly)
    // This saves current thread's registers and restores next thread's registers
    crate::arch::aarch64::context_switch::switch_context(current, next);

    // Note: Execution continues here AFTER another thread yields back to us
}

/// Block the current thread
///
/// Removes the current thread from the ready queue and yields to another thread.
/// The current thread's state should already be set to the appropriate blocked state
/// (BlockedOnSend, BlockedOnReceive, BlockedOnReply) before calling this.
///
/// # Safety
///
/// - Scheduler must be initialized
/// - Current thread must be valid
/// - Current thread's state must be set to a blocked state before calling
pub unsafe fn block_current() {
    let current = current_thread();

    if current.is_null() {
        return;
    }

    // Current thread is already in a blocked state (set by IPC code)
    // Just yield to next thread
    let next = schedule();

    if next.is_null() {
        // No other threads, can't block (shouldn't happen with idle thread)
        crate::ksched_debug!("[sched] WARNING: No threads available to schedule");
        return;
    }

    // Switch to next thread
    let next_tcb = &mut *next;
    next_tcb.set_state(crate::objects::ThreadState::Running);
    set_current_thread(next);

    // Perform context switch (assembly)
    crate::arch::aarch64::context_switch::switch_context(current, next);

    // Note: Execution continues here AFTER we're unblocked and scheduled again
}

/// Unblock a thread and make it runnable
///
/// Changes the thread's state to Runnable and adds it to the ready queue.
/// This is typically called by IPC when a blocked thread can proceed.
///
/// # Arguments
///
/// * `tcb` - Thread to unblock
///
/// # Safety
///
/// - Scheduler must be initialized
/// - tcb must be valid
/// - Thread must currently be in a blocked state
pub unsafe fn unblock(tcb: *mut TCB) {
    if tcb.is_null() {
        return;
    }

    let tcb_ref = &mut *tcb;

    // Change state to runnable
    tcb_ref.set_state(crate::objects::ThreadState::Runnable);

    // Add to ready queue
    enqueue(tcb);

    // Check if we should preempt current thread
    // If unblocked thread has higher priority than current, should reschedule
    let current = current_thread();
    if !current.is_null() {
        let current_priority = (*current).priority();
        let unblocked_priority = tcb_ref.priority();

        // Lower priority number = higher priority
        // If unblocked thread has higher priority, preempt current
        if unblocked_priority < current_priority {
            yield_current();
        }
    }
}

/// Set thread priority and reschedule if needed
///
/// Changes the thread's priority and re-queues it if necessary.
///
/// # Arguments
///
/// * `tcb` - Thread to modify
/// * `priority` - New priority (0 = highest, 255 = lowest)
///
/// # Safety
///
/// - Scheduler must be initialized
/// - tcb must be valid
pub unsafe fn set_priority(tcb: *mut TCB, priority: u8) {
    if tcb.is_null() {
        return;
    }

    let tcb_ref = &mut *tcb;
    let old_priority = tcb_ref.priority();

    if old_priority == priority {
        return; // No change
    }

    // If thread is in ready queue, need to move it
    if tcb_ref.state() == crate::objects::ThreadState::Runnable {
        // Remove from old priority queue
        dequeue(tcb);

        // Update priority
        tcb_ref.set_priority(priority);

        // Add to new priority queue
        enqueue(tcb);
    } else {
        // Just update priority (not in queue)
        tcb_ref.set_priority(priority);
    }

    // Check if we should reschedule
    // If priority increased above current thread, should preempt
    let current = current_thread();
    if !current.is_null() && tcb != current {
        let current_priority = (*current).priority();

        // Lower priority number = higher priority
        // If modified thread now has higher priority than current, preempt
        if priority < current_priority && tcb_ref.state() == crate::objects::ThreadState::Runnable {
            yield_current();
        }
    }
}
