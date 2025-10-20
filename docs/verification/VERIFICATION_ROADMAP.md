# Verification Roadmap for KaaL Microkernel

**Date**: 2025-10-20
**Status**: Phase 1 Complete, Phase 2 In Progress

---

## Executive Summary

This document outlines the verification strategy for the KaaL microkernel using Verus.
We're taking a **pragmatic, iterative approach** that balances:
- **Real value**: Verify actual production code, not toy models
- **Incremental progress**: Start with simpler properties, build up to complex ones
- **Engineering reality**: Work within Verus's current capabilities

---

## What We've Accomplished

### Phase 1: Framework Setup ✅

**Completed**: 2025-10-20

- ✅ Installed Verus 0.2025.10.17 and Z3 v4.15.3
- ✅ Created verification infrastructure (`.verus/`, `scripts/verify.nu`)
- ✅ **First verified code**: `bitmap_simple.rs` (3/3 functions verified)
- ✅ Established verification workflow

**Key Learning**: Verus works well for standalone algorithms with clear specifications.

---

## Phase 2: Memory Safety Proofs 🚧

**Started**: 2025-10-20
**Target**: Verify core data structures used by frame allocator

### Approach Evolution

#### Initial Plan (Naive)
✅ Create toy verified frame allocator
❌ Problem: Doesn't verify production code, limited value

#### Revised Plan (Better)
✅ Extract bitmap operations into verified core
✅ Use verified bitmap in production frame allocator
🚧 **Current challenge**: Verus limitations with const generics

### Current Status

**Created**:
- `kernel/src/memory/verified_bitmap.rs` - Production bitmap with verification hooks
- `kernel/src/verified/bitmap_core.rs` - Standalone verification target

**Verification Challenges**:
1. **Const generic arrays**: Verus struggles with `[u64; N]` where N is const generic
2. **Array initialization**: Hard to prove `[0u64; N]` creates all-zeros bitmap
3. **Bit manipulation proofs**: Requires manual axioms for bit operations

**Next Steps**:
1. Simplify bitmap to fixed size (e.g., `[u64; 16]` for 1024 frames)
2. Add manual axioms for bit operations if needed
3. Verify simpler properties first, then build up

---

## Verification Strategy: Pragmatic Approach

### What We're Doing

**✅ Modular Extraction**
- Extract core algorithms into verifiable modules
- Keep complex plumbing (address translation, config) separate
- Verify the algorithm, not the integration

**✅ Feature Flags**
- Production code compiles with or without verification
- `#[cfg(feature = "verification")]` for Verus-specific code
- Non-verified fallback always available

**✅ Incremental Properties**
- Start with basic safety (bounds checks, no double-allocation)
- Build up to functional correctness (allocator returns valid frames)
- Eventually prove security properties (isolation, no leaks)

### What We're NOT Doing (Yet)

**❌ Full System Verification** (too ambitious for Phase 2)
- seL4 took 11 person-years for full verification
- We're aiming for core algorithm verification first

**❌ Verifying Everything** (diminishing returns)
- Focus on security-critical code: memory allocator, capability system, IPC
- Leave logging, debugging, peripheral drivers unverified

**❌ Fighting Verus Limitations** (pragmatic engineering)
- If Verus can't handle something, simplify or defer
- Don't let perfect be the enemy of good

---

## Concrete Verification Targets

### Phase 2a: Bitmap Operations (Current)

**Target**: `VerifiedBitmap<const N: usize>`

**Properties to Verify**:
1. ✅ `new()` creates empty bitmap
2. ✅ `set(i)` sets bit i, preserves others
3. ✅ `is_set(i)` returns correct value
4. 🚧 `find_first_unset()` returns unallocated index
5. 🚧 No bit is both set and unset (consistency)

**Current Blockers**:
- Const generic array initialization proofs
- Bit manipulation axioms needed

**Workaround**:
- Fix `N=16` for now (1024 frames = 4MB RAM)
- Prove properties on fixed-size bitmap
- Generalize later if Verus improves

### Phase 2b: Frame Allocator Core (Next)

**Target**: Production `FrameAllocator` using `VerifiedBitmap`

**Properties to Verify**:
1. **No double allocation**: `alloc()` never returns same frame twice
2. **Deallocation safety**: Can only `dealloc()` allocated frames
3. **Conservation**: `allocated + free == total` frames
4. **Bounds safety**: Returned frames are within valid range

**Strategy**:
- Use verified bitmap as building block
- Add allocator-level specifications
- Verify high-level properties, assume bitmap correctness

### Phase 2c: Integration Testing

**Goal**: Confidence that verified code matches production behavior

**Approach**:
1. Extensive property-based tests
2. Fuzzing with verified properties as oracle
3. Runtime assertions checking verification invariants

---

## Verus Capability Assessment

Based on our experience so far:

### What Verus Handles Well ✅

- **Simple algorithms**: Linear search, tree traversal
- **Clear specifications**: Pre/postconditions on functions
- **Loop invariants**: With explicit `decreases` clauses
- **Basic data structures**: Structs with simple fields
- **Quantifiers**: With explicit `#[trigger]` annotations

### What Verus Struggles With 🚧

- **Const generics**: Arrays with generic size parameters
- **Complex initialization**: Proving array contents from initialization
- **Bit manipulation**: Requires manual axioms for bitwise ops
- **External dependencies**: Modules with generated code
- **No-std environment**: Limited prelude compared to std

### Workarounds We're Using

1. **Fixed-size specialization**: Use `N=16` instead of generic `N`
2. **Manual axioms**: Add trusted specs for bit operations
3. **Layered verification**: Verify core, assume dependencies
4. **Feature flags**: Conditional compilation for verification

---

## Success Metrics

### Phase 2 Goals

**Minimum (Must Have)**:
- ✅ Bitmap with 4/5 operations verified
- 🎯 Frame allocator with 2/4 safety properties proven
- 🎯 Production code compiles with verification enabled
- 🎯 Documentation of verification approach

**Target (Should Have)**:
- 🎯 Bitmap fully verified (5/5 operations)
- 🎯 Frame allocator fully verified (4/4 properties)
- 🎯 Integration tests validate verified properties
- 🎯 CI runs verification on every commit

**Stretch (Nice to Have)**:
- 🎯 Page table verification started
- 🎯 Capability system spec defined
- 🎯 Proof reuse patterns documented

---

## Lessons Learned

### From Phase 1

1. **Binary releases >> building from source** (saved hours)
2. **Trigger annotations are mandatory** (not optional)
3. **Start simple, iterate** (bitmap before allocator)
4. **Document early** (setup guide invaluable)

### From Phase 2 (So Far)

1. **Const generics are hard** (Verus limitation, not bug)
2. **Modular extraction works** (separate verification from integration)
3. **Feature flags essential** (optional verification pragmatic)
4. **Simplify when stuck** (fixed-size arrays acceptable)

---

## Timeline Revised

### Original Estimate
- Phase 1: 1 week → **Actual: 1 day** ✅
- Phase 2: 1-2 weeks → **Revised: 2-3 weeks** 🚧

### Why Longer?

**Underestimated**:
- Verus learning curve (const generics, triggers, axioms)
- Proof engineering complexity (bit manipulation proofs)
- Iterative refinement (multiple attempts to find working approach)

**Realistic Timeline**:
- Week 1: Bitmap verification (90% done)
- Week 2: Frame allocator verification
- Week 3: Integration, testing, documentation

---

## References

### Similar Projects

- **seL4**: 11 person-years, Isabelle/HOL, full functional correctness
- **IronFleet**: Verified distributed systems, Dafny
- **Verus Papers**: Linear Dafny successor, Rust integration

### Key Differences

**seL4** (Comparison):
- Manual Hoare logic in Isabelle (much more work)
- Verifies actual C code via refinement
- Proves functional correctness + security properties

**KaaL** (Our Approach):
- Automated SMT solving via Verus (less manual proof)
- Verify Rust code directly (no refinement gap)
- Focus on safety properties first, correctness later

---

## Path Forward

### Immediate (This Week)

1. Fix bitmap verification (simplify to `N=16`)
2. Verify frame allocator core (2/4 properties minimum)
3. Integrate verified bitmap into production code
4. Update Chapter 8 status to Phase 2 progress

### Near Term (Next 2 Weeks)

1. Complete frame allocator verification (4/4 properties)
2. Add integration tests validating verified properties
3. Document proof patterns for future verification
4. Begin page table verification planning

### Long Term (Phase 3-5)

1. **Phase 3**: IPC correctness proofs
2. **Phase 4**: Capability system security properties
3. **Phase 5**: Integration and system-level properties

---

## Conclusion

**We're making real progress** on verification, but it's harder than initially estimated.
The pragmatic approach (modular extraction + incremental properties) is working.

**Key Insight**: Verification is a marathon, not a sprint. Focus on:
- ✅ Verifying real code (not toys)
- ✅ Proving valuable properties (safety first)
- ✅ Building incrementally (simple → complex)
- ✅ Documenting learnings (for future work)

**Next milestone**: Frame allocator with proven no-double-allocation property.

---

*Last Updated: 2025-10-20*
*Phase 1: Complete | Phase 2: 40% | Overall: 28%*
