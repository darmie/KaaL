# Chapter 9 Phase 5: Inter-Component IPC Testing - Plan

**Status**: 🚧 Planning
**Date**: 2025-10-16

---

## Overview

Phase 5 focuses on testing **real inter-process communication** between spawned components using shared memory and notifications. This validates the complete microkernel IPC infrastructure.

## Prerequisites (All Complete ✅)

- ✅ Component loading infrastructure (Phase 4)
- ✅ Notification kernel objects and syscalls (Phase 2)
- ✅ SharedRing library (Phase 2)
- ✅ KaaL SDK with component patterns (Phase 3)
- ✅ Cooperative multitasking working

## Technical Challenges

### Challenge 1: Capability Passing to Spawned Components

**Problem**: Components currently spawn with empty CSpace (only null capabilities initialized by CNode::new).

**Options**:

**Option A: Pass Capability Slot Numbers via Arguments** ⭐ RECOMMENDED
- Pros: Simple, no kernel changes needed
- Cons: Components must trust slot numbers are correct
- Implementation: Pass cap slots as entry point arguments or environment
- Estimated effort: 1 day

**Option B: Pre-insert Capabilities During sys_process_create**
- Pros: Clean API, capabilities ready on spawn
- Cons: Requires kernel API changes, more complex
- Implementation: Add capability list parameter to sys_process_create
- Estimated effort: 2-3 days

**Option C: Implement Capability Transfer Syscalls**
- Pros: Most flexible, proper capability-based security
- Cons: Significant kernel work, testing complexity
- Implementation: sys_cap_copy, sys_cap_move, sys_cap_grant
- Estimated effort: 1 week

**Decision**: Start with **Option A** for MVP, plan Option C for production.

### Challenge 2: Shared Memory Mapping Across Components

**Problem**: sys_memory_map currently maps only into calling process's address space.

**Current API**:
```rust
unsafe fn sys_memory_map(phys_addr: usize, size: usize, permissions: usize) -> usize
```

**Needed**:
```rust
unsafe fn sys_memory_map_into(
    phys_addr: usize,
    size: usize,
    permissions: usize,
    target_pid: usize  // NEW: Target process to map into
) -> usize
```

**Implementation**:
1. Add new syscall number `SYS_MEMORY_MAP_INTO = 0x1B`
2. Kernel validates target PID and permissions
3. Get target process's TTBR0 from TCB
4. Perform mapping in target's page table
5. Return virtual address in target's address space

**Estimated effort**: 1-2 days

### Challenge 3: Notification Capability Sharing

**Problem**: Notification created in root-task CSpace, need to share with spawned components.

**Current Flow**:
1. root-task calls sys_notification_create() → cap slot 102 in root-task
2. Need same notification accessible in component A and component B

**Solution with Option A (Pass Slot Numbers)**:
1. root-task creates notification → cap slot N in root-task CSpace
2. root-task copies capability to components' CSpaces during spawn
3. Pass slot number to component via arguments
4. Component uses same slot number to signal/wait

**Implementation**:
1. Add CSpace manipulation during component spawn
2. Insert capabilities into spawned component's CNode
3. Pass slot numbers via entry point or environment

**Estimated effort**: 1-2 days

---

## Implementation Plan

### Step 1: Create IPC Test Components (1 day)

**New Components**:
1. `components/ipc-producer/` - Produces data to SharedRing
2. `components/ipc-consumer/` - Consumes data from SharedRing

**Producer Logic**:
```rust
// Receives arguments: shared_mem_slot, consumer_notify_slot, producer_notify_slot
fn main(shared_mem: usize, consumer_cap: usize, producer_cap: usize) {
    // Map shared memory
    let shared_ptr = sys_memory_map(shared_mem, 4096, RW);

    // Initialize SharedRing
    let ring = unsafe { SharedRing::new(shared_ptr, consumer_cap, producer_cap) };

    // Produce data
    for i in 0..10 {
        ring.push(i);
        syscall::signal(consumer_cap, 0x1);  // Signal data available
        syscall::wait(producer_cap);         // Wait for space available
    }
}
```

**Consumer Logic**:
```rust
fn main(shared_mem: usize, consumer_cap: usize, producer_cap: usize) {
    let shared_ptr = sys_memory_map(shared_mem, 4096, RW);
    let ring = unsafe { SharedRing::new(shared_ptr, consumer_cap, producer_cap) };

    // Consume data
    loop {
        syscall::wait(consumer_cap);       // Wait for data available
        if let Some(val) = ring.pop() {
            syscall::print(val);
            syscall::signal(producer_cap, 0x2);  // Signal space available
        }
    }
}
```

### Step 2: Implement sys_memory_map_into (1-2 days)

**Kernel Changes**:

File: `kernel/src/syscall/numbers.rs`
```rust
pub const SYS_MEMORY_MAP_INTO: u64 = 0x1B;
```

File: `kernel/src/syscall/mod.rs`
```rust
fn sys_memory_map_into(
    tf: &TrapFrame,
    phys_addr: u64,
    size: u64,
    permissions: u64,
    target_pid: u64
) -> u64 {
    // Validate target PID
    let target_tcb = scheduler::find_thread(target_pid as usize);
    if target_tcb.is_null() {
        return u64::MAX;
    }

    // Get target's page table
    let target_ttbr0 = unsafe { (*target_tcb).page_table_root() };

    // Allocate virtual address in target's space
    let virt_addr = allocate_virt_addr(size);

    // Map into target's page table
    map_into_page_table(target_ttbr0, virt_addr, phys_addr, size, permissions);

    virt_addr as u64
}
```

### Step 3: Implement Capability Insertion During Spawn (1-2 days)

**Root-Task Changes**:

File: `runtime/root-task/src/component_loader.rs`
```rust
pub struct SpawnConfig {
    pub component_name: &'static str,
    pub capabilities: &'static [(usize, Capability)],  // (slot, cap) pairs
    pub arguments: &'static [usize],  // Entry point args
}

impl ComponentLoader {
    unsafe fn spawn_with_caps(&self, config: SpawnConfig) -> Result<usize> {
        // ... existing spawn logic ...

        // Insert capabilities into spawned component's CSpace
        for (slot, cap_type) in config.capabilities {
            // Copy capability from root-task CSpace to component CSpace
            sys_capability_copy(
                own_cspace, *slot,      // Source: root-task CSpace
                cspace_root, *slot      // Dest: component CSpace
            );
        }

        // Pass arguments via registers or stack
        // ...
    }
}
```

### Step 4: Orchestrate Multi-Component IPC Test (1 day)

**Root-Task Test Code**:
```rust
unsafe fn test_inter_component_ipc() {
    // Step 1: Allocate shared memory
    let shared_mem = sys_memory_allocate(4096);

    // Step 2: Create notifications
    let consumer_notify = sys_notification_create();  // e.g., slot 102
    let producer_notify = sys_notification_create();  // e.g., slot 103

    // Step 3: Spawn producer with capabilities
    let producer_config = SpawnConfig {
        component_name: "ipc_producer",
        capabilities: &[
            (102, Capability::notification(consumer_notify)),
            (103, Capability::notification(producer_notify)),
        ],
        arguments: &[shared_mem, 102, 103],
    };
    let producer_pid = loader.spawn_with_caps(producer_config)?;

    // Step 4: Map shared memory into producer
    sys_memory_map_into(shared_mem, 4096, RW, producer_pid);

    // Step 5: Spawn consumer with same capabilities
    let consumer_config = SpawnConfig {
        component_name: "ipc_consumer",
        capabilities: &[
            (102, Capability::notification(consumer_notify)),
            (103, Capability::notification(producer_notify)),
        ],
        arguments: &[shared_mem, 102, 103],
    };
    let consumer_pid = loader.spawn_with_caps(consumer_config)?;

    // Step 6: Map shared memory into consumer
    sys_memory_map_into(shared_mem, 4096, RW, consumer_pid);

    // Step 7: Let them communicate
    sys_yield();

    sys_print("✓ Inter-component IPC test complete!\n");
}
```

---

## Testing Strategy

### Test 1: Basic Signaling (No Shared Memory)
- Spawn two components
- Component A signals notification
- Component B waits on notification
- Verify B wakes up when A signals

### Test 2: Shared Memory + Notifications
- Spawn producer and consumer
- Producer writes to shared memory, signals
- Consumer waits, reads from shared memory
- Verify data integrity

### Test 3: SharedRing IPC
- Full producer/consumer with SharedRing
- Producer pushes 100 items
- Consumer pops 100 items
- Verify all items received correctly

### Test 4: Capability Transfer (Future)
- Test sys_cap_copy, sys_cap_move, sys_cap_grant
- Verify capability rights propagation
- Test revocation

---

## Success Criteria

- [ ] Two components spawn simultaneously
- [ ] Shared memory accessible from both components
- [ ] Notifications work across component boundaries
- [ ] SharedRing communication functional
- [ ] Data integrity verified (no corruption)
- [ ] Performance: IPC latency < 500 cycles

---

## Timeline

| Task | Duration | Status |
|------|----------|--------|
| Create IPC test components | 1 day | ⬜ Planned |
| Implement sys_memory_map_into | 1-2 days | ⬜ Planned |
| Capability insertion during spawn | 1-2 days | ⬜ Planned |
| Multi-component IPC orchestration | 1 day | ⬜ Planned |
| Testing and debugging | 1-2 days | ⬜ Planned |
| **Total** | **5-8 days** | |

---

## Next Steps

1. **Immediate**: Create `components/ipc-producer/` and `components/ipc-consumer/`
2. **Day 2-3**: Implement sys_memory_map_into syscall
3. **Day 4-5**: Capability insertion during spawn
4. **Day 6**: Full integration test
5. **Day 7-8**: Debugging and performance tuning

---

**Last Updated**: 2025-10-16
**Ready to Start**: Yes ✅
