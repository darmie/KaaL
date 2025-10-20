# Chapter 4: Kernel Object Model - Status

**Status**: ✅ COMPLETE - 100% (7/7 phases)
**Started**: 2025-10-12
**Completed**: 2025-10-13

## Objectives

1. ✅ Implement capability representation
2. ✅ Create core kernel objects (TCB, CNode, Endpoint)
3. ✅ Implement object invocations
4. ✅ Build capability derivation system
5. ✅ Add capability rights enforcement

## Overview

Chapter 4 implements the capability-based object model that forms the foundation of KaaL's security architecture. This follows the seL4 design where all kernel resources are accessed through capabilities, providing:

- **Fine-grained access control**: Capabilities carry specific rights (Read, Write, Grant)
- **Unforgeable references**: Capabilities cannot be forged, only derived
- **Delegation**: Capabilities can be transferred between protection domains
- **Revocation**: Access can be revoked by removing capabilities

## Capability-Based Security Model

### Core Concepts

```
Capability = Unforgeable token granting specific rights to a kernel object

Key Properties:
1. Capabilities stored in CNodes (capability nodes)
2. Each capability has a type (TCB, Endpoint, Page, etc.)
3. Each capability has rights (Read, Write, Grant, etc.)
4. Capabilities can be derived with reduced rights
5. User space cannot forge capabilities
```

### Object Hierarchy

```
Kernel Objects:
├── Untyped Memory      ← Raw memory that can be retyped
├── CNode               ← Container for capabilities
├── TCB                 ← Thread control block
├── Endpoint            ← Synchronous IPC
├── Notification        ← Asynchronous signals
├── VSpace              ← Virtual address space root
├── Page Table          ← Page table levels
└── Page                ← Physical memory page
```

## Implementation Plan

### Phase 1: Capability Representation ⬜ NOT STARTED

Define the basic capability structure and types.

**Files to Create:**

- `kernel/src/objects/mod.rs` - Object module root
- `kernel/src/objects/capability.rs` - Capability struct and types

**Key Structures:**

```rust
/// Capability - unforgeable token granting rights to a kernel object
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Capability {
    /// Type of the object this capability refers to
    cap_type: CapType,

    /// Pointer to the kernel object
    object_ptr: usize,

    /// Access rights for this capability
    rights: CapRights,

    /// Guard word for CNode lookup
    guard: u64,

    /// Badge for endpoint identification
    badge: u64,
}

/// Types of kernel objects
#[repr(u8)]
pub enum CapType {
    Null = 0,
    UntypedMemory = 1,
    Endpoint = 2,
    Notification = 3,
    Tcb = 4,
    CNode = 5,
    VSpace = 6,
    PageTable = 7,
    Page = 8,
}

/// Capability rights (bitflags)
bitflags! {
    pub struct CapRights: u8 {
        const READ  = 0b0001;
        const WRITE = 0b0010;
        const GRANT = 0b0100;
        const ALL   = Self::READ.bits() | Self::WRITE.bits() | Self::GRANT.bits();
    }
}
```

**Success Criteria:**

- [x] Capability struct defined with proper layout
- [x] CapType enum covers all object types
- [x] CapRights uses bitflags for efficient checks
- [x] Capability size is 32 bytes (cache-friendly)

### Phase 2: CNode Implementation ⬜ NOT STARTED

Implement capability nodes for storing capabilities.

**Files to Create:**

- `kernel/src/objects/cnode.rs` - CNode implementation

**Key Features:**

```rust
/// CNode - a container for capabilities
///
/// CNodes are arrays of capability slots. They form a tree structure
/// for capability address space.
pub struct CNode {
    /// Number of slots (must be power of 2)
    size_bits: u8,

    /// Array of capability slots
    slots: *mut Capability,
}

impl CNode {
    /// Look up a capability by index
    pub fn lookup(&self, index: usize) -> Option<&Capability>;

    /// Insert a capability at an index
    pub fn insert(&mut self, index: usize, cap: Capability) -> Result<(), Error>;

    /// Remove a capability
    pub fn delete(&mut self, index: usize);

    /// Move a capability between CNodes
    pub fn move_cap(&mut self, src: usize, dest_cnode: &mut CNode, dest: usize) -> Result<(), Error>;
}
```

**Success Criteria:**

- [x] CNode can store N capabilities (power of 2)
- [x] Capability lookup is O(1)
- [x] Can insert/delete/move capabilities
- [x] Proper bounds checking

### Phase 3: TCB Implementation ⬜ NOT STARTED

Implement Thread Control Blocks.

**Files to Create:**

- `kernel/src/objects/tcb.rs` - TCB implementation

**Key Features:**

```rust
/// Thread Control Block - represents a thread of execution
pub struct TCB {
    /// CPU context (trap frame)
    context: TrapFrame,

    /// CSpace root (capability space)
    cspace_root: *mut CNode,

    /// VSpace root (virtual address space)
    vspace_root: usize,

    /// IPC buffer location
    ipc_buffer: usize,

    /// Thread state
    state: ThreadState,

    /// Priority and scheduling info
    priority: u8,
    time_slice: u32,
}

pub enum ThreadState {
    Inactive,
    Running,
    BlockedOnReceive(*mut Endpoint),
    BlockedOnSend(*mut Endpoint),
    BlockedOnReply,
}
```

**Success Criteria:**

- [x] TCB stores complete thread context
- [x] TCB links to CSpace and VSpace
- [x] Thread states properly modeled
- [x] Can switch between threads

### Phase 4: Endpoint Implementation ⬜ NOT STARTED

Implement IPC endpoints (basic structure only, full IPC in Chapter 5).

**Files to Create:**

- `kernel/src/objects/endpoint.rs` - Endpoint implementation

**Key Features:**

```rust
/// Endpoint - rendezvous point for IPC
pub struct Endpoint {
    /// Queue of threads waiting to send
    send_queue: ThreadQueue,

    /// Queue of threads waiting to receive
    recv_queue: ThreadQueue,
}

impl Endpoint {
    /// Create a new endpoint
    pub fn new() -> Self;

    /// Queue a thread for send
    pub fn queue_send(&mut self, tcb: *mut TCB);

    /// Queue a thread for receive
    pub fn queue_receive(&mut self, tcb: *mut TCB);

    /// Try to match sender and receiver
    pub fn try_match(&mut self) -> Option<(*mut TCB, *mut TCB)>;
}
```

**Success Criteria:**

- [x] Endpoint can queue threads
- [x] Basic structure ready for IPC (Chapter 5)
- [x] Thread queues implemented

### Phase 5: Untyped Memory ⬜ NOT STARTED

Implement untyped memory objects for retyping.

**Files to Create:**

- `kernel/src/objects/untyped.rs` - Untyped memory

**Key Features:**

```rust
/// Untyped Memory - raw memory that can be retyped into other objects
pub struct UntypedMemory {
    /// Physical address of the memory region
    paddr: PhysAddr,

    /// Size in bits (2^size_bits bytes)
    size_bits: u8,

    /// Watermark for allocations
    watermark: usize,

    /// Children derived from this untyped
    children: alloc::vec::Vec<usize>,
}

impl UntypedMemory {
    /// Retype untyped memory into a specific object type
    pub fn retype(&mut self, obj_type: CapType, size_bits: u8) -> Result<usize, Error>;

    /// Revoke all children (reclaim memory)
    pub fn revoke(&mut self) -> Result<(), Error>;
}
```

**Success Criteria:**

- [x] Can retype untyped into other objects
- [x] Watermark tracking prevents overlaps
- [x] Revocation reclaims memory

### Phase 6: Object Invocations ⬜ NOT STARTED

Implement the syscall interface for object operations.

**Files to Create:**

- `kernel/src/objects/invoke.rs` - Object invocation handlers

**Key Features:**

```rust
/// Invoke an operation on a capability
pub fn invoke_capability(cap: &Capability, args: &[u64]) -> Result<u64, Error> {
    match cap.cap_type {
        CapType::Tcb => invoke_tcb(cap, args),
        CapType::CNode => invoke_cnode(cap, args),
        CapType::Endpoint => invoke_endpoint(cap, args),
        CapType::UntypedMemory => invoke_untyped(cap, args),
        _ => Err(Error::InvalidCapability),
    }
}
```

**Success Criteria:**

- [x] Each object type has invocation handlers
- [x] Rights are checked before operations
- [x] Proper error handling

### Phase 7: Testing ⬜ NOT STARTED

Create tests for all object types and operations.

**Tests to Create:**

1. Capability creation and manipulation
2. CNode lookup and insertion
3. TCB context switching (basic)
4. Endpoint queuing
5. Untyped retyping
6. Rights enforcement

## Success Criteria

Chapter 4 is complete when:

1. ✅ All object types implemented
2. ✅ Capabilities can be created and manipulated
3. ✅ CNodes can store and look up capabilities
4. ✅ TCBs can represent threads
5. ✅ Endpoints can queue threads for IPC
6. ✅ Untyped memory can be retyped
7. ✅ Object invocations work through syscalls
8. ✅ Tests pass for all object operations

## Files Structure

```
kernel/src/objects/
├── mod.rs              ← Module root, re-exports
├── capability.rs       ← Capability type and rights
├── cnode.rs            ← Capability nodes
├── tcb.rs              ← Thread control blocks
├── endpoint.rs         ← IPC endpoints
├── notification.rs     ← Async notifications (optional)
├── untyped.rs          ← Untyped memory
├── vspace.rs           ← Virtual space (optional, later)
└── invoke.rs           ← Object invocation handlers
```

## References

### seL4 Documentation

- [seL4 Whitepaper](https://sel4.systems/About/seL4-whitepaper.pdf) - Object model design
- [seL4 Manual](https://sel4.systems/Info/Docs/seL4-manual-latest.pdf) - Object types and operations
- [Capability-based Systems](https://en.wikipedia.org/wiki/Capability-based_security)

### Implementation References

- seL4 kernel source: `libsel4/include/sel4/types.h` - Object type definitions
- seL4 kernel source: `kernel/include/object/structures.h` - Object structures
- seL4 kernel source: `kernel/src/object/` - Object implementations

## Test Results

### Test Suite Status: 18/18 PASS ✅ (100% SUCCESS RATE!)

All kernel object model tests passing successfully after identifying and fixing the FP/SIMD configuration issue.

#### All Tests Passing (18/18) ✅

**Capability Tests (4)**
1. test_capability_creation
2. test_capability_derivation
3. test_capability_minting
4. test_capability_rights

**CNode Tests (3)**
5. test_cnode_creation
6. test_cnode_insert_lookup
7. test_cnode_copy_move

**TCB Tests (3)**
8. test_tcb_creation
9. test_tcb_state_transitions
10. test_tcb_priority

**Endpoint Tests (2)**
11. test_endpoint_creation
12. test_endpoint_queue_operations ✅

**Untyped Memory Tests (3)**
13. test_untyped_creation
14. test_untyped_retype ✅
15. test_untyped_revoke ✅

**Invocation Tests (3)**
16. test_tcb_invocation_priority ✅
17. test_invocation_rights_enforcement
18. test_capability_delegation_chain ✅

### Root Cause: FP/SIMD Was Disabled

**Problem**: Tests 16-18 hung silently even after removing heap allocator and increasing stack to 256KB.

**Investigation**: Exception handlers revealed Exception Class 0x07 = "Trapped FP/SIMD operations"

**Root Cause**: CPACR_EL1 wasn't configured to allow FP/SIMD access. The compiler was generating SIMD instructions for optimized memory operations (array initialization, memcpy), but these trapped because FP/SIMD was disabled.

**Solution**: Enable FP/SIMD at boot by setting CPACR_EL1.FPEN = 0b11 in assembly:

```asm
mrs x10, cpacr_el1
orr x10, x10, #(0x3 << 20)  // FPEN = 0b11 (no trapping)
msr cpacr_el1, x10
isb
```

Applied to:
1. Main kernel ([kernel/src/main.rs:22-25](../../kernel/src/main.rs))
2. Test harness ([examples/kernel-test/src/main.rs](../../examples/kernel-test/src/main.rs))

**Result**: ALL 18/18 TESTS PASS!

### Architectural Fix: Eliminated Vec Usage (seL4 Design Principle)

**Research Finding**: seL4 kernels do NOT use dynamic heap allocation after boot. Everything is statically allocated or provided by userspace.

**Problem**: The `linked_list_allocator::LockedHeap` with spinlocks caused deadlocks when Vec was used in bare-metal environment.

**Solution**: Replaced all Vec usage with fixed-size arrays:

1. **Endpoint ThreadQueue** ([endpoint.rs:32-36](../../kernel/src/objects/endpoint.rs))
   - Before: `Vec<*mut TCB>`
   - After: `[*mut TCB; MAX_QUEUE_SIZE]` with count tracking
   - Constant: `MAX_QUEUE_SIZE = 256`

2. **UntypedMemory Children** ([untyped.rs:83](../../kernel/src/objects/untyped.rs))
   - Before: `Vec<PhysAddr>`
   - After: `[PhysAddr; MAX_CHILDREN]` with child_count
   - Constant: `MAX_CHILDREN = 128`

3. **UntypedMemory::split()** ([untyped.rs:324](../../kernel/src/objects/untyped.rs))
   - Before: `Result<Vec<UntypedMemory>, CapError>`
   - After: `Result<usize, CapError>` with out-parameter `&mut [UntypedMemory; MAX_SPLITS]`
   - Constant: `MAX_SPLITS = 64`

**Impact**:
- Eliminated all heap allocation from core kernel object operations
- Aligned with seL4's static allocation architecture
- Resolved spinlock deadlock issues
- Made tests deterministic and reliable

### Debugging Journey

1. ❌ Initial attempt: Reinitialize heap → No improvement
2. ❌ Removed Vec usage → Helped but tests 16-18 still hung
3. ❌ Increased stack to 256KB → Still failing
4. ✅ **Installed exception handlers** (critical insight!) → Got Exception Class 0x07
5. ✅ Decoded EC 0x07 → FP/SIMD trapped
6. ✅ Enabled CPACR_EL1.FPEN → **ALL TESTS PASS!**

**Key Insight**: In no_std bare-metal, exceptions don't trigger panics by default. Without proper exception handlers, FP/SIMD faults appeared as silent hangs. Installing exception handlers provided ESR/FAR register values that revealed the true cause.

## Progress Tracking

### Completed ✅

- All 7 phases of Chapter 4 complete
- Testable test suite integrated into kernel-test harness
- Core object model functionality verified

### In Progress 🚧

- Investigating heap allocation issues with Vec-heavy tests

### Blocked ⛔

- None (test issues are post-implementation validation)

## Key Design Decisions

### 1. Capability Representation

Following seL4's design:

- 32-byte capability structure (cache line friendly)
- Type field for fast dispatch
- Rights as bitflags for efficient checks
- Guard and badge for CNode addressing and endpoint identification

### 2. Object Allocation

Initially using simple bump allocator:

- Objects allocated from untyped memory
- Watermark tracks used space
- Later: implement proper object allocator

### 3. Capability Addressing

Using CNode tree structure:

- Each CNode is 2^n slots
- Capability address is a path through CNode tree
- Guard bits for compressed addressing

## Next Steps

1. Create `kernel/src/objects/` directory structure
2. Implement `Capability` struct in [capability.rs](../../kernel/src/objects/capability.rs)
3. Implement `CNode` in [cnode.rs](../../kernel/src/objects/cnode.rs)
4. Continue with TCB, Endpoint, and Untyped

---

**Last Updated**: 2025-10-13
**Status**: 🚧 IN PROGRESS - Starting Chapter 4 implementation
