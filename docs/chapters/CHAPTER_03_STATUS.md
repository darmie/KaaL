# Chapter 3: Exception Handling & Syscalls - Status

**Status**: 🚧 IN PROGRESS
**Started**: 2025-10-12
**Target Completion**: TBD

## Objectives

1. ⬜ Set up ARM64 exception vector table (16 entries)
2. ⬜ Implement trap frame (context save/restore)
3. ⬜ Handle synchronous exceptions (syscalls, page faults)
4. ⬜ Implement syscall dispatcher
5. ⬜ Create page fault handler
6. ⬜ Enable MMU (unblock Chapter 2!)

## Overview

Chapter 3 implements exception handling infrastructure, enabling user-to-kernel transitions and proper fault handling. This is critical for:
- Syscall interface (user programs calling kernel services)
- Page fault handling (demand paging, copy-on-write)
- Enabling MMU safely with fault recovery
- Foundation for interrupt handling (Chapter 6)

## ARM64 Exception Model

### Exception Levels (EL)
```
EL3: Secure Monitor (Firmware)
EL2: Hypervisor (Not used in KaaL)
EL1: Kernel (KaaL runs here)
EL0: User space (Future root task)
```

### Exception Vector Table (16 Entries)
```
Current EL with SP_EL0:
  0x000: Synchronous
  0x080: IRQ
  0x100: FIQ
  0x180: SError

Current EL with SP_ELx:
  0x200: Synchronous
  0x280: IRQ
  0x300: FIQ
  0x380: SError

Lower EL (AArch64):
  0x400: Synchronous  ← Syscalls come here
  0x480: IRQ
  0x500: FIQ
  0x580: SError

Lower EL (AArch32):
  0x600: Synchronous
  0x680: IRQ
  0x700: FIQ
  0x780: SError
```

### Trap Frame (Saved Context)
```rust
#[repr(C)]
pub struct TrapFrame {
    // General purpose registers
    pub x0: u64,   // Also syscall arg 0 / return value
    pub x1: u64,   // Syscall arg 1
    pub x2: u64,   // Syscall arg 2
    pub x3: u64,   // Syscall arg 3
    pub x4: u64,   // Syscall arg 4
    pub x5: u64,   // Syscall arg 5
    pub x6: u64,
    pub x7: u64,
    // x8-x29 ...
    pub x30: u64,  // Link register (LR)

    // Exception state
    pub elr_el1: u64,  // Exception return address
    pub spsr_el1: u64, // Saved processor state
    pub esr_el1: u64,  // Exception syndrome
    pub far_el1: u64,  // Fault address
    pub sp_el0: u64,   // User stack pointer
}
```

## Implementation Plan

### Phase 1: Exception Vector Table ✅ IN PROGRESS
- [ ] Create exception.rs with vector table assembly
- [ ] Implement 16 exception entry points
- [ ] Set VBAR_EL1 to point to vector table
- [ ] Add minimal stubs for each exception type

### Phase 2: Trap Frame
- [ ] Define TrapFrame structure
- [ ] Implement context save (assembly)
- [ ] Implement context restore (assembly)
- [ ] Create Rust handler entry points

### Phase 3: Synchronous Exception Handlers
- [ ] Decode ESR_EL1 (Exception Syndrome Register)
- [ ] Handle syscalls (EC = 0x15 - SVC from AArch64)
- [ ] Handle data aborts (EC = 0x24/0x25)
- [ ] Handle instruction aborts (EC = 0x20/0x21)

### Phase 4: Syscall Dispatcher
- [ ] Define syscall numbers (seL4-compatible)
- [ ] Implement syscall dispatch table
- [ ] Create stub syscall handlers
- [ ] Pass arguments via x0-x5, return via x0

### Phase 5: Page Fault Handler
- [ ] Parse FAR_EL1 (Fault Address Register)
- [ ] Parse DFSC/IFSC (fault status codes)
- [ ] Handle translation faults
- [ ] Handle permission faults
- [ ] Panic with detailed fault info (for now)

### Phase 6: Enable MMU
- [ ] Verify all exception handlers working
- [ ] Enable MMU in SCTLR_EL1
- [ ] Test with deliberate page fault
- [ ] Verify syscall from EL0 (future)

## Success Criteria

Chapter 3 is complete when:
1. ✅ Exception vector table installed and working
2. ✅ Can handle synchronous exceptions without crashing
3. ✅ Syscall interface functional (basic test)
4. ✅ Page faults handled gracefully with debug info
5. ✅ MMU enabled successfully
6. ✅ All existing functionality still works

## Files to Create/Modify

```
kernel/src/
├── arch/aarch64/
│   ├── exception.rs         # NEW: Exception vectors + handlers
│   ├── context.rs           # NEW: Trap frame + save/restore
│   └── registers.rs         # UPDATE: Add ESR_EL1, FAR_EL1, etc.
│
└── syscall/
    ├── mod.rs               # NEW: Syscall dispatcher
    └── numbers.rs           # NEW: Syscall number definitions
```

## References

### ARM Architecture Manual
- Exception levels and exception handling
- ESR_EL1 encoding (exception syndrome)
- FAR_EL1 (fault address register)
- VBAR_EL1 (vector base address)

### seL4 Syscall Interface
- Syscall numbers and conventions
- IPC syscalls (Send, Recv, Call, Reply)
- Object invocation model

## Progress Tracking

### In Progress 🚧
- Phase 1: Exception vector table (starting now)

### Blocked ⛔
- None yet

## Next Steps

Starting with exception vector table implementation...

---

**Last Updated**: 2025-10-12
**Status**: 🚧 IN PROGRESS
