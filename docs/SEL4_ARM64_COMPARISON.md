# seL4 vs KaaL: ARM64 Context Switching Comparison

## Executive Summary

After analyzing seL4's implementation and ARM64 architecture manual, comparing with KaaL's current approach.

## Key Findings from seL4

### Register Management
seL4 categorizes registers into:
1. **Message Registers** (X2-X5) - For IPC
2. **Frame Registers** - Essential for thread state:
   - FaultIP (ELR_EL1)
   - SP_EL0
   - SPSR_EL1
   - X0-X8, X16-X18, X29-X30
3. **General Purpose** - Saved but not in fast path:
   - X9-X15, X19-X28
   - TPIDR_EL0, TPIDRRO_EL0

### Exception Handling
- Uses macro-based assembly for consistency
- Systematic saving of registers in pairs (stp instructions)
- Differentiates between hypervisor and non-hypervisor modes
- Special handling for synchronous vs async exceptions

## ARM64 Architecture Requirements

### Hardware Automatic Actions on Exception Entry
1. ✅ Save current processor state → SPSR_ELx
2. ✅ Save return address → ELR_ELx
3. ✅ Transfer control to exception vector

### Software Manual Requirements
1. ❌ Save general-purpose registers (X0-X30)
2. ❌ Save SP_EL0 (user stack pointer)
3. ❌ Read ESR_EL1 (exception syndrome)
4. ❌ Read FAR_EL1 (fault address)
5. ⚠️  **Save TTBR0_EL1** (if switching address spaces)

### Exception Return Requirements
1. ✅ Restore general-purpose registers
2. ✅ Restore SP_EL0
3. ✅ Write ELR_EL1 (return address)
4. ✅ Write SPSR_EL1 (processor state)
5. ⚠️  **Restore TTBR0_EL1** (if different from current)
6. ⚠️  **TLB Invalidation** (if TTBR0 changed)
7. ✅ Execute ERET instruction

## TLB Invalidation Requirements (ARM Architecture)

### When TLB Invalidation is MANDATORY
1. **After changing TTBR0/TTBR1**:
   ```asm
   msr ttbr0_el1, x0
   tlbi vmalle1is      // Invalidate ALL TLB entries for EL1
   dsb ish             // Ensure TLB invalidation completes
   isb                 // Synchronize instruction fetch
   ```

2. **After modifying page tables**:
   - Must invalidate TLB entries for modified VAs
   - Use `tlbi vaae1is` for specific VA
   - Or `tlbi vmalle1is` for all entries

3. **Before enabling MMU**:
   - TLB must be clean before MMU enable
   - Use `tlbi vmalle1is`

### When TLB Invalidation is OPTIONAL
1. **On initial boot** (TLB empty)
2. **If using ASID tags** (Address Space ID) to disambiguate
3. **If you KNOW TLB doesn't have conflicting entries**

## KaaL vs seL4: Key Differences

### What KaaL Does CORRECTLY
1. ✅ Saves all general-purpose registers (X0-X30)
2. ✅ Saves SP_EL0, ELR_EL1, SPSR_EL1
3. ✅ Saves ESR_EL1, FAR_EL1 for fault analysis
4. ✅ Saves TTBR0_EL1 to TrapFrame (offset 288)
5. ✅ Restores TTBR0_EL1 from TrapFrame on exception return
6. ✅ Uses ISB for context synchronization

### What KaaL Does DIFFERENTLY (Potential Issues)
1. ⚠️  **TLB Invalidation on Exception Return**:
   - **KaaL**: Added `tlbi vmalle1is` after restoring TTBR0 (commit a408c3c)
   - **Impact**: Causes notification_create() to fail!
   - **Hypothesis**: TLB flush breaks memory access to stack/kernel data

2. ⚠️  **Context Switch Mechanism**:
   - **seL4**: Uses dedicated context switch code path
   - **KaaL**: Swaps TrapFrame directly in sys_wait(), relies on exception return
   - **Potential Issue**: TrapFrame on stack may be corrupted or misaligned

3. ⚠️  **Stack Management**:
   - **seL4**: Uses `lsp_i` macro to set up kernel stack pointer carefully
   - **KaaL**: Relies on exception handler's automatic stack setup
   - **Risk**: Stack corruption could corrupt TrapFrame

## Critical Questions for KaaL

### Q1: Is TLB invalidation breaking memory access?
**Hypothesis**: The TLB flush at exception return invalidates kernel mappings needed to access the stack where TrapFrame lives.

**Evidence**:
- BEFORE TLB fix: process restarts but kernel works
- AFTER TLB fix: notification_create() fails immediately

**Possible Cause**:
- Our unified page table has kernel mappings in user PT
- TLB flush removes cached translations for kernel .text/.data
- Next kernel instruction faults because TLB entry gone
- Fault happens before reaching userspace

### Q2: Should we flush TLB on exception return at all?
**ARM Manual says**: Flush TLB after changing TTBR0

**BUT**: In our case:
- Exception entry: TTBR0 is NOT changed (we keep user PT)
- Exception exit: TTBR0 IS restored from TrapFrame
- **If TTBR0 didn't change**: No TLB flush needed!
- **If TTBR0 changed**: Must flush TLB

**Solution**: Only flush TLB if TTBR0 actually changed:
```asm
mrs x14, ttbr0_el1       // Read current TTBR0
ldr x13, [sp, #288]      // Load saved TTBR0
cmp x13, x14             // Compare
b.eq skip_tlb_flush      // Skip if same
msr ttbr0_el1, x13       // Restore TTBR0
tlbi vmalle1is           // Flush TLB
dsb ish
b done_ttbr0
skip_tlb_flush:
msr ttbr0_el1, x13       // Restore anyway (cheap)
done_ttbr0:
isb
```

### Q3: Is sys_wait's TrapFrame swapping safe?
**Current approach**:
```rust
*tf = *next_tcb.context();  // Copy entire TrapFrame struct
```

**Risks**:
1. `tf` is a mutable reference to stack memory
2. Copying 296 bytes might be non-atomic
3. If interrupt occurs during copy, state corrupted
4. Stack alignment requirements (16-byte aligned?)

**seL4 approach**: Probably does atomic pointer swap or disables interrupts

## Recommendations

### Immediate Actions
1. **REVERT commit a408c3c** (TLB invalidation) - it makes things worse
2. **Add conditional TLB flush** - only if TTBR0 changed
3. **Verify stack alignment** - TrapFrame must be 16-byte aligned
4. **Add interrupt disable** around TrapFrame swap in sys_wait

### Investigation Priorities
1. Check if sys_wait runs with interrupts disabled
2. Verify TrapFrame is properly aligned on stack
3. Test if unconditional TLB flush breaks kernel mappings
4. Compare with working context from root_task

### Long-term Improvements
1. Consider dedicated context switch path (like seL4)
2. Use ASID (Address Space ID) to avoid full TLB flushes
3. Implement per-VA TLB invalidation for efficiency
4. Add compile-time assertions for TrapFrame layout

## References

- seL4 GitHub: https://github.com/seL4/seL4
- ARM Architecture Reference Manual for ARMv8-A
- seL4 Whitepaper: Performance characteristics
- ARM Cortex-A Programming Guide: TLB management

## Conclusion

The context switching bug likely stems from:
1. **Aggressive TLB invalidation** breaking kernel access
2. **Non-atomic TrapFrame swapping** causing corruption
3. **Stack or alignment issues** with saved state

The fix is NOT to add more TLB invalidation, but to:
- Make TLB flush conditional (only when needed)
- Ensure TrapFrame operations are atomic
- Verify memory safety of context switch path
