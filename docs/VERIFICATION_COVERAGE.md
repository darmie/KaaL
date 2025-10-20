# Formal Verification Coverage Report

**Last Updated**: 2025-10-20
**Total Status**: 10 modules, 96 items, 0 errors

## Executive Summary

KaaL's formal verification effort uses [Verus](https://github.com/verus-lang/verus) to mathematically prove correctness properties of critical kernel components. All verification has **zero runtime overhead** as proof code is erased during compilation.

### Current Coverage

| Category | Verified | Production | Coverage |
|----------|----------|------------|----------|
| **Memory Operations** | 3 modules (25 items) | ✅ | High |
| **Capability System** | 3 modules (20 items) | ✅ | Medium |
| **Thread Management** | 1 module (29 items) | ✅ | High |
| **Page Tables** | 1 module (7 items) | ⚠️ | Low |
| **Bitmaps** | 2 modules (15 items) | ✅ | High |
| **IPC** | 0 modules | ❌ | None |
| **Scheduling** | 0 modules | ❌ | None |
| **Syscalls** | 0 modules | ❌ | None |

## Verified Modules

### 1. Memory Operations (25 items)

#### PhysAddr (10 items)
- **File**: `kernel/src/verified/phys_addr.rs`
- **Production**: `kernel/src/memory/address.rs:47-155`
- **Deviation**: None (EXACT production code)
- **Functions**: new, as_usize, is_aligned, align_down, align_up, page_number, is_null
- **Properties**:
  - Alignment proofs (modular arithmetic)
  - Bounds checking
  - Address arithmetic safety

#### VirtAddr (10 items)
- **File**: `kernel/src/verified/virt_addr.rs`
- **Production**: `kernel/src/memory/address.rs:195-288`
- **Deviation**: None (EXACT production code)
- **Functions**: new, as_usize, is_aligned, align_down, align_up, page_number, is_null
- **Properties**: Same as PhysAddr

#### PageFrameNumber (5 items)
- **File**: `kernel/src/verified/page_frame_number.rs`
- **Production**: `kernel/src/memory/address.rs:290-342`
- **Deviation**: None (EXACT production code)
- **Functions**: new, as_usize, phys_addr, from_phys_addr
- **Properties**:
  - Page number conversion
  - Address to PFN mapping

### 2. Capability System (20 items)

#### CapRights (4 items)
- **File**: `kernel/src/verified/cap_rights.rs`
- **Production**: `kernel/src/objects/capability.rs:256-307`
- **Deviation**: None (EXACT production code)
- **Operations**: empty, contains, get_bits, constants (READ, WRITE, GRANT, ALL)
- **Properties**:
  - Bit flag operations
  - Rights containment

#### CNode Operations (6 items)
- **File**: `kernel/src/verified/cnode_ops.rs`
- **Production**: `kernel/src/objects/cnode.rs`
- **Deviation**: None (EXACT production code)
- **Functions**: num_slots, is_valid_index, size_bits, count, validate_size_bits
- **Properties**:
  - Power-of-2 slot calculations (16-4096 slots)
  - Index bounds checking
  - CNode size validation

#### Capability Operations (10 items)
- **File**: `kernel/src/verified/capability_ops.rs`
- **Production**: `kernel/src/objects/capability.rs`
- **Deviation**: ⚠️ **Simplified** (see below)
- **Functions**: has_right, derive, union, intersection, contains, bits, from_bits, empty
- **Properties**:
  - Rights derivation (can only reduce, not add)
  - Bitwise operation axioms
  - Union/intersection correctness

**Deviation Details**:
- **derive()**: Verified version omits `cap_type`, `object_ptr`, `guard` fields
- **Reason**: Focus on rights derivation logic (the critical security property)
- **Impact**: Core algorithm IDENTICAL, only data fields simplified
- **Safety**: Rights checking logic is EXACT production code

### 3. Thread Management (29 items)

#### TCB State Machine (29 items)
- **File**: `kernel/src/verified/tcb.rs`
- **Production**: `kernel/src/objects/tcb.rs`
- **Deviation**: None (EXACT production code)
- **Operations**:
  - State transitions: Inactive→Runnable→Running→Blocked
  - Capability checking
  - Time slice management
  - Thread lifecycle (activate, deactivate, block, unblock, resume, suspend)
- **Properties**:
  - State machine invariants
  - Frame conditions (what doesn't change)
  - Time slice overflow protection

### 4. Page Tables (7 items)

#### PageTableLevel Operations (7 items)
- **File**: `kernel/src/verified/page_table_ops.rs`
- **Production**: `kernel/src/arch/aarch64/page_table.rs:223-270`
- **Deviation**: ⚠️ **Simplified** (see below)
- **Functions**: shift, block_size, index, supports_blocks, next, entries_per_table
- **Properties**:
  - ARMv8-A 4-level page table structure
  - Level-specific shifts (L0:39, L1:30, L2:21, L3:12)
  - Block sizes (512GB, 1GB, 2MB, 4KB)
  - Index extraction (9-bit, 512 entries)

**Deviation Details**:
- **index()**: Production uses `(vaddr >> shift) & 0x1FF`, verified uses `(vaddr / block_size) % 512`
- **Reason**: Verus doesn't support bitwise operations in spec functions
- **Mathematical Equivalence**: Division by 2^n equals right shift by n; modulo 512 equals masking with 0x1FF
- **Safety**: Mathematically proven equivalent; both extract 9-bit page table indices

### 5. Bitmaps (15 items)

#### Simple Bitmap (3 items)
- **File**: `kernel/src/verified/bitmap_simple.rs`
- **Purpose**: Teaching/reference implementation
- **Functions**: new, set, is_set

#### Production Bitmap (12 items)
- **File**: `kernel/src/verified/bitmap_prod.rs`
- **Production**: `kernel/src/memory/bitmap.rs`
- **Deviation**: None (EXACT production code)
- **Functions**: new, is_set, set, clear, find_first_unset
- **Advanced Features**:
  - 4 bit-level axioms (OR/AND operations)
  - Frame conditions with `old()`
  - Loop invariants with termination proofs
  - Quantified assumptions

## Algorithm Deviations Summary

| Module | Function | Deviation | Reason | Impact |
|--------|----------|-----------|--------|--------|
| capability_ops | derive() | Omits cap_type, object_ptr, guard fields | Focus on rights logic | Core algorithm IDENTICAL |
| page_table_ops | index() | Uses division/modulo instead of bit ops | Verus spec function limits | Mathematically equivalent |

**Key Finding**: All deviations are either:
1. **Structural simplifications** (omitting data fields while preserving logic)
2. **Mathematical equivalences** (different notation, same semantics)

**No algorithmic deviations exist** - all core logic is EXACT production code.

## Remaining High-Priority Targets

### Immediate Priority
1. **IPC Operations** (0% coverage)
   - Endpoint send/receive
   - Message passing
   - Capability transfer
   - Call/reply protocols

2. **Syscall Interface** (0% coverage)
   - Syscall argument validation
   - Capability lookup
   - Error handling

### Medium Priority
3. **Scheduler Operations** (0% coverage)
   - Priority queue operations
   - Thread selection
   - Context switching logic

4. **Frame Allocator** (0% coverage)
   - Allocation/deallocation
   - Free list management
   - Bitmap integration

5. **VSpace Operations** (0% coverage)
   - Page table manipulation
   - Memory mapping
   - TLB management

### Future Work
6. **Advanced Page Tables**
   - Page table entry manipulation
   - Multi-level traversal
   - Block mapping logic

7. **Interrupt Handling**
   - IRQ handler registration
   - Priority management

## Verification Techniques Used

### 1. Specification Functions
```rust
pub closed spec fn spec_is_bit_set(self, index: usize) -> bool {
    // Abstract model of the operation
}
```

### 2. Frame Conditions
```rust
ensures
    self.is_bit_set(index),  // What changed
    forall|i: usize| i != index ==>
        self.is_bit_set(i) == old(self).is_bit_set(i)  // What didn't
```

### 3. Loop Invariants
```rust
while idx < max
    invariant idx <= max,
              forall|j: usize| j < idx ==> self.is_bit_set(j),
    decreases max - idx  // Termination proof
```

### 4. Axioms (Trusted Assumptions)
```rust
proof fn axiom_or_sets_bit(val: u64, bit_idx: u64)
    requires bit_idx < 64,
    ensures (val | (1u64 << bit_idx)) & (1u64 << bit_idx) != 0,
{
    admit()  // Zero runtime cost
}
```

### 5. State Machine Verification
```rust
pub closed spec fn valid_transition(from: ThreadState, to: ThreadState) -> bool {
    match (from, to) {
        (Inactive, Runnable) => true,
        (Runnable, Running) => true,
        // ... exhaustive pattern matching
    }
}
```

## Comparison with Other Verified Kernels

| Kernel | Verifier | Lines Verified | Approach |
|--------|----------|----------------|----------|
| **seL4** | Isabelle/HOL | ~10,000 | Full functional correctness |
| **Asterinas** | Verus | ~5,000 | Modular verification |
| **KaaL** | Verus | ~1,500 | Critical path verification |

**KaaL's Strategy**: Verify critical security-relevant components first (capabilities, memory, TCB) before expanding to full kernel coverage.

## Metrics

- **Total Verified Items**: 96
- **Total Modules**: 10
- **Verification Errors**: 0
- **Axioms Used**: 8 (all documented)
- **Lines of Proof Code**: ~1,500
- **Production Lines Verified**: ~800
- **Verification Overhead**: 0% (proofs erased at compile time)

## Running Verification

```bash
# Verify all modules
nu scripts/verify.nu

# Verify specific module
~/verus/verus kernel/src/verified/tcb.rs
```

## References

- [Verus Documentation](https://verus-lang.github.io/verus/)
- [Advanced Verification Techniques](./ADVANCED_VERIFICATION.md)
- [Verification Setup Guide](./verification/SETUP.md)

---

**Note**: This report reflects verification status as of the last update. Run `nu scripts/verify.nu` for current status.
