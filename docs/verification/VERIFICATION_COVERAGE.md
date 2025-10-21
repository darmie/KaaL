# Verification Coverage

**Last Updated**: 2025-10-21
**Status**: 20 modules, 358 verified items, 0 errors

---

## Summary

The KaaL microkernel has **17 verified modules** covering core memory management, scheduling, capability operations, and IPC primitives. All modules verify successfully with Verus.

### Verification Statistics

| Category | Modules | Verified Items | Status |
|----------|---------|----------------|--------|
| **Memory Management** | 9 | 139 | ✅ Complete |
| **Capability System** | 5 | 96 | ✅ Complete |
| **Scheduling & IPC** | 4 | 77 | ✅ Complete |
| **System Invocations** | 1 | 40 | ✅ Complete |
| **Page Tables** | 1 | 6 | ✅ Complete |
| **TOTAL** | **20** | **358** | **✅ All Pass** |

---

## Verified Modules

### 1. Memory Management (139 items)

#### Physical/Virtual Addresses
- **[phys_addr.rs](../../kernel/src/verified/phys_addr.rs)** - 10 items
  - Physical address operations: new, as_usize, alignment, page numbers
  - Verification: bounds checking, alignment properties

- **[virt_addr.rs](../../kernel/src/verified/virt_addr.rs)** - 10 items
  - Virtual address operations: new, as_usize, alignment, page numbers
  - Verification: 48-bit canonical addressing, alignment

- **[page_frame_number.rs](../../kernel/src/verified/page_frame_number.rs)** - 5 items
  - Page frame number (PFN) operations
  - Verification: PFN ↔ physical address conversions

#### Frame Allocation
- **[bitmap_simple.rs](../../kernel/src/verified/bitmap_simple.rs)** - 3 items
  - Simple bitmap operations: set, is_set, find_first_unset
  - Educational example for learning Verus

- **[bitmap_prod.rs](../../kernel/src/verified/bitmap_prod.rs)** - 12 items
  - Production bitmap with advanced features
  - Verification: frame conditions with `old()`, loop invariants

- **[frame_allocator_ops.rs](../../kernel/src/verified/frame_allocator_ops.rs)** - 15 items
  - Frame allocator operations: alloc, dealloc, add/reserve regions
  - Verification: no double allocation, conservation of frames

- **[untyped_ops.rs](../../kernel/src/verified/untyped_ops.rs)** - 11 items
  - Untyped memory watermark allocator
  - Verification: allocate, revoke, child tracking, monotonic watermark

#### Virtual Memory
- **[vspace_ops.rs](../../kernel/src/verified/vspace_ops.rs)** - 19 items
  - VSpace operations: page table walking (L0-L3)
  - Map/unmap pages: 4KB, 2MB, 1GB
  - Verification: alignment checks, valid level operations

- **[pte_ops.rs](../../kernel/src/verified/pte_ops.rs)** - 41 items
  - Page table entry (PTE) operations for ARMv8-A
  - Descriptor types: valid, table, block, page
  - Address extraction and setting
  - Permission bits: PXN, UXN, AF (Access Flag)
  - Verification: frame conditions, bit operation axioms

#### TLB Management
- **[tlb_ops.rs](../../kernel/src/verified/tlb_ops.rs)** - 23 items
  - TLB invalidation by VA, ASID, or all entries
  - ASID allocation and validation (8-bit, 0-255)
  - Context switch TLB handling
  - Virtual address bounds checking (48-bit)
  - Page-aligned address validation
  - TLB barriers for ordering guarantees
  - Verification: ASID bounds, VA validity, operation safety axioms

#### Page Table Operations
- **[page_table_ops.rs](../../kernel/src/verified/page_table_ops.rs)** - 7 items
  - Page table level operations: shift, block_size, index
  - Verification: level bounds, block support at L1/L2

---

### 2. Capability System (96 items)

#### Rights Management
- **[cap_rights.rs](../../kernel/src/verified/cap_rights.rs)** - 4 items
  - Capability rights bit operations
  - Constants: READ, WRITE, GRANT, ALL
  - Verification: contains, get_bits

#### Capability Operations
- **[capability_ops.rs](../../kernel/src/verified/capability_ops.rs)** - 10 items
  - Capability derivation and rights checking
  - Union/intersection operations for rights
  - Verification: rights monotonicity

#### CNode Operations
- **[cnode_ops.rs](../../kernel/src/verified/cnode_ops.rs)** - 6 items
  - CNode slot operations: num_slots, is_valid_index
  - Size validation (power of 2)
  - Verification: slot bounds checking

#### Thread Control Block
- **[tcb.rs](../../kernel/src/verified/tcb.rs)** - 29 items
  - TCB state machine: Inactive, Ready, Running, Blocked
  - Capability checking for TCB operations
  - Time slice management
  - Verification: state transition properties

#### Capability Transfer
- **[cap_transfer_ops.rs](../../kernel/src/verified/cap_transfer_ops.rs)** - 23 items
  - Capability transfer with rights diminishing
  - Badge assignment and minting
  - GRANT right validation (prevents unauthorized duplication)
  - Copy vs transfer vs mint vs mutate operations
  - Verification: rights subset preservation, CSpace isolation, badge correctness

#### System Invocations
- **[invocation_ops.rs](../../kernel/src/verified/invocation_ops.rs)** - 53 items
  - Invocation argument validation
  - Rights checking for TCB/CNode/Endpoint operations
  - Label parsing for system calls
  - Verification: argument bounds, rights requirements

---

### 3. Scheduling & IPC (77 items)

#### Thread Queues
- **[thread_queue_ops.rs](../../kernel/src/verified/thread_queue_ops.rs)** - 19 items
  - ThreadQueue and Endpoint operations
  - Enqueue, dequeue operations
  - Verification: queue state consistency, FIFO properties

#### Scheduler
- **[scheduler_ops.rs](../../kernel/src/verified/scheduler_ops.rs)** - 21 items
  - Priority-based scheduler operations
  - Priority bitmap for O(1) priority lookup
  - `leading_zeros` optimization
  - Verification: priority bounds, bitmap consistency

#### IPC Message Transfer
- **[ipc_message_ops.rs](../../kernel/src/verified/ipc_message_ops.rs)** - 37 items
  - Message info encoding/decoding (label, length, caps fields)
  - Message register operations (MR0-MR7)
  - Message buffer copying with frame conditions
  - Bounds checking for message lengths (max 120 words)
  - Badge extraction from capabilities
  - Verification: bijective encoding/decoding, register preservation, buffer bounds

---

## Verification Properties

### Memory Safety Properties
1. ✅ **No out-of-bounds access**: All array/bitmap operations bounds-checked
2. ✅ **No double allocation**: Frame allocator prevents double allocation
3. ✅ **Alignment correctness**: Address operations maintain alignment
4. ✅ **Conservation**: Allocated + free frames = total frames

### Functional Correctness Properties
1. ✅ **Address conversions**: PFN ↔ PhysAddr bijection
2. ✅ **Bitmap operations**: Set/unset operations preserve other bits
3. ✅ **State machine validity**: TCB state transitions follow seL4 model
4. ✅ **Capability rights**: Derivation never increases rights

### Security Properties
1. ✅ **Rights checking**: Operations require appropriate rights
2. ✅ **Slot bounds**: CNode operations stay within valid slots
3. ✅ **Priority isolation**: Scheduler respects priority boundaries

---

## Verification Techniques Used

### 1. Preconditions and Postconditions
```rust
pub fn phys_addr(&self) -> (result: u64)
    requires self.spec_is_valid(),
    ensures
        result == self.spec_phys_addr(),
        result < (1u64 << 36),
{
    (self.bits & ADDR_MASK) >> ADDR_SHIFT
}
```

### 2. Frame Conditions with `old()`
```rust
pub fn set_accessed(&mut self)
    ensures
        self.spec_is_accessed(),
        // Frame condition: other bits unchanged
        (old(self).bits & !(1u64 << AF_BIT)) == (self.bits & !(1u64 << AF_BIT)),
{
    self.bits = self.bits | (1u64 << AF_BIT);
}
```

### 3. Loop Invariants
```rust
while idx < self.bits.len()
    invariant
        idx <= self.bits.len(),
        forall|i: int| 0 <= i < idx ==> self.bits[i] == u64::MAX,
{
    // Loop body
}
```

### 4. State Machine Verification
```rust
pub enum ThreadState {
    Inactive,
    Ready,
    Running,
    BlockedOnReceive,
    BlockedOnSend,
    BlockedOnReply,
}

pub closed spec fn spec_can_transition(old_state: ThreadState, new_state: ThreadState) -> bool {
    // Valid transitions proven
}
```

### 5. Bit Operation Axioms
```rust
proof fn axiom_or_sets_bit(val: u64, bit_pos: u64)
    requires bit_pos < 64,
    ensures (val | (1u64 << bit_pos)) & (1u64 << bit_pos) != 0,
{
    admit()  // Trusted bit operation
}
```

---

## Running Verification

### Prerequisites
- Verus 0.2025.10.17 or later
- Z3 v4.15.3 (bundled with Verus binary release)

### Verify All Modules
```bash
nu scripts/verify.nu
```

### Verify Single Module
```bash
~/verus/verus --crate-type=lib kernel/src/verified/pte_ops.rs
```

### Expected Output
```
🔍 Running Verus verification...

📦 Verifying (pte_ops)...
  ✅ pte_ops: 41 verified, 0 errors
     Page table entry operations: descriptor types, address extraction, permission bits

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ All verification passed: 275 items verified, 0 errors
```

---

## Verification Roadmap

### Completed (Phase 1-2)
- ✅ Basic data structures (addresses, PFNs, bitmaps)
- ✅ Frame allocator with no-double-allocation proof
- ✅ Capability system (rights, derivation)
- ✅ TCB state machine
- ✅ Page table operations (levels, walking, PTE operations)
- ✅ Scheduler with priority bitmap
- ✅ Thread queues with FIFO properties
- ✅ System invocation validation

### In Progress (Phase 3)
- 🚧 IPC message transfer verification
- 🚧 Capability transfer during IPC
- 🚧 TLB operations verification

### Planned (Phase 4-5)
- 📋 Exception handling verification
- 📋 IRQ handling verification
- 📋 System initialization verification
- 📋 End-to-end properties (isolation, security)

---

## Lessons Learned

### What Works Well
1. **Modular extraction**: Separating algorithm logic from integration code
2. **Incremental properties**: Start with safety, build to correctness
3. **Feature flags**: `#[cfg(feature = "verification")]` for optional verification
4. **Axioms for bit operations**: Trust hardware semantics

### Verus Capabilities
✅ **Strong for**:
- Functional specifications (pre/post conditions)
- State machines with enum verification
- Array/bitmap operations with bounds
- Loop invariants with `decreases` clauses

🚧 **Challenges**:
- Const generic arrays require workarounds
- Complex bit manipulation needs manual axioms
- No-std environment has limited prelude

### Proof Patterns
1. **Spec/Exec separation**: Ghost specifications guide executable code
2. **Frame conditions**: Explicitly state what doesn't change
3. **Quantifier triggers**: Always annotate with `#![trigger ...]`
4. **Trusted axioms**: Use `admit()` for hardware semantics (bit ops, arithmetic)

---

## References

### Documentation
- [Chapter 8: Formal Verification](../chapters/08-formal-verification.md) - Educational guide
- [Verification Setup Guide](./SETUP.md) - Installation instructions
- [Verification Roadmap](./VERIFICATION_ROADMAP.md) - Future plans

### External Resources
- [Verus Documentation](https://verus-lang.github.io/verus/)
- [seL4 Verification](https://sel4.systems/Info/FAQ/proof.pmc)
- [ARMv8-A Architecture Reference Manual](https://developer.arm.com/documentation/ddi0487/latest/)

---

*Generated by `scripts/verify.nu` on 2025-10-21*
