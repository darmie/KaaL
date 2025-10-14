# Chapter 6: Scheduling & Context Switching - Status

**Status**: 🚧 IN PROGRESS - 0% Complete (0/6 phases)
**Started**: 2025-10-14
**Target Completion**: TBD

## Objectives

1. ⬜ Implement scheduler infrastructure (types, traits, global state)
2. ⬜ Implement round-robin scheduler
3. ⬜ Add priority-based scheduling
4. ⬜ Build context switching mechanism
5. ⬜ Integrate with IPC (yield points for send/recv/call/reply)
6. ⬜ Support timer-based preemption

## Overview

Chapter 6 implements the scheduler that enables multiple threads to share CPU time. This is critical for:
- **IPC blocking**: Threads can yield when waiting for messages
- **Fairness**: All threads get CPU time based on priority
- **Preemption**: High-priority threads can interrupt lower-priority ones
- **Responsiveness**: System remains interactive under load

The scheduler integrates tightly with:
- **TCB** (Thread Control Blocks): Stores thread state and context
- **IPC**: Yield points when blocking on send/recv/call/reply
- **Timer**: Preempts running thread when timeslice expires

## Architecture

```
┌─────────────────────────────────────────────┐
│            Scheduler Core                    │
│  ┌────────────────────────────────────────┐ │
│  │  Ready Queue (per-priority)            │ │
│  │  - Priority 0 (highest): [TCB1, TCB2]  │ │
│  │  - Priority 1:           [TCB3]        │ │
│  │  - Priority 255 (lowest): [TCB4]       │ │
│  └────────────────────────────────────────┘ │
│                                              │
│  Current Thread: *mut TCB                   │
│  Idle Thread: *mut TCB                      │
└─────────────────────────────────────────────┘
         ↓                   ↑
    schedule()          yield_to()
         ↓                   ↑
┌─────────────────────────────────────────────┐
│         Context Switcher                     │
│  - Save current context to TCB               │
│  - Restore next context from TCB             │
│  - Update current thread pointer             │
└─────────────────────────────────────────────┘
         ↓                   ↑
    switch_context()    restore_context()
         ↓                   ↑
┌─────────────────────────────────────────────┐
│         Integration Points                   │
│  - IPC: yield on block                       │
│  - Timer: preempt on tick                    │
│  - Syscall: yield explicit                   │
└─────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Scheduler Infrastructure ⬜ NOT STARTED

Create basic types and global scheduler state.

**Files to Create:**
- `kernel/src/scheduler/mod.rs` - Module root
- `kernel/src/scheduler/types.rs` - Scheduler types

**Key Structures:**

```rust
/// Scheduler - manages runnable threads
pub struct Scheduler {
    /// Ready queues per priority (256 priority levels)
    ready_queues: [ThreadQueue; 256],

    /// Currently running thread
    current: *mut TCB,

    /// Idle thread (runs when nothing else is ready)
    idle: *mut TCB,

    /// Bitmap of non-empty priority levels (fast lookup)
    priority_bitmap: [u64; 4], // 256 bits = 4x u64
}

/// Thread queue (linked list or array)
struct ThreadQueue {
    head: *mut TCB,
    tail: *mut TCB,
    len: usize,
}
```

**Success Criteria:**
- [x] Scheduler struct defined
- [x] Ready queue structure chosen
- [x] Global scheduler instance created
- [x] Basic enqueue/dequeue operations

### Phase 2: Round-Robin Scheduler ⬜ NOT STARTED

Implement simple round-robin scheduling within each priority level.

**Files to Create:**
- `kernel/src/scheduler/round_robin.rs` - Round-robin implementation

**Key Operations:**

```rust
/// Pick next thread to run
pub fn schedule() -> *mut TCB {
    // 1. Find highest non-empty priority level
    // 2. Dequeue head of that priority's queue
    // 3. Return thread to run
}

/// Add thread to ready queue
pub fn enqueue(tcb: *mut TCB) {
    // 1. Get thread's priority
    // 2. Add to tail of that priority's queue
    // 3. Update priority bitmap
}

/// Remove thread from ready queue
pub fn dequeue(tcb: *mut TCB) {
    // 1. Get thread's priority
    // 2. Remove from that priority's queue
    // 3. Update priority bitmap if queue now empty
}

/// Yield current thread
pub fn yield_current() {
    // 1. Save current thread's context
    // 2. Add current to ready queue
    // 3. Pick next thread
    // 4. Switch context
}
```

**Success Criteria:**
- [x] Threads enqueued by priority
- [x] Highest priority always runs first
- [x] Round-robin within priority level
- [x] Yield operation works

### Phase 3: Priority Scheduling ⬜ NOT STARTED

Add priority management and dynamic priority changes.

**Files to Create:**
- `kernel/src/scheduler/priority.rs` - Priority management

**Key Features:**

```rust
/// Set thread priority
pub fn set_priority(tcb: *mut TCB, priority: u8) {
    // 1. Remove from old priority queue
    // 2. Update TCB priority
    // 3. Add to new priority queue
    // 4. Reschedule if higher priority than current
}

/// Get effective priority (for priority inheritance)
pub fn effective_priority(tcb: *mut TCB) -> u8 {
    // For Phase 3: Just return base priority
    // Later: Consider priority inheritance
}
```

**Success Criteria:**
- [x] Can change thread priority
- [x] Priority change triggers reschedule if needed
- [x] 256 priority levels supported
- [x] Priority 0 = highest, 255 = lowest

### Phase 4: Context Switching ⬜ NOT STARTED

Implement low-level context switching in assembly.

**Files to Create:**
- `kernel/src/arch/aarch64/context_switch.rs` - Context switcher
- `kernel/src/arch/aarch64/context_switch.s` - Assembly helpers

**Key Operations:**

```rust
/// Switch from current thread to next thread
///
/// # Safety
/// - Both TCBs must be valid
/// - Must be called with interrupts disabled
pub unsafe fn switch_context(current: *mut TCB, next: *mut TCB) {
    // Assembly implementation:
    // 1. Save current thread's registers to TrapFrame
    // 2. Save SP, ELR, SPSR
    // 3. Restore next thread's registers from TrapFrame
    // 4. Restore SP, ELR, SPSR
    // 5. Return (now executing as next thread)
}
```

**Success Criteria:**
- [x] Saves all general-purpose registers
- [x] Saves/restores stack pointer
- [x] Saves/restores ELR and SPSR
- [x] Works correctly with interrupts

### Phase 5: IPC Integration ⬜ NOT STARTED

Integrate scheduler with IPC operations to enable proper yielding.

**Files to Modify:**
- `kernel/src/ipc/operations.rs` - Add yield points
- `kernel/src/ipc/call.rs` - Add yield points

**Integration Points:**

```rust
// In send():
if !endpoint.has_receivers() {
    // No receiver ready, block sender
    tcb.block_on_send(endpoint_addr);
    scheduler::yield_current(); // ← NEW: Yield to another thread
}

// In recv():
if !endpoint.has_senders() {
    // No sender ready, block receiver
    tcb.block_on_receive(endpoint_addr);
    scheduler::yield_current(); // ← NEW: Yield to another thread
}

// In call():
// After sending, block for reply
tcb.block_on_reply();
scheduler::yield_current(); // ← NEW: Yield to another thread

// In reply():
// After replying, wake the caller
tcb.unblock();
scheduler::enqueue(tcb); // ← NEW: Add caller to ready queue
```

**Success Criteria:**
- [x] send() yields when blocking
- [x] recv() yields when blocking
- [x] call() yields after sending
- [x] reply() wakes caller
- [x] IPC works end-to-end with real yielding

### Phase 6: Timer & Preemption ⬜ NOT STARTED

Add timer-based preemption for fairness.

**Files to Create:**
- `kernel/src/scheduler/timer.rs` - Timer integration

**Key Features:**

```rust
/// Timer interrupt handler
pub fn timer_tick() {
    unsafe {
        // Decrement current thread's timeslice
        let current = scheduler::current_thread();
        (*current).decrement_timeslice();

        // If timeslice expired, reschedule
        if (*current).timeslice() == 0 {
            (*current).reset_timeslice();
            scheduler::yield_current();
        }
    }
}

/// Configure timer for scheduling
pub fn init_scheduler_timer() {
    // Set up periodic timer interrupt
    // Typical: 1ms tick (1000 Hz)
}
```

**Success Criteria:**
- [x] Timer interrupts fire periodically
- [x] Timeslice tracking works
- [x] Preemption occurs when timeslice expires
- [x] Round-robin still works within priority

## Success Criteria

Chapter 6 is complete when:

1. ✅ Scheduler can manage multiple threads
2. ✅ Round-robin scheduling works
3. ✅ Priority-based scheduling works
4. ✅ Context switching preserves all state
5. ✅ IPC operations properly yield
6. ✅ Timer-based preemption works
7. ✅ Tests pass for scheduling scenarios

## Files Structure

```
kernel/src/scheduler/
├── mod.rs              ← Module root, global scheduler
├── types.rs            ← Scheduler types (Scheduler, ThreadQueue)
├── round_robin.rs      ← Round-robin scheduling logic
├── priority.rs         ← Priority management
└── timer.rs            ← Timer integration

kernel/src/arch/aarch64/
├── context_switch.rs   ← Context switching (Rust)
└── context_switch.s    ← Context switching (ASM)
```

## References

### seL4 Documentation

- [seL4 Scheduler](https://docs.sel4.systems/projects/sel4/api-doc.html#scheduling)
- [seL4 Scheduling Context](https://docs.sel4.systems/projects/sel4/mcs.html)

### ARM64 Context Switching

- ARM Architecture Reference Manual - Exception entry/return
- Cortex-A Series Programmer's Guide - Context switching

## Key Design Decisions

### 1. Scheduler Algorithm

Using **fixed-priority preemptive scheduling** with **round-robin within priority**:
- 256 priority levels (0 = highest)
- O(1) scheduling (bitmap lookup)
- Deterministic behavior

### 2. Context Switching

Using **trap frame** approach:
- All context stored in TCB's TrapFrame
- Same structure for exceptions and context switches
- Simplified implementation

### 3. Ready Queue

Using **array of linked lists**:
- One queue per priority level
- Fast enqueue/dequeue
- Priority bitmap for O(1) highest-priority lookup

### 4. Integration with IPC

**Explicit yield points**:
- IPC operations call `yield_current()` when blocking
- No automatic yielding (deterministic)
- Clear control flow

## Known Limitations

1. **No Priority Inheritance** (Phase 6 only):
   - Basic priority inversion possible
   - TODO: Add priority inheritance protocol (Chapter 7)

2. **Single-Core Only**:
   - No SMP support yet
   - TODO: Add per-CPU schedulers (Chapter 8)

3. **No Real-Time Guarantees**:
   - Fixed-priority is deterministic but not hard real-time
   - TODO: Add scheduling contexts for temporal isolation (Chapter 8)

## Progress Tracking

### Completed ✅

- None yet (just started Chapter 6)

### In Progress 🚧

- Phase 1: Scheduler infrastructure

### Blocked ⛔

- None

## Next Steps

1. Create `kernel/src/scheduler/` directory
2. Implement basic Scheduler struct in `types.rs`
3. Add global scheduler instance in `mod.rs`
4. Implement enqueue/dequeue operations
5. Continue with round-robin scheduling

---

**Last Updated**: 2025-10-14
**Status**: 🚧 STARTING - Phase 1 beginning
