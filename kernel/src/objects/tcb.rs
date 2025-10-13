//! Thread Control Block (TCB) Implementation
//!
//! TCBs represent threads of execution in KaaL. Each TCB contains:
//! - CPU context (trap frame for save/restore)
//! - CSpace root (capability address space)
//! - VSpace root (virtual address space)
//! - IPC buffer location
//! - Thread state and scheduling information
//!
//! ## Thread Lifecycle
//!
//! ```
//! Inactive → Running → Blocked → Running → Inactive
//!            ↓                        ↑
//!            └────────────────────────┘
//!                (context switch)
//! ```

use crate::arch::aarch64::context::TrapFrame;
use crate::memory::VirtAddr;
use super::CNode;

/// Thread Control Block - represents a thread of execution
///
/// TCBs are the fundamental unit of execution in KaaL. Each thread has its own:
/// - CPU context (saved when not running)
/// - Capability space (CSpace root)
/// - Virtual address space (VSpace root)
/// - IPC buffer
/// - Scheduling parameters
#[repr(C)]
pub struct TCB {
    /// CPU context (trap frame) - saved when thread is not running
    ///
    /// This contains all general-purpose registers (x0-x30) plus special
    /// registers (ELR, SPSR, etc.) needed to resume execution.
    context: TrapFrame,

    /// CSpace root - pointer to the thread's capability space
    ///
    /// All capability lookups start from this CNode. Typically points to
    /// a top-level CNode that may contain pointers to other CNodes forming
    /// a capability address space tree.
    cspace_root: *mut CNode,

    /// VSpace root - physical address of the page table root
    ///
    /// This is the physical address loaded into TTBR0_EL0 when switching
    /// to this thread, defining its virtual address space.
    vspace_root: usize,

    /// IPC buffer virtual address
    ///
    /// User-accessible memory region for IPC message registers and
    /// capability transfer. Must be mapped in the thread's VSpace.
    ipc_buffer: VirtAddr,

    /// Thread state
    state: ThreadState,

    /// Thread priority (0 = lowest, 255 = highest)
    priority: u8,

    /// Time slice remaining (in ticks)
    time_slice: u32,

    /// Thread ID (for debugging)
    tid: usize,
}

/// Thread state - lifecycle states of a thread
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// Thread is not scheduled (initial state or terminated)
    Inactive,

    /// Thread is currently running on a CPU
    Running,

    /// Thread is ready to run but not currently scheduled
    Runnable,

    /// Thread is blocked waiting to receive on an endpoint
    BlockedOnReceive {
        /// Endpoint address the thread is waiting on
        endpoint: usize,
    },

    /// Thread is blocked waiting to send on an endpoint
    BlockedOnSend {
        /// Endpoint address the thread is waiting on
        endpoint: usize,
    },

    /// Thread is blocked waiting for a reply
    BlockedOnReply,

    /// Thread is blocked waiting on a notification
    BlockedOnNotification {
        /// Notification object address
        notification: usize,
    },
}

impl TCB {
    /// Default priority for new threads
    pub const DEFAULT_PRIORITY: u8 = 128;

    /// Default time slice (in ticks)
    pub const DEFAULT_TIME_SLICE: u32 = 10;

    /// Create a new TCB in the Inactive state
    ///
    /// # Arguments
    /// * `tid` - Thread ID (for debugging/tracking)
    /// * `cspace_root` - Pointer to the thread's CSpace root CNode
    /// * `vspace_root` - Physical address of the page table root
    /// * `ipc_buffer` - Virtual address of the IPC buffer
    /// * `entry_point` - Initial program counter (ELR_EL1)
    /// * `stack_pointer` - Initial stack pointer (SP_EL0)
    ///
    /// # Safety
    /// - `cspace_root` must be a valid pointer to a CNode
    /// - `vspace_root` must be a valid page table root
    /// - `ipc_buffer` must be mapped in the VSpace
    pub unsafe fn new(
        tid: usize,
        cspace_root: *mut CNode,
        vspace_root: usize,
        ipc_buffer: VirtAddr,
        entry_point: u64,
        stack_pointer: u64,
    ) -> Self {
        let mut context = TrapFrame::new();

        // Set up initial context
        context.elr_el1 = entry_point;
        context.sp_el0 = stack_pointer;

        // Set SPSR for EL0 execution:
        // - D=1 (Debug exceptions masked)
        // - A=1 (SError masked)
        // - I=1 (IRQ masked)
        // - F=1 (FIQ masked)
        // - EL=0 (Exception Level 0)
        context.spsr_el1 = 0x3c5; // All interrupts masked, EL0

        Self {
            context,
            cspace_root,
            vspace_root,
            ipc_buffer,
            state: ThreadState::Inactive,
            priority: Self::DEFAULT_PRIORITY,
            time_slice: Self::DEFAULT_TIME_SLICE,
            tid,
        }
    }

    /// Get the thread ID
    #[inline]
    pub fn tid(&self) -> usize {
        self.tid
    }

    /// Get the thread state
    #[inline]
    pub fn state(&self) -> ThreadState {
        self.state
    }

    /// Set the thread state
    #[inline]
    pub fn set_state(&mut self, state: ThreadState) {
        self.state = state;
    }

    /// Get the thread priority
    #[inline]
    pub fn priority(&self) -> u8 {
        self.priority
    }

    /// Set the thread priority
    #[inline]
    pub fn set_priority(&mut self, priority: u8) {
        self.priority = priority;
    }

    /// Get the time slice remaining
    #[inline]
    pub fn time_slice(&self) -> u32 {
        self.time_slice
    }

    /// Set the time slice
    #[inline]
    pub fn set_time_slice(&mut self, time_slice: u32) {
        self.time_slice = time_slice;
    }

    /// Decrement the time slice
    ///
    /// Returns true if the time slice is exhausted.
    #[inline]
    pub fn tick(&mut self) -> bool {
        if self.time_slice > 0 {
            self.time_slice -= 1;
        }
        self.time_slice == 0
    }

    /// Refill the time slice
    #[inline]
    pub fn refill_time_slice(&mut self) {
        self.time_slice = Self::DEFAULT_TIME_SLICE;
    }

    /// Get a reference to the CPU context
    #[inline]
    pub fn context(&self) -> &TrapFrame {
        &self.context
    }

    /// Get a mutable reference to the CPU context
    #[inline]
    pub fn context_mut(&mut self) -> &mut TrapFrame {
        &mut self.context
    }

    /// Get the CSpace root pointer
    #[inline]
    pub fn cspace_root(&self) -> *mut CNode {
        self.cspace_root
    }

    /// Get the VSpace root physical address
    #[inline]
    pub fn vspace_root(&self) -> usize {
        self.vspace_root
    }

    /// Get the IPC buffer virtual address
    #[inline]
    pub fn ipc_buffer(&self) -> VirtAddr {
        self.ipc_buffer
    }

    /// Check if the thread is runnable
    #[inline]
    pub fn is_runnable(&self) -> bool {
        matches!(self.state, ThreadState::Runnable | ThreadState::Running)
    }

    /// Check if the thread is blocked
    #[inline]
    pub fn is_blocked(&self) -> bool {
        matches!(
            self.state,
            ThreadState::BlockedOnReceive { .. }
                | ThreadState::BlockedOnSend { .. }
                | ThreadState::BlockedOnReply
                | ThreadState::BlockedOnNotification { .. }
        )
    }

    /// Activate the thread (make it runnable)
    pub fn activate(&mut self) {
        if matches!(self.state, ThreadState::Inactive) {
            self.state = ThreadState::Runnable;
            self.refill_time_slice();
        }
    }

    /// Deactivate the thread (make it inactive)
    pub fn deactivate(&mut self) {
        self.state = ThreadState::Inactive;
    }

    /// Block the thread on an endpoint for receive
    pub fn block_on_receive(&mut self, endpoint: usize) {
        self.state = ThreadState::BlockedOnReceive { endpoint };
    }

    /// Block the thread on an endpoint for send
    pub fn block_on_send(&mut self, endpoint: usize) {
        self.state = ThreadState::BlockedOnSend { endpoint };
    }

    /// Block the thread waiting for a reply
    pub fn block_on_reply(&mut self) {
        self.state = ThreadState::BlockedOnReply;
    }

    /// Unblock the thread (make it runnable)
    pub fn unblock(&mut self) {
        if self.is_blocked() {
            self.state = ThreadState::Runnable;
        }
    }

    /// Resume the thread (mark as running)
    pub fn resume(&mut self) {
        if matches!(self.state, ThreadState::Runnable) {
            self.state = ThreadState::Running;
        }
    }

    /// Suspend the thread (mark as runnable)
    pub fn suspend(&mut self) {
        if matches!(self.state, ThreadState::Running) {
            self.state = ThreadState::Runnable;
        }
    }
}

impl core::fmt::Debug for TCB {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TCB")
            .field("tid", &self.tid)
            .field("state", &self.state)
            .field("priority", &self.priority)
            .field("time_slice", &self.time_slice)
            .field("cspace_root", &format_args!("{:p}", self.cspace_root))
            .field("vspace_root", &format_args!("{:#x}", self.vspace_root))
            .field("ipc_buffer", &format_args!("{:#x}", self.ipc_buffer.as_usize()))
            .field("pc", &format_args!("{:#x}", self.context.elr_el1))
            .field("sp", &format_args!("{:#x}", self.context.sp_el0))
            .finish()
    }
}

// Thread-safe marker - TCBs are managed by the kernel
// and access is synchronized through kernel entry/exit
unsafe impl Send for TCB {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::PhysAddr;

    #[test]
    fn tcb_creation() {
        // Create a dummy CNode for testing
        let mut cnode_memory = [crate::objects::Capability::null(); 16];
        let cnode_ptr = &mut cnode_memory[0] as *mut _ as *mut CNode;

        unsafe {
            let tcb = TCB::new(
                1, // tid
                cnode_ptr,
                0x40000000, // vspace_root
                VirtAddr::new(0x10000000), // ipc_buffer
                0x200000, // entry_point
                0x300000, // stack_pointer
            );

            assert_eq!(tcb.tid(), 1);
            assert_eq!(tcb.state(), ThreadState::Inactive);
            assert_eq!(tcb.priority(), TCB::DEFAULT_PRIORITY);
            assert_eq!(tcb.context().elr_el1, 0x200000);
            assert_eq!(tcb.context().sp_el0, 0x300000);
        }
    }

    #[test]
    fn tcb_state_transitions() {
        let mut cnode_memory = [crate::objects::Capability::null(); 16];
        let cnode_ptr = &mut cnode_memory[0] as *mut _ as *mut CNode;

        unsafe {
            let mut tcb = TCB::new(
                1,
                cnode_ptr,
                0x40000000,
                VirtAddr::new(0x10000000),
                0x200000,
                0x300000,
            );

            // Inactive → Runnable
            tcb.activate();
            assert_eq!(tcb.state(), ThreadState::Runnable);

            // Runnable → Running
            tcb.resume();
            assert_eq!(tcb.state(), ThreadState::Running);

            // Running → Runnable
            tcb.suspend();
            assert_eq!(tcb.state(), ThreadState::Runnable);

            // Runnable → Blocked
            tcb.block_on_receive(0x5000);
            assert!(tcb.is_blocked());

            // Blocked → Runnable
            tcb.unblock();
            assert_eq!(tcb.state(), ThreadState::Runnable);
        }
    }

    #[test]
    fn tcb_time_slice() {
        let mut cnode_memory = [crate::objects::Capability::null(); 16];
        let cnode_ptr = &mut cnode_memory[0] as *mut _ as *mut CNode;

        unsafe {
            let mut tcb = TCB::new(
                1,
                cnode_ptr,
                0x40000000,
                VirtAddr::new(0x10000000),
                0x200000,
                0x300000,
            );

            tcb.set_time_slice(3);
            assert_eq!(tcb.time_slice(), 3);

            assert!(!tcb.tick()); // 2 remaining
            assert!(!tcb.tick()); // 1 remaining
            assert!(tcb.tick());  // 0 remaining (exhausted)

            tcb.refill_time_slice();
            assert_eq!(tcb.time_slice(), TCB::DEFAULT_TIME_SLICE);
        }
    }
}
