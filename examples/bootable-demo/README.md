# KaaL Bootable Demo - Phase 1

Demonstrates KaaL's Phase 1 functionality with capability broker integration.

## What This Demonstrates

**This is the reference implementation for KaaL Phase 1** - it actually uses KaaL's modules:

✅ **kaal_cap_broker** - Capability management
✅ **BootInfo parsing** - Resource discovery
✅ **MMIO utilities** - Memory-mapped I/O
✅ **IRQ allocation** - Interrupt handling
✅ **Component spawning** - Process management
✅ **Heap allocator** - Dynamic memory

## Quick Build & Test

```bash
./tools/build-bootimage.sh \
    --project examples/bootable-demo \
    --lib libkaal_bootable_demo.a \
    --clean --test
```

## Expected Output

```
═══════════════════════════════════════════════════════════
  KaaL Phase 1 Bootable Demo v0.1.0
  Booted with Rust Elfloader + seL4 Microkernel
═══════════════════════════════════════════════════════════

Phase 1: Testing KaaL Core Infrastructure
------------------------------------------

[1/4] Capability Broker - Resource Management
  ✓ BootInfo parsing (cap_broker::bootinfo)
  ✓ MMIO mapping (cap_broker::mmio)
  ✓ IRQ allocation (cap_broker::irq)
  ✓ VSpace management (cap_broker::vspace)
  ✓ TCB management (cap_broker::tcb)
  ✓ Component spawning (cap_broker::component)

[2/4] Memory Management Utilities
  ✓ Page alignment: 0x12345 → 0x13000
  ✓ Pages needed for 8KB: 2 pages

[3/4] Heap Allocator (256KB bump allocator)
  ✓ Vector allocation successful
  ✓ Test data: [0x42, 0x13, 0x37]

[4/4] Platform Configuration
  ✓ Architecture: ARM64 (aarch64)
  ✓ Microkernel: seL4 v13.0.0
  ✓ Platform: QEMU ARM virt (Cortex-A53)
  ✓ Page size: 4096 bytes
  ✓ Elfloader: Rust-based (Phase 1)

All Phase 1 infrastructure tests passed!

═══════════════════════════════════════════════════════════
  Phase 1 Demo Complete - Entering Idle Loop
═══════════════════════════════════════════════════════════
```

## Architecture

```
┌─────────────────────────────────────────┐
│       Rust Elfloader (Phase 1)          │
│  - Loads seL4 kernel                    │
│  - Loads root task                      │
│  - Sets up MMU & page tables            │
│  - DTB parsing                          │
└───────────────┬─────────────────────────┘
                ↓
┌─────────────────────────────────────────┐
│       seL4 Microkernel v13.0.0          │
│  - Capability system                    │
│  - IPC                                  │
│  - Scheduling                           │
└───────────────┬─────────────────────────┘
                ↓
┌─────────────────────────────────────────┐
│   Root Task (bootable-demo)             │
│  ┌────────────────────────────────────┐ │
│  │  kaal_cap_broker                   │ │
│  │  - BootInfo parsing                │ │
│  │  - MMIO mapping                    │ │
│  │  - IRQ allocation                  │ │
│  │  - VSpace management               │ │
│  │  - TCB management                  │ │
│  │  - Component spawning              │ │
│  └────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

## Code Structure

- [src/lib.rs](src/lib.rs) - Main implementation
  - `_start()` - Entry point from seL4
  - `demo_phase1_functionality()` - Tests all Phase 1 modules
  - `BumpAllocator` - Simple heap for `alloc` support
  - Debug output utilities

- [Cargo.toml](Cargo.toml) - Dependencies
  - `kaal_cap_broker` - The actual KaaL module we're testing

## See Also

- [BUILD_BOOTABLE_IMAGE.md](../../BUILD_BOOTABLE_IMAGE.md) - Full build documentation
- [runtime/cap_broker](../../runtime/cap_broker/) - Capability Broker source
- [runtime/elfloader](../../runtime/elfloader/) - Elfloader source
