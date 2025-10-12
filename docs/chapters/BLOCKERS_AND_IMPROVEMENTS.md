# KaaL Microkernel - Blockers & Future Improvements

**Purpose**: Track technical debt, blockers, and future improvements across all chapters.

**Last Updated**: 2025-10-12

---

## Chapter 1: Bare Metal Boot & Early Init

**Status**: ✅ Complete

### Blockers
*None - Chapter 1 is fully complete*

### Future Improvements

#### High Priority
- [ ] **DTB Parser Enhancement**: Full FDT parsing with all node types
  - Current: Basic token parsing (FDT_BEGIN_NODE, FDT_PROP, FDT_END)
  - Needed: Complete node tree traversal, property extraction
  - Impact: Required for device discovery in later chapters
  - File: [kernel/src/boot/dtb.rs](../../kernel/src/boot/dtb.rs)

- [ ] **Error Handling**: Replace panics with Result types
  - Current: Direct panics on DTB magic mismatch, etc.
  - Needed: Proper error propagation and recovery
  - Impact: Better debugging and reliability
  - Files: [kernel/src/boot/dtb.rs](../../kernel/src/boot/dtb.rs), [kernel/src/boot/mod.rs](../../kernel/src/boot/mod.rs)

#### Medium Priority
- [ ] **UART Driver Robustness**: Add timeout and error detection
  - Current: Assumes UART always ready
  - Needed: Check status registers, handle failures
  - Impact: Better debugging when UART unavailable
  - File: [kernel/src/arch/aarch64/uart.rs](../../kernel/src/arch/aarch64/uart.rs)

- [ ] **Multi-UART Support**: Support for multiple console types
  - Current: PL011 only (QEMU virt)
  - Needed: Mini UART (RPi4), 16550 (generic)
  - Impact: Better platform portability
  - File: [kernel/src/arch/aarch64/uart.rs](../../kernel/src/arch/aarch64/uart.rs)

#### Low Priority
- [ ] **Boot Banner Customization**: Build-time configurable banner
  - Current: Hardcoded banner in boot/mod.rs
  - Needed: Generate from build-config.toml
  - Impact: Better branding flexibility
  - File: [kernel/src/boot/mod.rs](../../kernel/src/boot/mod.rs)

### Technical Debt
- **None identified** - Chapter 1 is production-quality for this stage

---

## Chapter 2: Memory Management

**Status**: ✅ Complete

### Blockers

#### Critical - Requires Chapter 3
- [x] **MMU Enable Deferred**: MMU setup complete but not enabled
  - Reason: Page fault handling not yet implemented
  - Blocker: Need exception vector table (Chapter 3) before enabling MMU
  - Impact: Kernel runs with MMU disabled (physical addressing only)
  - Resolution: Enable MMU after Chapter 3 exception handling
  - Files: [kernel/src/arch/aarch64/mmu.rs](../../kernel/src/arch/aarch64/mmu.rs), [kernel/src/memory/paging.rs](../../kernel/src/memory/paging.rs)
  - Note: All MMU registers configured, page tables mapped, ready to enable

### Future Improvements

#### High Priority
- [ ] **Frame Allocator Optimization**: Replace linear scan with buddy allocator
  - Current: O(n) bitmap scan for free frames
  - Needed: O(log n) buddy allocator for better performance
  - Impact: Critical for high-frequency allocations in later chapters
  - File: [kernel/src/memory/frame_allocator.rs](../../kernel/src/memory/frame_allocator.rs:7-21)
  - Estimated effort: 1-2 days
  - References: seL4 uses buddy allocator, see [seL4 whitepaper](https://sel4.systems/About/seL4-whitepaper.pdf)

- [ ] **Heap Allocator Safety**: Fix `static mut` reference warning
  - Current: Warning on line 38 of heap.rs - mutable reference to static
  - Issue: Undefined behavior per Rust 2024 edition
  - Needed: Use `SyncUnsafeCell` or atomic operations
  - Impact: Future-proof for Rust edition migration
  - File: [kernel/src/memory/heap.rs:38](../../kernel/src/memory/heap.rs:38)
  - Reference: https://doc.rust-lang.org/edition-guide/rust-2024/static-mut-references.html

- [ ] **Page Table Caching**: Implement TLB invalidation strategy
  - Current: No explicit TLB management
  - Needed: TLBI instructions after page table modifications
  - Impact: Prevent stale TLB entries causing faults
  - File: [kernel/src/arch/aarch64/mmu.rs](../../kernel/src/arch/aarch64/mmu.rs)
  - Estimated effort: 1 day

#### Medium Priority
- [ ] **Large Page Support**: Enable 2MB and 1GB pages
  - Current: Only 4KB pages implemented
  - Needed: Large page mapping for kernel code/data
  - Impact: Reduced TLB pressure, better performance
  - File: [kernel/src/memory/paging.rs](../../kernel/src/memory/paging.rs)
  - Estimated effort: 2-3 days

- [ ] **NUMA-Aware Frame Allocation**: Support for multi-node systems
  - Current: Single memory pool
  - Needed: Per-node frame allocators
  - Impact: Better performance on multi-socket ARM servers
  - File: [kernel/src/memory/frame_allocator.rs](../../kernel/src/memory/frame_allocator.rs)
  - Estimated effort: 3-5 days
  - Priority: Low for embedded, high for server workloads

- [ ] **Memory Statistics**: Add allocation tracking and reporting
  - Current: Basic free/total count only
  - Needed: Peak usage, fragmentation metrics, per-allocator stats
  - Impact: Better debugging and capacity planning
  - Files: [kernel/src/memory/frame_allocator.rs](../../kernel/src/memory/frame_allocator.rs), [kernel/src/memory/heap.rs](../../kernel/src/memory/heap.rs)
  - Estimated effort: 1 day

#### Low Priority
- [ ] **Heap Allocator Benchmarking**: Compare linked-list vs buddy allocator
  - Current: Using `linked_list_allocator` crate (production-ready)
  - Potential: Buddy allocator may be faster for kernel workloads
  - Impact: Performance optimization (marginal)
  - Reference: Compare with [talc](https://crates.io/crates/talc) or custom buddy allocator
  - Estimated effort: 2-3 days

- [ ] **Memory Zeroing on Free**: Security feature for sensitive data
  - Current: Frames/heap not zeroed on deallocation
  - Needed: Configurable zeroing for security-critical builds
  - Impact: Prevent information leakage between processes
  - Files: [kernel/src/memory/frame_allocator.rs](../../kernel/src/memory/frame_allocator.rs), [kernel/src/memory/heap.rs](../../kernel/src/memory/heap.rs)
  - Estimated effort: 1 day

### Technical Debt

#### Warnings to Fix
1. **Unused imports** (7 warnings)
   - `NullConsole`, `NullConfig` in config.rs:6
   - `ENTRIES_PER_TABLE`, `PageTableEntry` in paging.rs:11
   - `PageFrameNumber`, `dealloc_frame` in paging.rs:13
   - `alloc::vec::Vec` in heap.rs:15
   - `alloc::boxed::Box` in heap.rs:16
   - Action: Remove or mark with `#[allow(unused)]`
   - Impact: Code cleanliness

2. **Dead code warnings** (9 warnings)
   - Unused constants in memory_config.rs (HEAP_SIZE, MAX_PHYSICAL_FRAMES, etc.)
   - Action: These are intentional exports for future use - add `#[allow(dead_code)]`
   - Impact: Build output cleanliness

3. **Stable features** (3 warnings)
   - `naked_functions` stabilized in 1.88.0
   - `asm_const` stabilized in 1.82.0
   - Action: Remove `#![feature(...)]` declarations
   - Files: [kernel/src/lib.rs:22-23](../../kernel/src/lib.rs:22-23), [kernel/src/main.rs:3](../../kernel/src/main.rs:3)
   - Impact: Forward compatibility

#### Code Quality
- [ ] **Frame Allocator Documentation**: Add doc comments to all public methods
  - Current: Module-level docs only
  - Needed: Per-function documentation with examples
  - File: [kernel/src/memory/frame_allocator.rs](../../kernel/src/memory/frame_allocator.rs)

- [ ] **Test Coverage**: Add unit tests for edge cases
  - Current: 8 heap allocator tests (100% pass rate)
  - Needed: Frame allocator tests, page table tests
  - Impact: Better regression detection
  - Location: [examples/kernel-test/](../../examples/kernel-test/)

---

## Chapter 3: Exception Handling & Syscalls

**Status**: 📋 Planned

### Prerequisites
- ✅ Chapter 2 memory management complete
- ⬜ Exception vector table implementation
- ⬜ Trap frame (context saving/restoring)

### Known Blockers
*To be documented during implementation*

### Future Improvements
*To be documented during implementation*

---

## Chapter 4: Kernel Object Model

**Status**: 📋 Planned

### Prerequisites
- ✅ Chapter 3 exception handling complete
- ⬜ Capability representation
- ⬜ CNode implementation

### Known Blockers
*To be documented during implementation*

### Future Improvements
*To be documented during implementation*

---

## Chapter 5: IPC & Message Passing

**Status**: 📋 Planned

### Prerequisites
- ✅ Chapter 4 object model complete
- ⬜ Endpoint objects
- ⬜ Basic send/receive

### Known Blockers
*To be documented during implementation*

### Future Improvements
*To be documented during implementation*

---

## Chapter 6: Scheduling & Context Switching

**Status**: 📋 Planned

### Prerequisites
- ✅ Chapter 5 IPC complete
- ⬜ Thread control blocks (TCB)
- ⬜ Context switching

### Known Blockers
*To be documented during implementation*

### Future Improvements
*To be documented during implementation*

---

## Chapter 7: Performance & Optimization

**Status**: 📋 Planned

### Prerequisites
- ✅ Chapter 6 scheduling complete
- ⬜ Baseline benchmarks

### Known Blockers
*To be documented during implementation*

### Future Improvements
*To be documented during implementation*

---

## Chapter 8: Verification & Hardening

**Status**: 📋 Planned

### Prerequisites
- ✅ Chapter 7 performance complete
- ⬜ Security audit

### Known Blockers
*To be documented during implementation*

### Future Improvements
*To be documented during implementation*

---

## Cross-Cutting Concerns

### Build System
- [ ] **Platform Detection**: Auto-detect target platform
  - Current: Manual `--platform` flag
  - Needed: Detect from hardware or environment
  - Impact: Better user experience

- [ ] **Incremental Builds**: Speed up rebuilds
  - Current: Full rebuild on config change
  - Needed: Track dependencies properly
  - Impact: Faster development iteration

### Testing Infrastructure
- [x] **Unit Test Framework**: Custom no_std test runner
  - Status: ✅ Complete - [examples/kernel-test/](../../examples/kernel-test/)
  - 8/8 heap allocator tests passing

- [ ] **Integration Tests**: System-level testing
  - Current: Manual QEMU runs
  - Needed: Automated test suite with assertions
  - Impact: Regression prevention

- [ ] **CI/CD Pipeline**: Automated builds and tests
  - Current: Manual builds
  - Needed: GitHub Actions or similar
  - Impact: Continuous validation

### Documentation
- [x] **Chapter Status Tracking**: Per-chapter progress docs
  - Status: ✅ Complete - CHAPTER_01_STATUS.md, CHAPTER_02_STATUS.md

- [x] **Blockers Document**: This document
  - Status: ✅ Complete - BLOCKERS_AND_IMPROVEMENTS.md

- [ ] **API Documentation**: rustdoc for all modules
  - Current: Partial doc comments
  - Needed: Complete API docs with examples
  - Impact: Better code maintainability

---

## How to Use This Document

### For Contributors
1. **Before Starting Work**: Check blockers for your chapter
2. **During Implementation**: Document new blockers/improvements as discovered
3. **After Completion**: Move items from "Future" to "Complete" or next chapter

### For Project Planning
- **Critical Blockers**: Must be resolved before chapter completion
- **High Priority Improvements**: Should be addressed in current chapter
- **Medium/Low Priority**: Can be deferred to later optimization passes

### Update Frequency
- Update after completing each chapter
- Add blockers immediately when discovered
- Review monthly for priority adjustments

---

## Priority Legend

- **Critical**: Blocks further progress, must fix now
- **High**: Significantly impacts functionality/performance, address soon
- **Medium**: Noticeable improvement, schedule for next optimization pass
- **Low**: Nice-to-have, defer until post-v1.0

---

**Maintained By**: KaaL Development Team
**Next Review**: After Chapter 3 completion
