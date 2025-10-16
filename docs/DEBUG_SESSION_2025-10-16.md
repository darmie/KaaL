# Debug Session: Data Abort at 0x1003

## Date: 2025-10-16

## Current Issue
After successfully fixing DTB parsing, stack alignment, and root-task linker script issues, the kernel now boots fully and transitions to EL0, but immediately faults with:

```
[exception] Current EL with SP_ELx - Synchronous
  ELR: 0x40407a50, ESR: 0x96000006, FAR: 0x1003
  Exception class: 0x25
  → Data abort at address 0x1003
  Fault Status Code: 0x06
    → Translation fault, level 2
```

## Analysis

### What We Know

1. **Exception occurs AFTER eret**:
   - Kernel prints "About to call transition_to_el0..." successfully
   - The `transition_to_el0` function at 0x404000d8 contains only:
     ```assembly
     404000d8: msr TTBR0_EL1, x2
     404000dc: isb
     404000e0: msr ELR_EL1, x0
     404000e4: msr SP_EL0, x1
     404000e8: mov x3, #0x0
     404000ec: msr SPSR_EL1, x3
     404000f0: eret
     ```
   - Clean assembly, no extra code

2. **Exception is in kernel mode ("Current EL with SP_ELx")**:
   - This means we're BACK in EL1, not stuck trying to get to EL0
   - The eret succeeded, but user code immediately faulted
   - Kernel exception handler caught it

3. **Fault address 0x1003**:
   - Very low address (only 4099 bytes into address space)
   - Not mapped in user page tables
   - Translation fault level 2 = page table walk failed at L2

4. **ELR points to memset** (0x40407a50):
   - This is `compiler_builtins::mem::memset`
   - Kernel is trying to write to 0x1003
   - But why is kernel code running after eret to userspace?

5. **Root task entry point looks correct**:
   ```assembly
   40100000: stp x29, x30, [sp, #-0x60]!
   40100004: stp x28, x27, [sp, #0x10]
   ...
   40100034: mov w19, #0x1001            // Suspicious: 0x1001 is close to 0x1003!
   ```

### Hypotheses

#### Hypothesis 1: Exception Handler Issue
When user code faults, the exception handler is entered but something in the exception handling path is broken:
- Maybe exception handler is trying to access an unmapped address
- Could be related to exception frame saving/restoring
- Memset might be called to zero out exception frame

#### Hypothesis 2: Page Table Not Active
- Maybe TTBR0 switch didn't take effect properly
- User code tries to access low address thinking it's mapped
- But this doesn't explain why kernel code (memset) is running

#### Hypothesis 3: Root Task _start Has Issue
- Root task entry point might immediately fault
- First few instructions try to set up stack
- Stack operations at 0x40100018-0x40100028 look fine though

#### Hypothesis 4: Boot Info Pointer Issue
- Root task might be trying to access boot info or IPC buffer
- Boot info mapped at 0x7ffff000
- IPC buffer at 0x80000000
- But neither of these are near 0x1003

## Memory Layout

### Kernel
- Kernel: 0x40400000 - 0x4043c000
- memset function: ~0x40407a50

### User Space (Root Task)
- Code: 0x40100000 (10 pages = 40KB)
- Stack: 0x400bf000 - 0x400ff000 (256KB)
- UART: 0x9000000 (4KB)
- Boot Info: 0x7ffff000 (1 page)

### Transition Parameters
```
Entry:    0x40100000
Stack:    0x400ff000
TTBR0:    0x40443000
```

## Next Steps for Investigation

### 1. Check Exception Handler
Look at what happens when user code takes an exception:
- File: kernel/src/arch/aarch64/exception.rs
- Check lower_el_aarch64_sync handler
- Verify it doesn't access invalid addresses
- Check if memset is called anywhere in exception path

### 2. Add More Debug Output
Before eret, verify:
- TTBR0_EL1 is correctly set
- ELR_EL1 contains 0x40100000
- SP_EL0 contains 0x400ff000
- SPSR_EL1 is correct (should be 0 for EL0t)

After fault, check:
- What was ELR_EL1 when the user fault occurred?
- What instruction caused the fault?
- Was it really a data abort or instruction abort?

### 3. Verify Page Tables
Use debug walk to verify user page tables:
- Check 0x40100000 is mapped correctly
- Check stack pages are mapped
- Check if there's an entry for low addresses (there shouldn't be)

### 4. Check Root Task Start
Examine runtime/root-task/src/main.rs:
- Look at _start function
- See if it tries to access any low addresses
- Check if there's a null pointer dereference

### 5. Compare with seL4/Microkit
Study how seL4 handles first transition to userspace:
- How do they set up initial thread state?
- Do they do anything special before eret?
- How do they handle first exception from userspace?

## Files to Examine
- kernel/src/arch/aarch64/exception.rs - Exception handlers
- kernel/src/boot/root_task.rs - EL0 transition code
- runtime/root-task/src/main.rs - Root task entry point
- kernel/src/objects/tcb.rs - TCB structure and state

## Useful Commands

### Disassemble Functions
```bash
# Find symbol address
nm kernel/target/aarch64-unknown-none/release/kaal-kernel | grep <symbol>

# Disassemble range
llvm-objdump -d kernel/target/aarch64-unknown-none/release/kaal-kernel \
  --start-address=0x<addr> --stop-address=0x<addr>
```

### Check Page Mappings
Add to root_task.rs before transition:
```rust
mapper.debug_walk(VirtAddr::new(0x40100000));  // Entry point
mapper.debug_walk(VirtAddr::new(0x400ff000));  // Stack
mapper.debug_walk(VirtAddr::new(0x1003));      // Fault address
```

### Run with Timeout
```bash
timeout 12 qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M \
  -nographic -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader
```

## Conclusion

The kernel successfully boots and attempts EL0 transition, but something goes wrong immediately after eret. The fault address 0x1003 and the fact that kernel's memset is being called suggests either:
1. An issue in the exception handler when catching the first user fault
2. A problem with how user code is set up (null pointer, bad stack, etc.)
3. A page table or MMU configuration issue

The next session should focus on understanding what happens immediately after eret and why address 0x1003 is being accessed by kernel code.
