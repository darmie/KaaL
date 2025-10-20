# Chapter 2: Memory Management & MMU - Status

**Status**: ✅ COMPLETE
**Started**: 2025-10-12
**Completed**: 2025-10-13

## Objectives

1. ✅ Implement physical memory frame allocator
2. ✅ Set up 4-level ARM64 page tables (L0-L3)
3. ✅ Enable MMU with virtual memory
4. ✅ Implement kernel heap allocator
5. ✅ Integrate exception handling with MMU

## Progress Tracking

### Completed ✅

- [x] Physical frame allocator with bitmap tracking
- [x] Page table structures (L0-L3) with walking
- [x] Identity mapping for kernel bootstrap
- [x] 2MB block entries for efficient mappings
- [x] 4KB page entries for fine-grained control
- [x] MMU enable with proper barriers and TLB invalidation
- [x] Exception vector table installed before MMU enable
- [x] Kernel heap (1MB linked-list allocator)
- [x] Rust alloc integration (Box, Vec working)
- [x] Debug page table walker for verification

## File Structure Created

```
kernel/src/
├── memory/
│   ├── mod.rs              # ✅ Memory subsystem
│   ├── address.rs          # ✅ VirtAddr/PhysAddr types
│   ├── frame.rs            # ✅ Physical frame allocator
│   ├── paging.rs           # ✅ Page table management
│   └── heap.rs             # ✅ Kernel heap allocator
├── arch/aarch64/
│   ├── page_table.rs       # ✅ ARM64 page table types
│   ├── mmu.rs              # ✅ MMU initialization
│   └── exception.rs        # ✅ Exception handlers (integrated)
└── boot/
    └── mod.rs              # ✅ Updated for MMU enable

kernel/
├── kernel.ld               # ✅ Linker script (BSS, stack)
└── .cargo/config.toml      # ✅ Cargo configuration
```

## Achievements

### 1. Physical Memory Manager ✅

**Implementation**: Bitmap-based frame allocator
- Tracks 32,768 frames (128MB RAM)
- 31,466 frames free after kernel and page tables
- O(1) allocation and deallocation
- Proper alignment and size validation

**Files**:
- [kernel/src/memory/frame.rs](../../kernel/src/memory/frame.rs)

### 2. 4-Level ARM64 Page Tables ✅

**Implementation**: Complete translation infrastructure
- L0-L3 page table walking with automatic allocation
- 2MB block entries for efficient large mappings
- 4KB page entries for fine-grained control
- Identity mapping for kernel bootstrap
- Page table entry flags (VALID, AF, UXN, PXN, etc.)

**Key Features**:
- `PageMapper::map()` - Create virtual memory mappings
- `PageMapper::translate()` - Walk page tables to resolve addresses
- `PageMapper::debug_walk()` - Inspect translations for debugging
- `identity_map_region()` - Create 1:1 virt=phys mappings

**Files**:
- [kernel/src/arch/aarch64/page_table.rs](../../kernel/src/arch/aarch64/page_table.rs)
- [kernel/src/memory/paging.rs](../../kernel/src/memory/paging.rs)

### 3. MMU Enable ✅ **MAJOR MILESTONE**

**Implementation**: Successfully enabled with proper barriers
- Exception handlers installed BEFORE MMU enable (critical!)
- TLB invalidation with full system barriers (`dsb sy`, `isb`)
- MMU-only enable (caches disabled, following seL4 pattern)
- Fixed block entry encoding (TABLE_OR_PAGE bit handling)

**Critical Fixes**:
1. **PXN Bit**: Created KERNEL_RWX flags with PXN=0 for execution
2. **Exception Timing**: Moved exception::init() before init_mmu()
3. **Block Encoding**: Clear TABLE_OR_PAGE bit for L1/L2 blocks

**Files**:
- [kernel/src/arch/aarch64/mmu.rs](../../kernel/src/arch/aarch64/mmu.rs)

### 4. Kernel Heap ✅

**Implementation**: 1MB linked-list allocator
- Box and Vec allocations working
- Alloc trait integrated for Rust collections
- `#[global_allocator]` configured
- `#[alloc_error_handler]` for OOM panics

**Test Output**:
```
[memory] Initializing kernel heap...
  Heap size: 1024 KB (1048576 bytes free)

[test] Testing heap allocator with Box and Vec...
  ✓ Box allocation: Hello from Box on the heap!
  ✓ Vec allocation: Hello World from Vec!
  Heap after allocations: 1048496 bytes free (consumed 80 bytes)
```

**Files**:
- [kernel/src/memory/heap.rs](../../kernel/src/memory/heap.rs)

## Technical Challenges Solved

### Challenge 1: PXN Bit Preventing Execution

**Problem**: Kernel code pages had PXN=1 (Privileged Execute Never), preventing EL1 code execution after MMU enable.

**Root Cause**: Used KERNEL_DATA flags which include both UXN=1 and PXN=1.

**Solution**: Created KERNEL_RWX flags with:
- VALID=1, TABLE_OR_PAGE=1
- AP_RW_EL1 (read-write for EL1)
- ACCESSED=1 (AF bit set)
- UXN=1 (prevent user execution)
- **PXN=0** (allow privileged execution)

**Impact**: MMU could enable, but CPU couldn't execute the next instruction. System would hang silently.

**Debug Method**: Page table walker showed `PXN=1` on kernel code pages.

### Challenge 2: Exception Handlers Not Ready

**Problem**: `exception::init()` was called in Chapter 3, AFTER MMU enable in Chapter 2.

**Root Cause**: MMU enable can trigger exceptions (translation faults, alignment faults, etc.), but handlers weren't installed yet.

**Symptom**: Infinite prefetch abort loop at PC 0x200 (VBAR_EL1 + synchronous exception offset).

**Solution**: Moved exception::init() to run BEFORE init_mmu() in boot sequence.

**Impact**: System entered infinite exception loop, unable to handle the fault.

**Debug Method**: QEMU `-d int,mmu` showed repeated prefetch aborts at PC 0x200.

### Challenge 3: Block Entry Encoding Wrong

**Problem**: 2MB block entries at L2 had TABLE_OR_PAGE=1, causing them to be interpreted as table descriptors instead of block descriptors.

**Root Cause**: ARM64 descriptor bit 1 encoding:
- **Table descriptor**: bit 1 = 1
- **Block descriptor** (L1/L2): bit 1 = 0
- **Page descriptor** (L3): bit 1 = 1

Our KERNEL_RWX flags had TABLE_OR_PAGE=1, which works for L3 pages but is wrong for L1/L2 blocks.

**Symptom**: Translation fault at 0x40600000, L2 entry showed "TABLE" pointing to 0x40600000 itself.

**Solution**: Modified `PageMapper::map()` to clear TABLE_OR_PAGE bit for block entries:
```rust
let mut entry_flags = flags;
if page_size != PageSize::Size4KB {
    // This is a block entry (1GB or 2MB), clear TABLE_OR_PAGE bit
    entry_flags.remove(PageTableFlags::TABLE_OR_PAGE);
}
```

**Impact**: L2 entry was treated as pointer to next level table at physical address 0x40600000, which wasn't a page table at all. Led to translation fault.

**Debug Method**: Page table walker showed L2 entry as "TABLE" with next table address = 0x40600000 (same as the block address). After fix, shows "BLOCK" correctly.

## Debug Tools Created

### 1. Page Table Walker (`debug_walk`)

**Purpose**: Inspect page table translations before MMU enable

**Features**:
- Walks all 4 levels (L0-L3) for a given virtual address
- Decodes entry flags (VALID, AF, UXN, PXN, memory attributes)
- Shows physical address resolution
- Distinguishes between table, block, and page entries

**Example Output**:
```
[walk] Translating 0x40400000:
  L0 [0]: 0x0000000040517003
    -> VALID | TABLE | AF=0 | UXN=0 | PXN=0
    -> Next table at 0x40517000
  L1 [1]: 0x0000000040518003
    -> VALID | TABLE | AF=0 | UXN=0 | PXN=0
    -> Next table at 0x40518000
  L2 [2]: 0x0000000040519003
    -> VALID | TABLE | AF=0 | UXN=0 | PXN=0
    -> Next table at 0x40519000
  L3 [0]: 0x0040000040400703
    -> VALID | TABLE | AF=1 | UXN=1 | PXN=0
    -> Translates to 0x40400000 (page @ 0x40400000 + offset 0x0)
```

### 2. QEMU Debug Mode

**Command**: `qemu-system-aarch64 -d int,mmu ...`

**Use Case**: Caught infinite exception loop at PC 0x200

**Output**: Showed repeated prefetch aborts with ESR and FAR values, revealing that exception handlers weren't ready.

## Testing Criteria ✅ PASSED

Expected output when Chapter 2 is complete:

```
═══════════════════════════════════════════════════════════
  Chapter 2: Memory Management
═══════════════════════════════════════════════════════════

[memory] Initializing memory subsystem
  RAM:    0x40000000 - 0x48000000 (128MB)
  Kernel: 0x40400000 - 0x40515000 (1112KB)
  Frames: 31467/32768 free (122MB usable)

[test] Testing frame allocator...
  Allocated frame: PFN(263445) @ 0x40515000
  Allocated frame: PFN(263446) @ 0x40516000
  Deallocated both frames
  Final stats: 31467/32768 frames free

[memory] Setting up page tables and MMU...
  Mapping DTB: 0x40000000 - 0x40200000
  Mapping kernel: 0x40400000 - 0x40515000
  Mapping stack/heap region: 0x40515000 - 0x48000000
  Mapping UART device: 0x9000000
  Root page table at: 0x40515000

[exception] Installing exception vector table at 0x0000000040400800
[exception] Exception handlers installed
  Enabling MMU...
  MMU enabled: true
  ✓ MMU enabled successfully with virtual memory!

[memory] Initializing kernel heap...
  Heap size: 1024 KB (1048576 bytes free)

[test] Testing heap allocator with Box and Vec...
  ✓ Box allocation: Hello from Box on the heap!
  ✓ Vec allocation: Hello World from Vec!
  Heap after allocations: 1048496 bytes free (consumed 80 bytes)

═══════════════════════════════════════════════════════════
  Chapter 2: COMPLETE ✓
═══════════════════════════════════════════════════════════
```

**Status**: ✅ PASSED - MMU enabled and heap working!

## Memory Layout (After MMU Enable)

### Virtual Memory Regions

| Region | Virtual Address | Size | Permissions | Description |
|--------|----------------|------|-------------|-------------|
| DTB | 0x40000000 - 0x40200000 | 2MB | RW, NX | Device tree blob |
| Kernel | 0x40400000 - 0x40515000 | ~1.1MB | RWX | Kernel code/data/BSS |
| Heap/Stack | 0x40515000 - 0x48000000 | ~123MB | RW, NX | Dynamic allocation |
| UART | 0x09000000 - 0x09001000 | 4KB | RW, Device | MMIO device |

### Page Table Structure

```
Root (L0) @ 0x40515000
├── [0] -> L1 @ 0x40517000
│   ├── [0] -> L2 @ 0x4051a000 (UART region)
│   └── [1] -> L2 @ 0x40518000 (kernel/heap region)
│       ├── [2] -> L3 @ 0x40519000 (kernel 4KB pages)
│       └── [3] -> 2MB block @ 0x40600000 (heap region)
```

## Key Learnings

1. **ARM64 Descriptor Encoding**: Block vs page descriptors have different bit 1 values
2. **Exception Readiness**: Handlers must be installed before any operation that might fault
3. **seL4 Pattern**: Enable MMU first (caches disabled), then enable caches after verification
4. **Debug Tools**: Page table walkers and QEMU debug mode are essential for low-level issues
5. **Barrier Sequence**: `dsb sy` ensures page table writes are visible, `tlbi` invalidates TLB, `isb` synchronizes instruction fetch

## Next Steps

Chapter 2 is complete! Move on to Chapter 3: Exception Handling & Syscalls

Key features remaining in Chapter 3:
- Test exception handling with deliberate faults
- Implement syscall dispatcher
- Implement page fault handler (decode FAR/ESR)
- Add context switching infrastructure

## References

- [MICROKERNEL_CHAPTERS.md](../MICROKERNEL_CHAPTERS.md) - Full development roadmap
- [ARM Architecture Reference Manual (ARMv8-A)](https://developer.arm.com/documentation/ddi0487/latest)
- [seL4 ARM64 MMU Setup](https://github.com/seL4/seL4/blob/master/src/arch/arm/64/machine/capdl.c)
- ARM Trusted Firmware MMU initialization

---

**Last Updated**: 2025-10-13
**Status**: ✅ COMPLETE - MMU Fully Operational!
