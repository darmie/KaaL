# Context Switching Bug Investigation

## Problem Statement

system_init process restarts from entry point (`_start`) after calling `wait()` instead of resuming from the saved program counter.

## Symptoms

1. system_init banner appears twice
2. Second execution: `notification_create()` fails (notification already exists)
3. system_init loops yielding instead of blocking in event loop
4. Never reaches "Entering event loop" message

## Investigation Timeline

### Phase 1: Initial Diagnosis
- **Suspected**: Context switching bug causing ELR corruption
- **Evidence**: system_init prints banner twice, implying restart from 0x200000

### Phase 2: Deep Dive into Context Switching
- Verified exception handler correctly saves/restores TTBR0
- Confirmed `sys_wait()` correctly saves context with ELR=0x201230
- Found `sys_wait()` switches to test_minimal (TCB 0x4049f000) with ELR=0x200000

### Phase 3: Page Table Bug Discovery
- Discovered inline assembly bug in root-task's `sys_process_create` wrapper
- Register allocation conflict caused PT addresses to be corrupted
- **FIXED** (commit f2d8f4c): Changed to positional operands

### Phase 4: TLB Investigation
- Found exception handler missing TLB invalidation after TTBR0 restore
- **ATTEMPTED FIX** (commit a408c3c): Added `tlbi vmalle1is` after restoring TTBR0
- **REGRESSION**: This broke notification_create entirely!

### Phase 5: Root Cause Clarification
- **KEY INSIGHT**: Comparing logs before/after TLB fix:
  - BEFORE TLB fix: system_init runs TWICE (banner appears twice)
  - AFTER TLB fix: system_init runs ONCE but notification_create fails

- **Conclusion**: TLB invalidation made things WORSE, not better
- Original bug remains: process restarts from entry point after blocking

## Technical Details

### What Works Correctly
1. ✅ Exception handler saves TTBR0 to TrapFrame (offset 288)
2. ✅ Exception handler restores TTBR0 from TrapFrame
3. ✅ `sys_wait()` saves current thread context: `*(*current).context_mut() = *tf`
4. ✅ `sys_wait()` swaps TrapFrame: `*tf = *next_tcb.context()`
5. ✅ Inline assembly register allocation fixed

### What's Broken
1. ❌ Process restarts from entry point instead of resuming after wait()
2. ❌ TLB invalidation in exception return path breaks memory access
3. ❌ Unknown mechanism causing ELR to revert to 0x200000

## Evidence from Logs

### Old Log (context-debug.log - 12:08, BEFORE fixes)
```
System Init Component v0.1.0        ← First run
[system_init] Creating notification for event handling...
[sys_wait output showing context switch]
System Init Component v0.1.0        ← Second run (BUG!)
[system_init] Creating notification for event handling...
[system_init] ERROR: Failed to create notification!
```

### New Log (tlb-fix-test.log - 13:07, AFTER TLB fix)
```
System Init Component v0.1.0        ← Only one run
[system_init] Creating notification for event handling...
[system_init] ERROR: Failed to create notification!  ← Fails immediately!
```

## Hypotheses

### Hypothesis 1: Scheduler picks wrong TCB
- Scheduler might be selecting system_init's TCB again with stale ELR
- Need to verify scheduler selection logic and thread states

### Hypothesis 2: TrapFrame corruption
- Something corrupts the saved TrapFrame after `sys_wait()` saves it
- TCB's context might be overwritten between save and restore

### Hypothesis 3: Exception return path issue
- Exception handler might not be using the swapped TrapFrame correctly
- TLB invalidation interferes with TrapFrame access on stack

## Next Steps

1. **REVERT** TLB invalidation commit (a408c3c) - it makes things worse
2. **INVESTIGATE** scheduler logic:
   - Which TCB does scheduler pick after system_init blocks?
   - What is that TCB's ELR value?
   - Is the TCB state correctly set to Blocked?
3. **VERIFY** TrapFrame swapping in sys_wait:
   - Add debug to confirm TrapFrame is on stack, not copied
   - Verify exception handler uses the modified stack TrapFrame
4. **CHECK** for TrapFrame overwrites:
   - Look for places that might write to the exception stack
   - Verify no buffer overflows or stack corruption

## Code Locations

- Exception handler: `kernel/src/arch/aarch64/exception.rs:187-255`
- sys_wait implementation: `kernel/src/syscall/mod.rs:1713-1771`
- Scheduler: `kernel/src/scheduler/mod.rs`
- TCB: `kernel/src/objects/tcb.rs`
- TrapFrame: `kernel/src/arch/aarch64/context.rs`

## Commit History

- `ae27953`: Test component added
- `59f2d80`: Delegated spawning implemented
- `5fb3734`: Investigation started, page table bug identified
- `f2d8f4c`: ✅ Fixed inline assembly register conflict
- `a408c3c`: ⚠️  Added TLB invalidation (REGRESSION)

## Key Files Modified

- `runtime/root-task/src/main.rs`: Fixed inline assembly
- `kernel/src/arch/aarch64/exception.rs`: Added TLB invalidation (needs revert)
- `kernel/src/syscall/mod.rs`: Added debug logging
- `runtime/root-task/src/component_loader.rs`: Added debug logging
