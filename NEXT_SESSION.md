# Next Session Start Point

## Current Status (Session End)

**Git commits**: 54 total (main branch ahead by 54)
- Last commit: `b49790a` - Reverted MMU enable (was premature)
- Working tree: Clean

## What We Learned

**CRITICAL LESSON**: Don't enable MMU without proper exception handling!
- Attempted to enable MMU → system hung
- Root cause: Need context save/restore FIRST for exception handling
- MMU enable will trigger exceptions (page faults etc) - must handle them

## Prerequisites for MMU Enable (IN ORDER!)

### ✅ DONE:
1. Exception vector table (16 entries) - COMPLETE
2. TrapFrame structure (36 × 64-bit registers) - COMPLETE
3. Syscall dispatcher module - COMPLETE
4. Exception syndrome registers (ESR/FAR/ELR/SPSR/VBAR) - COMPLETE

### 🚧 TODO (Next Session):

**Step 1**: Add context save/restore assembly (1-2 hours)
- File: `kernel/src/arch/aarch64/exception.rs`
- Add SAVE_CONTEXT macro (saves x0-x30 + special regs to stack)
- Add RESTORE_CONTEXT macro (restores from stack + eret)
- Insert macros after line 118 (after vector table)

**Step 2**: Wire TrapFrame to exception handlers (1 hour)
- Change exception vector stubs to call SAVE_CONTEXT
- Update Rust handlers to take `&mut TrapFrame` parameter
- Call RESTORE_CONTEXT before returning

**Step 3**: Test with deliberate exception (30 min)
- Add test code to trigger data abort
- Verify trap frame is populated correctly
- Check exception handler receives valid TrapFrame

**Step 4**: Implement page fault handler (2-3 hours)
- Decode FAR_EL1 (fault address)
- Decode DFSC/IFSC (fault status code)
- Print detailed fault information
- Prepare for demand paging (future)

**Step 5**: Enable MMU (1 hour)
- NOW it's safe to enable MMU
- Any exceptions will be handled properly
- Test that kernel continues after MMU enable

## Files to Modify

1. `kernel/src/arch/aarch64/exception.rs`
   - Add SAVE_CONTEXT/RESTORE_CONTEXT macros after line 118
   - Update all exception handlers to use TrapFrame

2. `kernel/src/boot/mod.rs`
   - Enable MMU after exception handling complete (line ~186)

3. `kernel/src/syscall/mod.rs`
   - Already created, wire to exception handler

## Key Code Snippets

### Context Save Macro Template
```assembly
.macro SAVE_CONTEXT
    sub sp, sp, #288        // Allocate trap frame
    stp x0, x1, [sp, #0]   // Save registers
    // ... save x2-x30 ...
    mrs x0, sp_el0          // Save special regs
    mrs x1, elr_el1
    mrs x2, spsr_el1
    mrs x3, esr_el1
    mrs x4, far_el1
    stp x0, x1, [sp, #248]
    stp x2, x3, [sp, #264]
    str x4, [sp, #280]
    mov x0, sp              // Pass TrapFrame* in x0
.endm
```

### Exception Handler Update
```rust
// BEFORE:
extern "C" fn exception_curr_el_spx_sync() {
    kprintln!("[exception] Sync");
    panic!();
}

// AFTER:
extern "C" fn exception_curr_el_spx_sync(tf: &mut TrapFrame) {
    kprintln!("[exception] Sync at ELR={:#x}", tf.elr_el1);

    if tf.is_syscall() {
        crate::syscall::handle_syscall(tf);
        // Return normally, context will be restored
    } else if tf.is_data_abort() {
        handle_page_fault(tf);
    } else {
        panic!("Unhandled exception");
    }
}
```

## Current Chapter Status

**Chapter 2**: COMPLETE (MMU pending)
**Chapter 3**: 50% complete
  - Phase 1: Exception vectors ✅
  - Phase 2: Trap frame ✅
  - Phase 3: Context save/restore ⬜ ← START HERE
  - Phase 4: Syscall/page fault ⬜
  - Phase 5: MMU enable ⬜

## Estimated Time Remaining

- Context save/restore: 1-2 hours
- Wire to handlers: 1 hour
- Test: 30 min
- Page fault handler: 2-3 hours
- MMU enable: 1 hour
- **Total: 5-8 hours to complete Chapter 3**

## Commands to Run Next Session

```bash
# Check status
git log --oneline -5
git status

# Kill any background QEMU
killall -9 qemu-system-aarch64

# Edit exception.rs
vim kernel/src/arch/aarch64/exception.rs

# Build and test
./build.sh --platform qemu-virt
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
    -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
```

## Success Criteria

Chapter 3 is COMPLETE when:
- [x] Exception vector table installed
- [x] TrapFrame structure defined
- [ ] Context save/restore working
- [ ] Exception handlers receive TrapFrame
- [ ] Page faults handled gracefully
- [ ] **MMU ENABLED and working!**

After Chapter 3, we're unblocked for Chapters 4-8!
