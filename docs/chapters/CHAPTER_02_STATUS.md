# Chapter 2: Memory Management - Status

**Status**: 🚧 IN PROGRESS
**Started**: 2025-10-12
**Target Completion**: TBD

## Objectives

1. ⬜ Set up ARM64 page tables (TTBR0, TTBR1)
2. ⬜ Implement virtual memory mapping
3. ⬜ Create physical memory allocator (frame allocator)
4. ⬜ Implement kernel heap allocator
5. ⬜ Enable MMU with identity mapping
6. ⬜ Separate kernel and user address spaces

## Overview

Chapter 2 focuses on implementing a complete memory management subsystem for the KaaL microkernel. Following seL4's architecture, we'll implement:

- **Page tables** - ARM64 4-level page tables (L0/L1/L2/L3)
- **Virtual memory** - Separate kernel and user address spaces
- **Physical allocator** - Manage physical memory frames
- **Kernel heap** - Dynamic allocation for kernel data structures

## Memory Layout (QEMU virt, 128MB)

```
Physical Memory:
0x00000000 - 0x3FFFFFFF  (1GB)   [Unmapped - MMIO space]
0x40000000 - 0x47FFFFFF  (128MB) [RAM]
  ├─ 0x40000000          DTB
  ├─ 0x40200000          Elfloader
  ├─ 0x40400000          Kernel code/data
  └─ 0x40500000+         Available for allocation

Virtual Memory:
0x0000000000000000 - 0x0000FFFFFFFFFFFF  User space (TTBR0)
0xFFFF000000000000 - 0xFFFFFFFFFFFFFFFF  Kernel space (TTBR1)
  ├─ 0xFFFF000000000000  Kernel code (identity mapped from 0x40400000)
  ├─ 0xFFFF000010000000  Kernel heap
  └─ 0xFFFF800000000000  Physical memory map
```

## Key Concepts (seL4-Inspired)

### 1. Page Table Hierarchy (ARM64)

```
L0 Table (512GB per entry)
  └─ L1 Table (1GB per entry)
      └─ L2 Table (2MB per entry)
          └─ L3 Table (4KB per entry)
```

### 2. Translation Tables (seL4 Model)

- **TTBR0_EL1** - User-space translations (0x0000...)
- **TTBR1_EL1** - Kernel-space translations (0xFFFF...)
- **TCR_EL1** - Translation Control Register

### 3. Physical Memory Management

Following seL4's untyped memory model:
- Track physical frames (4KB pages)
- Allocate/free frames for page tables and data
- Support memory regions from DTB

### 4. Kernel Heap

Dynamic allocation for kernel data structures:
- Capability nodes (CNode)
- Thread control blocks (TCB)
- Endpoint objects
- Page tables

## Architecture

```
kernel/src/
├── memory/                     # NEW: Memory management subsystem
│   ├── mod.rs                  # Memory subsystem entry point
│   ├── paging.rs               # Page table abstraction
│   ├── frame_allocator.rs     # Physical frame allocator
│   ├── address.rs              # Virtual/Physical address types
│   └── heap.rs                 # Kernel heap allocator
│
└── arch/aarch64/
    ├── mmu.rs                  # NEW: ARM64 MMU setup
    ├── page_table.rs          # NEW: ARM64 page table implementation
    └── registers.rs            # UPDATE: Add MMU control registers
```

## Implementation Plan

### Phase 1: Address Types & Basic Structures
- [ ] Define `PhysAddr` and `VirtAddr` types
- [ ] Define page table entry structures
- [ ] Define page size constants (4KB, 2MB, 1GB)
- [ ] Create memory region tracking

### Phase 2: Frame Allocator
- [ ] Parse memory regions from DTB
- [ ] Track available physical frames
- [ ] Implement frame allocation/deallocation
- [ ] Reserve kernel and boot regions

### Phase 3: Page Tables
- [ ] Implement 4-level page table structure
- [ ] Create page table entry manipulation
- [ ] Implement mapping functions (map_page, unmap_page)
- [ ] Handle different page sizes (4KB, 2MB, 1GB)

### Phase 4: MMU Setup
- [ ] Create identity mapping for kernel
- [ ] Set up TTBR0 (user-space, empty for now)
- [ ] Set up TTBR1 (kernel-space)
- [ ] Configure TCR_EL1
- [ ] Enable MMU (SCTLR_EL1)

### Phase 5: Kernel Heap
- [ ] Choose heap allocator (linked-list or buddy)
- [ ] Implement GlobalAlloc trait
- [ ] Map kernel heap region
- [ ] Enable `alloc` crate features
- [ ] Test with Vec, Box, etc.

### Phase 6: Testing & Validation
- [ ] Verify MMU is enabled
- [ ] Test kernel heap allocation
- [ ] Verify page table walking
- [ ] Test frame allocator
- [ ] Ensure kernel still boots correctly

## Success Criteria

Chapter 2 is complete when:
1. ✅ MMU is enabled with proper page tables
2. ✅ Kernel has working heap allocator
3. ✅ Physical frame allocator works correctly
4. ✅ Kernel virtual address space is properly mapped
5. ✅ Can allocate/deallocate memory dynamically
6. ✅ All existing functionality (console, boot) still works

## Dependencies

### Rust Features
- `alloc` crate for heap allocation
- No external dependencies (pure `no_std`)

### Hardware
- ARM64 MMU (Memory Management Unit)
- TTBR0/TTBR1 translation table base registers
- TCR_EL1 translation control register
- SCTLR_EL1 system control register

## References

### seL4 Memory Management
- seL4 implements untyped memory and typed kernel objects
- Capability-based memory management
- Page table capabilities for mapping

### ARM64 Documentation
- ARM Architecture Reference Manual (ARMv8)
- Chapter D5: The AArch64 Virtual Memory System Architecture
- Page table format and translation process

### Rust Resources
- `alloc` crate documentation
- GlobalAlloc trait implementation
- no_std memory allocators

## Notes

### Differences from Chapter 1
- Chapter 1: Kernel ran with MMU disabled
- Chapter 2: Kernel runs with MMU enabled
- Chapter 1: Static memory layout only
- Chapter 2: Dynamic memory allocation available

### Future Chapters
- Chapter 3: Will use page tables for interrupt handling
- Chapter 4: Will create user-space page tables for processes
- Chapter 5: Will implement capability-based page table operations

## Progress Tracking

### In Progress 🚧
- Phase 5: Kernel heap allocator (next)
- MMU enable (deferred - needs more testing)

### Completed ✅
- ✅ Phase 1: Address Types (PhysAddr, VirtAddr, PageFrameNumber)
- ✅ Phase 2: Frame Allocator (31,726/32,768 frames working)
- ✅ Phase 3: Page Table Types (ARM64 4-level tables, bitflags)
- ✅ Phase 4: MMU Setup (registers configured, page tables created)
- ✅ Fixed DTB parser infinite loop
- ✅ Platform-agnostic memory constants via build-config.toml

### Blocked ⛔
- MMU enable requires additional testing (page fault handling not yet implemented)

### Test Results
```
[memory] Initializing memory subsystem
  RAM:    0x40000000 - 0x48000000 (128MB)
  Kernel: 0x40400000 - 0x40412000 (72KB)
  Frames: 31726/32768 free (123MB usable)

[memory] Setting up page tables and MMU...
  Mapping DTB: 0x40000000 - 0x40200000
  Mapping kernel: 0x40400000 - 0x40412000
  Mapping stack/heap region: 0x40412000 - 0x48000000
  Mapping UART device: 0x9000000
  Root page table at: 0x40412000
  MMU currently enabled: false
```
