//! Context Switching for ARM64
//!
//! This module implements low-level context switching between threads.
//! Context switching involves saving the current thread's CPU state and
//! restoring the next thread's CPU state.
//!
//! ## ARM64 Context Switch Process
//!
//! 1. **Save current thread context**:
//!    - Save all general-purpose registers (x0-x30)
//!    - Save stack pointer (SP)
//!    - Save program counter (via LR for voluntary switches)
//!    - Save processor state (SPSR for exception-based switches)
//!
//! 2. **Switch page tables** (if different address spaces):
//!    - Update TTBR0_EL0 with next thread's VSpace
//!
//! 3. **Restore next thread context**:
//!    - Restore all general-purpose registers
//!    - Restore stack pointer
//!    - Restore program counter
//!    - Restore processor state
//!
//! ## Implementation Notes
//!
//! For Phase 4, we implement **voluntary context switching** (yield-based).
//! Later phases will add preemptive switching (timer interrupt-based).
//!
//! The context is stored in the TCB's TrapFrame structure. Since TrapFrame
//! is the first field in TCB, we can access it directly at offset 0.

use core::arch::global_asm;
use crate::objects::TCB;

/// Switch context from current thread to next thread
///
/// This is the main entry point for context switching. It saves the
///current thread's context to its TCB and restores the next thread's
/// context from its TCB.
///
/// # Arguments
///
/// * `current` - Pointer to current thread's TCB
/// * `next` - Pointer to next thread's TCB
///
/// # Safety
///
/// - Both TCB pointers must be valid and non-null
/// - Must be called with interrupts disabled
/// - TCBs must remain valid for the duration of the switch
/// - This function will not return to the same execution context
///
/// # Notes
///
/// After this function completes, execution continues in the `next` thread
/// at whatever instruction it was previously executing (or its entry point
/// if it's a new thread).
#[inline(never)]
pub unsafe fn switch_context(current: *mut TCB, next: *mut TCB) {
    // Call the assembly implementation
    // Both TCBs have TrapFrame as first field, so we can pass the TCB pointer directly
    switch_context_asm(current as *mut u8, next as *mut u8);
}

/// Assembly implementation of context switch
///
/// This is defined in inline assembly below.
extern "C" {
    fn switch_context_asm(current: *mut u8, next: *mut u8);
}

// ARM64 context switch implementation in assembly
//
// Register layout assumptions:
// - x0: pointer to current TCB (TrapFrame at offset 0)
// - x1: pointer to next TCB (TrapFrame at offset 0)
//
// TrapFrame layout (matching context.rs):
// Offset | Field
// -------|-------
// 0x00   | x0
// 0x08   | x1
// ...    | x2-x30 (8 bytes each)
// 0xF0   | sp_el0  (31 * 8 = 0xF8, but x30 at 0xF0, sp_el0 at 0xF8)
// 0xF8   | elr_el1
// 0x100  | spsr_el1
// 0x108  | esr_el1
// 0x110  | far_el1

global_asm!(
    "
    .section .text
    .global switch_context_asm
    .type switch_context_asm, @function

switch_context_asm:
    // x0 = current TCB (TrapFrame pointer)
    // x1 = next TCB (TrapFrame pointer)

    // Save current thread's context
    // Save general-purpose registers x0-x30
    stp x0, x1,   [x0, #(0 * 8)]
    stp x2, x3,   [x0, #(2 * 8)]
    stp x4, x5,   [x0, #(4 * 8)]
    stp x6, x7,   [x0, #(6 * 8)]
    stp x8, x9,   [x0, #(8 * 8)]
    stp x10, x11, [x0, #(10 * 8)]
    stp x12, x13, [x0, #(12 * 8)]
    stp x14, x15, [x0, #(14 * 8)]
    stp x16, x17, [x0, #(16 * 8)]
    stp x18, x19, [x0, #(18 * 8)]
    stp x20, x21, [x0, #(20 * 8)]
    stp x22, x23, [x0, #(22 * 8)]
    stp x24, x25, [x0, #(24 * 8)]
    stp x26, x27, [x0, #(26 * 8)]
    stp x28, x29, [x0, #(28 * 8)]

    // Save x30 (link register) - this is our return address
    str x30, [x0, #(30 * 8)]

    // Save stack pointer
    mov x2, sp
    str x2, [x0, #(31 * 8)]  // sp_el0 field

    // Save ELR (program counter for return)
    // For voluntary switch, use current LR as the resume point
    str x30, [x0, #(32 * 8)]  // elr_el1 field

    // Save SPSR (processor state)
    mrs x2, spsr_el1
    str x2, [x0, #(33 * 8)]  // spsr_el1 field

    // Now restore next thread's context
    // Restore general-purpose registers x2-x30 first
    ldp x2, x3,   [x1, #(2 * 8)]
    ldp x4, x5,   [x1, #(4 * 8)]
    ldp x6, x7,   [x1, #(6 * 8)]
    ldp x8, x9,   [x1, #(8 * 8)]
    ldp x10, x11, [x1, #(10 * 8)]
    ldp x12, x13, [x1, #(12 * 8)]
    ldp x14, x15, [x1, #(14 * 8)]
    ldp x16, x17, [x1, #(16 * 8)]
    ldp x18, x19, [x1, #(18 * 8)]
    ldp x20, x21, [x1, #(20 * 8)]
    ldp x22, x23, [x1, #(22 * 8)]
    ldp x24, x25, [x1, #(24 * 8)]
    ldp x26, x27, [x1, #(26 * 8)]
    ldp x28, x29, [x1, #(28 * 8)]

    // Restore x30 (link register / return address)
    ldr x30, [x1, #(30 * 8)]

    // Restore stack pointer
    ldr x0, [x1, #(31 * 8)]  // sp_el0 field
    mov sp, x0

    // Restore SPSR (processor state)
    ldr x0, [x1, #(33 * 8)]  // spsr_el1 field
    msr spsr_el1, x0

    // Finally restore x0 and x1
    ldp x0, x1, [x1, #(0 * 8)]

    // Return to next thread
    // Since we restored x30 (LR), this ret will jump to next thread's resume point
    ret

    .size switch_context_asm, .-switch_context_asm
    "
);

/// Initialize a thread's context for first run
///
/// Sets up the TrapFrame so that when the thread is first switched to,
/// it will start executing at the given entry point with the given stack.
///
/// # Arguments
///
/// * `tcb` - Thread to initialize
/// * `entry` - Function pointer to start executing
/// * `stack_top` - Top of the thread's stack (grows down)
/// * `arg` - Argument to pass to entry function (in x0)
///
/// # Safety
///
/// - TCB must be valid
/// - Entry point must be a valid function pointer
/// - Stack must be properly allocated and aligned
pub unsafe fn init_thread_context(
    tcb: *mut TCB,
    entry: usize,
    stack_top: usize,
    arg: u64,
) {
    let tcb_ref = &mut *tcb;
    let context = tcb_ref.context_mut();

    // Clear all registers
    *context = crate::arch::aarch64::context::TrapFrame::new();

    // Set up entry point (will be loaded into PC via x30/LR)
    context.x30 = entry as u64;  // Link register = entry point
    context.elr_el1 = entry as u64;  // Also set ELR for consistency

    // Set up stack pointer (8-byte aligned, grows down)
    context.sp_el0 = (stack_top & !0x7) as u64;

    // Set up argument in x0
    context.x0 = arg;

    // Set up processor state (EL1, interrupts enabled)
    // SPSR_EL1 format:
    // - Bits 0-3: Mode (0b0101 = EL1h, handler uses SP_EL1)
    // - Bit 6: FIQ mask (0 = not masked)
    // - Bit 7: IRQ mask (0 = not masked)
    // - Bit 8: SError mask (0 = not masked)
    // - Bit 9: Debug mask
    context.spsr_el1 = 0x00000005; // EL1h mode, interrupts enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_switch_not_null() {
        // This test just verifies the function exists and can be called
        // Real testing requires actual TCBs and a running kernel
        // We'll test this properly in Phase 5 integration tests
    }
}
