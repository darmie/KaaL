# KaaL Microkernel - Blockers & Future Improvements

**Purpose**: Track technical debt, blockers, and future improvements across all chapters.

**Last Updated**: 2025-10-14

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

**Status**: ✅ Complete - MMU Fully Operational! (2025-10-13)

### Blockers

#### ~~Critical - Requires Chapter 3~~ ✅ RESOLVED (2025-10-13)
- [x] **MMU Enable Deferred**: ✅ **COMPLETE** - MMU now enabled and working!
  - Resolution: Integrated exception handling with MMU enable
  - Result: MMU successfully enabled with virtual memory
  - Fixed: Three critical bugs (PXN bit, exception timing, block encoding)
  - Status: Kernel heap working, Box/Vec allocations successful
  - See: [SESSION_SUMMARY_2025-10-13.md](../SESSION_SUMMARY_2025-10-13.md) for details

### Future Improvements

#### High Priority
- [x] **Large Page Support**: ✅ **COMPLETE** - 2MB blocks now working!
  - Implementation: Block entries with TABLE_OR_PAGE bit cleared
  - Status: Heap region uses 2MB blocks for efficient mapping
  - File: [kernel/src/memory/paging.rs](../../kernel/src/memory/paging.rs)

- [x] **Page Table Caching**: ✅ **COMPLETE** - TLB invalidation implemented
  - Implementation: `tlbi vmalle1` before MMU enable with proper barriers
  - Status: Full system barriers (dsb sy, isb) in place
  - File: [kernel/src/arch/aarch64/mmu.rs](../../kernel/src/arch/aarch64/mmu.rs)

- [ ] **Frame Allocator Optimization**: Replace linear scan with buddy allocator
  - Current: O(n) bitmap scan for free frames (acceptable for now)
  - Needed: O(log n) buddy allocator for better performance
  - Impact: Important for high-frequency allocations
  - Priority: **DEFERRED** to post-Chapter 4 (not blocking)
  - File: [kernel/src/memory/frame.rs](../../kernel/src/memory/frame.rs)
  - Estimated effort: 1-2 days

- [ ] **Heap Allocator Safety**: Fix `static mut` reference warning
  - Current: Warning about mutable reference to static
  - Needed: Use `SyncUnsafeCell` or atomic operations
  - Priority: **DEFERRED** - not affecting functionality
  - File: [kernel/src/memory/heap.rs](../../kernel/src/memory/heap.rs)
  - Estimated effort: 1 day

#### Medium Priority (Deferred)

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

**Status**: ✅ COMPLETE (2025-10-13)

### Completed ✅
- [x] Exception vector table (16 entries, 2KB aligned)
- [x] Trap frame structure (36 × 64-bit registers)
- [x] Context save/restore assembly (52 instructions)
- [x] Exception handlers (synchronous exceptions working)
- [x] ESR/FAR decoding for fault analysis
- [x] Integration with MMU (handlers installed before MMU enable)
- [x] **Test Exception Handling** - Data abort (EC 0x25) ✅ TESTED
- [x] **Syscall Testing** - Syscall (EC 0x15) ✅ TESTED

### Testing Results

Both exception types verified successfully:
- **Data Abort**: Caught at FAR 0xdeadbeef, decoded EC 0x25, translation fault level 1
- **Syscall**: Caught EC 0x15, extracted syscall #42 and arguments from trap frame

### Blockers for Chapter 4
**None** - All prerequisites complete! Ready to proceed to Chapter 4.

### Future Improvements (Post-Chapter 3)

- [ ] **User Mode Context Switching**
  - Needed for full syscall testing from EL0
  - Will be implemented in Chapter 4 with TCBs

- [ ] **Advanced Page Fault Handling**
  - Current: Panics with detailed fault info
  - Future: Demand paging, copy-on-write
  - Can defer to later optimization phase

---

## Chapter 4: Kernel Object Model

**Status**: 🚧 IN PROGRESS - 43% Complete (3/7 phases, 2025-10-13)

### Completed ✅
- [x] **Phase 1: Capability System** (2025-10-13)
  - 32-byte Capability structure, CapType enum, CapRights bitflags
  - Capability derivation and minting
  - File: [kernel/src/objects/capability.rs](../../kernel/src/objects/capability.rs)

- [x] **Phase 2: CNode Implementation** (2025-10-13)
  - Capability container (2^n slots, O(1) lookup)
  - Insert/delete/move/copy operations
  - File: [kernel/src/objects/cnode.rs](../../kernel/src/objects/cnode.rs)

- [x] **Phase 3: TCB Implementation** (2025-10-13)
  - Thread representation with TrapFrame, state machine
  - CSpace/VSpace roots, IPC buffer, priority scheduling
  - File: [kernel/src/objects/tcb.rs](../../kernel/src/objects/tcb.rs)

### Remaining Phases

- [ ] **Phase 4: Endpoint Objects** - Basic IPC structure (2-3 hours)
- [ ] **Phase 5: Untyped Memory** - Retyping infrastructure (4-6 hours)
- [ ] **Phase 6: Object Invocations** - Syscall dispatch (6-8 hours)
- [ ] **Phase 7: Integration Testing** - End-to-end tests (4-6 hours)

### Rendezvous Point with Chapter 5

**Decision**: We can proceed to Chapter 5 (IPC) after Phase 4 completes.

**Rationale**:
- ✅ Core object infrastructure ready (Capability, CNode, TCB)
- ⬜ Endpoint basic structure needed (Phase 4, ~3 hours)
- ⚠️ Untyped memory can be deferred - not required for IPC
- ⚠️ Object invocations better understood after IPC implementation

**Proposed Path**:
1. Complete Phase 4 (Endpoint structure)
2. Move to Chapter 5 (implement full IPC with message passing)
3. Return to Chapter 4 Phases 5-7 (Untyped, invocations, testing)

This provides a natural development flow where IPC implementation informs the final object model design.

### Known Blockers
**None** - All dependencies from previous chapters complete.

### Future Improvements

#### High Priority
- [ ] **Capability Revocation** - Required for security (1-2 days)
- [ ] **CNode Guard Bits** - Efficient capability addressing (1 day)
- [ ] **TCB Scheduler Integration** - Deferred to Chapter 6

#### Medium Priority
- [ ] **Object Size Optimization** - Cache efficiency (1-2 days)
- [ ] **Capability Address Space Compression** (2-3 days)

#### Low Priority
- [ ] **Type-Safe Object Wrappers** - Better compile-time safety (2-3 days)

---

## Chapter 5: IPC & Message Passing

**Status**: ✅ COMPLETE - 100% (Implementation Complete, 2025-10-14)

### Completed ✅
- [x] **Phase 1: Message Structure** (2025-10-13)
  - Message with label, 64 registers, 3 capabilities
  - IpcBuffer for extended data
  - File: [kernel/src/ipc/message.rs](../../kernel/src/ipc/message.rs)

- [x] **Phase 2: Send Operation** (2025-10-14)
  - Complete implementation with scheduler integration
  - Capability rights validation, fast/slow path transfer
  - File: [kernel/src/ipc/operations.rs](../../kernel/src/ipc/operations.rs)

- [x] **Phase 3: Receive Operation** (2025-10-14)
  - Complete implementation with scheduler integration
  - File: [kernel/src/ipc/operations.rs](../../kernel/src/ipc/operations.rs)

- [x] **Phase 4: Message Transfer** (2025-10-14)
  - Fast path (registers) and slow path (IPC buffer) working
  - File: [kernel/src/ipc/transfer.rs](../../kernel/src/ipc/transfer.rs)

- [x] **Phase 5: Capability Transfer** (2025-10-14)
  - Complete grant/mint/derive capability transfer protocol
  - File: [kernel/src/ipc/cap_transfer.rs](../../kernel/src/ipc/cap_transfer.rs)

- [x] **Phase 6: Call/Reply Semantics** (2025-10-14)
  - Complete RPC-style operations with reply capabilities
  - File: [kernel/src/ipc/call.rs](../../kernel/src/ipc/call.rs)

- [x] **Phase 7: Testing** (2025-10-14)
  - 4/4 message structure tests passing
  - Full IPC operation tests deferred to Chapter 7 (require multi-threading)
  - File: [kernel/src/ipc/test_runner.rs](../../kernel/src/ipc/test_runner.rs)
  - Documentation: [CHAPTER_05_IPC_TEST_LIMITATIONS.md](./CHAPTER_05_IPC_TEST_LIMITATIONS.md)

### Implementation Summary
- **Total Code**: ~1,630 lines of production IPC implementation
- **Message Structure**: 370 LOC
- **Send/Receive Operations**: 300 LOC
- **Message Transfer**: 200 LOC
- **Capability Transfer**: 370 LOC
- **Call/Reply**: 390 LOC
- **Test Infrastructure**: 500+ LOC (static allocation only)

### Known Limitations

#### Testing Limitations
- **Full IPC operation tests deferred to Chapter 7**
  - Reason: IPC rendezvous requires multi-threading (two active threads)
  - Current: Single-threaded test harness can only test message structures
  - Status: 4/4 message tests passing (100% of testable parts)
  - Future: Will be fully tested in Chapter 7 with user-space programs
  - Documentation: See [CHAPTER_05_IPC_TEST_LIMITATIONS.md](./CHAPTER_05_IPC_TEST_LIMITATIONS.md)

### Future Improvements

#### High Priority (Chapter 9)
- [ ] **Full IPC Integration Tests** - End-to-end IPC with real components
  - Needed: Runtime Services (Capability Broker, Memory Manager) that can send/receive
  - Location: Chapter 9 (Framework Integration & Runtime Services)
  - Estimated effort: 1 week (Phase 2 of Chapter 9)

#### Medium Priority
- [ ] **IPC Performance Optimization** - Fastpath improvements
  - Current: Fast path implemented, further optimization possible
  - Impact: Reduce IPC latency
  - Estimated effort: 2-3 days

#### Low Priority
- [ ] **IPC Buffer Size Configuration** - Runtime configurable buffer size
  - Current: Fixed 512-byte IPC buffer
  - Needed: Per-process configurable size
  - Impact: Better memory efficiency
  - Estimated effort: 1 day

---

## Chapter 6: Scheduling & Context Switching

**Status**: ✅ COMPLETE - 100% (6/6 phases, 2025-10-14)

### Completed ✅
- [x] **Phase 1: Scheduler Infrastructure** (2025-10-14)
  - Round-robin and priority scheduling
  - Global scheduler state with run queues
  - File: [kernel/src/scheduler/mod.rs](../../kernel/src/scheduler/mod.rs)

- [x] **Phase 2: Round-Robin Scheduling** (2025-10-14)
  - Integrated within Phase 1
  - FIFO queue per priority level

- [x] **Phase 3: Priority Scheduling** (2025-10-14)
  - Integrated within Phase 1
  - 256 priority levels (0 = highest)

- [x] **Phase 4: Context Switching** (2025-10-14)
  - Complete ARM64 assembly implementation
  - Saves/restores all 31 GPRs, SP, PC, SPSR
  - File: [kernel/src/arch/aarch64/context_switch.rs](../../kernel/src/arch/aarch64/context_switch.rs)

- [x] **Phase 5: IPC Integration** (2025-10-14)
  - Scheduler yield points in send/recv/call/reply
  - Blocking state management

- [x] **Phase 6: Timer & Preemption** (2025-10-14)
  - ARM Generic Timer integration
  - Configurable timeslice (default 10ms)
  - File: [kernel/src/scheduler/timer.rs](../../kernel/src/scheduler/timer.rs)

### Implementation Summary
- **Total Code**: ~1,100 lines
- **Scheduler Core**: 656 LOC
- **Context Switching**: 252 LOC
- **Timer Integration**: 200 LOC

### Known Blockers
**None** - Chapter 6 complete!

### Future Improvements

#### Medium Priority
- [ ] **SMP Support** - Multi-core scheduling
  - Current: Single-core only
  - Needed: Per-CPU run queues, load balancing
  - Impact: Better performance on multi-core ARM systems
  - Estimated effort: 1-2 weeks

#### Low Priority
- [ ] **Advanced Schedulers** - CFS, EDF, etc.
  - Current: Simple priority-based round-robin
  - Potential: More sophisticated scheduling algorithms
  - Impact: Better fairness/latency for specific workloads
  - Estimated effort: 2-3 weeks

---

## Chapter 7: Root Task & Boot Protocol

**Status**: 📋 Planned

### Prerequisites
- ✅ Chapters 1-6 complete
- ⬜ ELF loader for initial task
- ⬜ Boot info structure
- ⬜ Initial capability space

### Objectives
1. Implement ELF loader for root task
2. Create boot info structure (memory regions, device tree, initial caps)
3. Load and start root task (first user-space component)
4. Establish initial capability delegation
5. Basic root task that prints "Hello from user-space!"

**Note**: This chapter focuses on **microkernel-side boot protocol only**. The root task itself will be a minimal stub. Full Runtime Services (Capability Broker, Memory Manager) are part of the **KaaL Framework**, developed separately after microkernel is complete.

### Known Blockers
**None** - All prerequisites from Chapters 1-6 are complete!

### Future Improvements
*To be documented during implementation*

---

## Chapter 8: Verification & Hardening

**Status**: 📋 Planned

### Prerequisites
- ✅ Chapter 7 complete (Root Task & Boot Protocol)
- ⬜ Security audit framework
- ⬜ Verus proof infrastructure

### Objectives
1. Add Verus proofs for core invariants
2. Prove memory safety properties
3. Verify IPC correctness
4. Stress testing framework

### Known Blockers
*To be documented during implementation*

### Future Improvements
*To be documented during implementation*

---

## Chapter 9: Framework Integration & Runtime Services

**Status**: 📋 Planned

### Prerequisites
- ✅ Chapters 0-8 complete (microkernel done!)
- ⬜ Component architecture design
- ⬜ IPC binding library

### Objectives

#### Phase 1: Runtime Services Foundation
1. Implement Capability Broker (~5K LOC)
2. Implement Memory Manager (~3K LOC)

#### Phase 2: IPC Integration Testing
3. Full end-to-end IPC tests (deferred from Chapter 5)
4. IPC performance benchmarking

#### Phase 3: DDDK & Basic Drivers
5. Device Driver Development Kit
6. Example drivers (UART, Timer, GPIO)

#### Phase 4: System Services
7. Basic VFS implementation
8. Network stack foundation

### Known Blockers
**None** - All microkernel prerequisites complete after Chapter 8!

### Future Improvements
*To be documented during implementation*

**Note**: Chapter 9 transitions from **microkernel development** (Chapters 0-8) to **ecosystem building** (Framework components in user-space).

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
**Next Review**: After Chapter 7 planning
