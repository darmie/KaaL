//! ARM64 Exception Handling
//!
//! This module implements the ARM64 exception vector table and exception handlers.
//! The vector table contains 16 entries (4 exception types × 4 sources).
//!
//! # Exception Types
//! - Synchronous: Syscalls, data/instruction aborts, alignment faults
//! - IRQ: Normal interrupts
//! - FIQ: Fast interrupts (not used in KaaL)
//! - SError: System errors (async aborts)
//!
//! # Exception Sources
//! - Current EL with SP_EL0
//! - Current EL with SP_ELx
//! - Lower EL (AArch64) - User space exceptions
//! - Lower EL (AArch32) - Not supported
//!
//! # Vector Table Layout (each entry is 128 bytes apart)
//! ```
//! 0x000: Current EL with SP_EL0 - Synchronous
//! 0x080: Current EL with SP_EL0 - IRQ
//! 0x100: Current EL with SP_EL0 - FIQ
//! 0x180: Current EL with SP_EL0 - SError
//! 0x200: Current EL with SP_ELx - Synchronous
//! 0x280: Current EL with SP_ELx - IRQ
//! 0x300: Current EL with SP_ELx - FIQ
//! 0x380: Current EL with SP_ELx - SError
//! 0x400: Lower EL (AArch64) - Synchronous  ← Syscalls from user space
//! 0x480: Lower EL (AArch64) - IRQ
//! 0x500: Lower EL (AArch64) - FIQ
//! 0x580: Lower EL (AArch64) - SError
//! 0x600: Lower EL (AArch32) - Synchronous  (not supported)
//! 0x680: Lower EL (AArch32) - IRQ
//! 0x700: Lower EL (AArch32) - FIQ
//! 0x780: Lower EL (AArch32) - SError
//! ```

use core::arch::global_asm;
use crate::kprintln;
use super::context::TrapFrame;

/// Exception vector table - must be 2KB aligned
///
/// Each entry has 128 bytes (0x80) to save context and branch to handler.
/// The table is written in assembly and must be properly aligned.
global_asm!(
    ".section .text.exception_vectors",
    ".balign 2048",  // Vector table must be 2KB aligned (0x800)
    ".global exception_vector_table",
    "exception_vector_table:",

    // Current EL with SP_EL0
    ".balign 0x80",
    "curr_el_sp0_sync:",
    "    b exception_curr_el_sp0_sync",

    ".balign 0x80",
    "curr_el_sp0_irq:",
    "    b exception_curr_el_sp0_irq",

    ".balign 0x80",
    "curr_el_sp0_fiq:",
    "    b exception_curr_el_sp0_fiq",

    ".balign 0x80",
    "curr_el_sp0_serror:",
    "    b exception_curr_el_sp0_serror",

    // Current EL with SP_ELx
    ".balign 0x80",
    "curr_el_spx_sync:",
    "    b handle_curr_el_spx_sync",

    ".balign 0x80",
    "curr_el_spx_irq:",
    "    b exception_curr_el_spx_irq",

    ".balign 0x80",
    "curr_el_spx_fiq:",
    "    b exception_curr_el_spx_fiq",

    ".balign 0x80",
    "curr_el_spx_serror:",
    "    b exception_curr_el_spx_serror",

    // Lower EL (AArch64)
    ".balign 0x80",
    "lower_el_aarch64_sync:",
    "    b handle_lower_el_aarch64_sync",

    ".balign 0x80",
    "lower_el_aarch64_irq:",
    "    b exception_lower_el_aarch64_irq",

    ".balign 0x80",
    "lower_el_aarch64_fiq:",
    "    b exception_lower_el_aarch64_fiq",

    ".balign 0x80",
    "lower_el_aarch64_serror:",
    "    b exception_lower_el_aarch64_serror",

    // Lower EL (AArch32) - not supported
    ".balign 0x80",
    "lower_el_aarch32_sync:",
    "    b exception_lower_el_aarch32_sync",

    ".balign 0x80",
    "lower_el_aarch32_irq:",
    "    b exception_lower_el_aarch32_irq",

    ".balign 0x80",
    "lower_el_aarch32_fiq:",
    "    b exception_lower_el_aarch32_fiq",

    ".balign 0x80",
    "lower_el_aarch32_serror:",
    "    b exception_lower_el_aarch32_serror",
);

/// Shared exception handler stub - saves context, calls Rust handler, restores context
global_asm!(
    ".global handle_curr_el_spx_sync",
    "handle_curr_el_spx_sync:",
    // Save all context to stack (288 bytes)
    "    sub sp, sp, #288",
    "    stp x0, x1, [sp, #0]",
    "    stp x2, x3, [sp, #16]",
    "    stp x4, x5, [sp, #32]",
    "    stp x6, x7, [sp, #48]",
    "    stp x8, x9, [sp, #64]",
    "    stp x10, x11, [sp, #80]",
    "    stp x12, x13, [sp, #96]",
    "    stp x14, x15, [sp, #112]",
    "    stp x16, x17, [sp, #128]",
    "    stp x18, x19, [sp, #144]",
    "    stp x20, x21, [sp, #160]",
    "    stp x22, x23, [sp, #176]",
    "    stp x24, x25, [sp, #192]",
    "    stp x26, x27, [sp, #208]",
    "    stp x28, x29, [sp, #224]",
    "    str x30, [sp, #240]",
    "    mrs x0, sp_el0",
    "    mrs x1, elr_el1",
    "    mrs x2, spsr_el1",
    "    mrs x3, esr_el1",
    "    mrs x4, far_el1",
    "    stp x0, x1, [sp, #248]",
    "    stp x2, x3, [sp, #264]",
    "    str x4, [sp, #280]",
    "    mov x0, sp",                 // Pass TrapFrame* to handler
    // Call Rust handler
    "    bl exception_curr_el_spx_sync_handler",
    // Restore context
    "    ldr x0, [sp, #248]",
    "    msr sp_el0, x0",
    "    ldp x0, x1, [sp, #0]",
    "    ldp x2, x3, [sp, #16]",
    "    ldp x4, x5, [sp, #32]",
    "    ldp x6, x7, [sp, #48]",
    "    ldp x8, x9, [sp, #64]",
    "    ldp x10, x11, [sp, #80]",
    "    ldp x12, x13, [sp, #96]",
    "    ldp x14, x15, [sp, #112]",
    "    ldp x16, x17, [sp, #128]",
    "    ldp x18, x19, [sp, #144]",
    "    ldp x20, x21, [sp, #160]",
    "    ldp x22, x23, [sp, #176]",
    "    ldp x24, x25, [sp, #192]",
    "    ldp x26, x27, [sp, #208]",
    "    ldp x28, x29, [sp, #224]",
    "    ldr x30, [sp, #240]",
    "    ldr x1, [sp, #256]",
    "    ldr x2, [sp, #264]",
    "    msr elr_el1, x1",
    "    msr spsr_el1, x2",
    "    add sp, sp, #288",
    "    eret",
);

/// Lower EL exception handler stub - saves context, calls Rust handler, restores context
///
/// Chapter 9: This handler implements page table switching for secure syscall handling.
/// When transitioning from EL0->EL1, we save the user's TTBR0 and restore it on return.
/// The kernel runs with TTBR1 (upper address space), so we need to ensure TTBR0 doesn't
/// interfere with kernel memory access during exception handling.
global_asm!(
    ".global handle_lower_el_aarch64_sync",
    "handle_lower_el_aarch64_sync:",
    // Save all context to stack (296 bytes = 288 + 8 for TTBR0)
    "    sub sp, sp, #296",
    "    stp x0, x1, [sp, #0]",
    "    stp x2, x3, [sp, #16]",
    "    stp x4, x5, [sp, #32]",
    "    stp x6, x7, [sp, #48]",
    "    stp x8, x9, [sp, #64]",
    "    stp x10, x11, [sp, #80]",
    "    stp x12, x13, [sp, #96]",
    "    stp x14, x15, [sp, #112]",
    "    stp x16, x17, [sp, #128]",
    "    stp x18, x19, [sp, #144]",
    "    stp x20, x21, [sp, #160]",
    "    stp x22, x23, [sp, #176]",
    "    stp x24, x25, [sp, #192]",
    "    stp x26, x27, [sp, #208]",
    "    stp x28, x29, [sp, #224]",
    "    str x30, [sp, #240]",
    "    mrs x0, sp_el0",
    "    mrs x1, elr_el1",
    "    mrs x2, spsr_el1",
    "    mrs x3, esr_el1",
    "    mrs x4, far_el1",
    "    stp x0, x1, [sp, #248]",
    "    stp x2, x3, [sp, #264]",
    "    str x4, [sp, #280]",
    // Note: With our unified page table design (kernel + user in same PT with EL-based permissions),
    // we don't need to switch TTBR0. The user page table already contains kernel mappings
    // with EL1-only access permissions, so kernel code can run while user code can't access it.
    "    mrs x5, ttbr0_el1",           // Save user's page table (for debugging)
    "    str x5, [sp, #288]",          // Store at offset 288
    // No page table switch needed - we stay on the user PT with kernel mappings
    "    mov x0, sp",                  // Pass TrapFrame* to handler
    // Call Rust handler
    "    bl exception_lower_el_aarch64_sync_handler",
    // No need to restore TTBR0 since we never changed it
    // Restore system registers first (using temporary registers)
    "    ldr x10, [sp, #248]",         // sp_el0
    "    ldr x11, [sp, #256]",         // elr_el1
    "    ldr x12, [sp, #264]",         // spsr_el1
    "    ldr x13, [sp, #288]",         // ttbr0_el1 (saved)
    "    msr sp_el0, x10",
    "    msr elr_el1, x11",
    "    msr spsr_el1, x12",
    // Conditional TLB invalidation: Only flush if TTBR0 changed
    // This is critical - unconditional flush breaks kernel memory access!
    "    mrs x14, ttbr0_el1",          // Read current TTBR0
    "    cmp x13, x14",                // Compare saved vs current
    "    b.eq 1f",                     // Skip TLB flush if unchanged
    // TTBR0 changed - must flush TLB per ARM architecture requirements
    "    msr ttbr0_el1, x13",          // Restore new TTBR0
    "    tlbi vmalle1is",              // Invalidate all TLB entries
    "    dsb ish",                     // Ensure TLB invalidation completes
    "    b 2f",
    "1:",                              // TTBR0 unchanged - no TLB flush needed
    "    msr ttbr0_el1, x13",          // Restore TTBR0 anyway (cheap)
    "2:",
    "    isb",                         // Synchronize context
    // Now restore GPRs (including x0-x3 with potentially modified syscall returns)
    "    ldp x0, x1, [sp, #0]",
    "    ldp x2, x3, [sp, #16]",
    "    ldp x4, x5, [sp, #32]",
    "    ldp x6, x7, [sp, #48]",
    "    ldp x8, x9, [sp, #64]",
    "    ldp x10, x11, [sp, #80]",
    "    ldp x12, x13, [sp, #96]",
    "    ldp x14, x15, [sp, #112]",
    "    ldp x16, x17, [sp, #128]",
    "    ldp x18, x19, [sp, #144]",
    "    ldp x20, x21, [sp, #160]",
    "    ldp x22, x23, [sp, #176]",
    "    ldp x24, x25, [sp, #192]",
    "    ldp x26, x27, [sp, #208]",
    "    ldp x28, x29, [sp, #224]",
    "    ldr x30, [sp, #240]",
    "    add sp, sp, #296",            // Adjusted for new stack frame size
    "    eret",
);

// Rust exception handlers (called from assembly stubs with TrapFrame* in x0)

#[no_mangle]
extern "C" fn exception_curr_el_sp0_sync() {
    kprintln!("[exception] Current EL with SP_EL0 - Synchronous");
    print_exception_info();
    panic!("Unhandled exception: Current EL SP0 Sync");
}

#[no_mangle]
extern "C" fn exception_curr_el_sp0_irq() {
    kprintln!("[exception] Current EL with SP_EL0 - IRQ");
    panic!("Unhandled exception: Current EL SP0 IRQ");
}

#[no_mangle]
extern "C" fn exception_curr_el_sp0_fiq() {
    kprintln!("[exception] Current EL with SP_EL0 - FIQ");
    panic!("Unhandled exception: Current EL SP0 FIQ");
}

#[no_mangle]
extern "C" fn exception_curr_el_sp0_serror() {
    kprintln!("[exception] Current EL with SP_EL0 - SError");
    print_exception_info();
    panic!("Unhandled exception: Current EL SP0 SError");
}

#[no_mangle]
extern "C" fn exception_curr_el_spx_sync_handler(tf: &mut TrapFrame) {
    kprintln!("[exception] Current EL with SP_ELx - Synchronous");
    kprintln!("  ELR: {:#x}, ESR: {:#x}, FAR: {:#x}", tf.elr_el1, tf.esr_el1, tf.far_el1);
    kprintln!("  Exception class: {:#x}", tf.exception_class());

    // Check if it's a syscall (shouldn't happen from kernel)
    if tf.is_syscall() {
        kprintln!("  → Syscall from kernel space (test mode)");
        kprintln!("  Syscall number: {}", tf.syscall_number());
        let args = tf.syscall_args();
        kprintln!("  Arguments: [{}, {}, {}, {}, {}, {}]",
            args[0], args[1], args[2], args[3], args[4], args[5]);
        kprintln!("  ✓ Syscall trap frame working correctly!");
        // For testing, don't panic - just return
        return;
    } else if tf.is_data_abort() {
        kprintln!("  → Data abort at address {:#x}", tf.far_el1);

        // Check if this is a test data abort at 0xdeadbeef
        if tf.far_el1 == 0xdeadbeef {
            print_exception_info();
            kprintln!("  ✓ Data abort exception caught successfully!");
            // Skip past the faulting instruction (ARM64 instructions are 4 bytes)
            tf.elr_el1 += 4;
            // For testing, don't panic - just return
            return;
        }
    } else if tf.is_instruction_abort() {
        kprintln!("  → Instruction abort at address {:#x}", tf.far_el1);
    }

    print_exception_info();
    panic!("Unhandled exception: Current EL SPx Sync");
}

#[no_mangle]
extern "C" fn exception_curr_el_spx_irq() {
    // IRQ while kernel is running
    unsafe {
        // Acknowledge interrupt and get IRQ number from GIC
        if let Some(irq_id) = crate::arch::aarch64::gic::acknowledge_irq() {
            // Check if this is the timer IRQ (special case - handled by kernel)
            if irq_id == crate::generated::memory_config::IRQ_TIMER {
                crate::scheduler::timer::timer_tick();
            } else {
                // Check if a userspace driver has registered for this IRQ
                crate::objects::irq_handler::handle_irq(irq_id);
            }

            // Signal end of interrupt to GIC
            crate::arch::aarch64::gic::end_of_interrupt(irq_id);
        }
        // Spurious IRQ if None - just return
    }
}

#[no_mangle]
extern "C" fn exception_curr_el_spx_fiq() {
    kprintln!("[exception] Current EL with SP_ELx - FIQ");
    panic!("Unhandled exception: Current EL SPx FIQ");
}

#[no_mangle]
extern "C" fn exception_curr_el_spx_serror() {
    kprintln!("[exception] Current EL with SP_ELx - SError");
    print_exception_info();
    panic!("Unhandled exception: Current EL SPx SError");
}

/// Debug function called right before eret to check TrapFrame
#[no_mangle]
extern "C" fn debug_before_eret(frame: &TrapFrame) {
    kprintln!("[exception] About to eret:");
    kprintln!("  ELR={:#x}, SP={:#x}, SPSR={:#x}, TTBR0={:#x}",
              frame.elr_el1, frame.sp_el0, frame.spsr_el1, frame.saved_ttbr0);
}

/// Debug function called right before eret with minimal register clobbering
#[no_mangle]
extern "C" fn debug_eret_state(elr: u64, spsr: u64) {
    // Check if this looks like a first-time userspace transition
    if spsr == 0x0 && elr > 0x200000 && elr < 0x300000 {
        kprintln!("[exception] First eret to userspace:");
        kprintln!("  ELR={:#x}, SPSR={:#x}", elr, spsr);

        // Read current PSTATE to see processor state
        let current_el: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, CurrentEL",
                out(reg) current_el,
            );
        }
        kprintln!("  CurrentEL={:#x} (should be 0x4 for EL1)", current_el >> 2);
    }
}

/// Handler for synchronous exceptions from lower EL (EL0 userspace)
/// Called from assembly stub with TrapFrame* in x0
#[no_mangle]
extern "C" fn exception_lower_el_aarch64_sync_handler(frame: &mut TrapFrame) {
    // Extract exception class from ESR_EL1 (bits 26-31)
    let esr = frame.esr_el1;
    let ec = (esr >> 26) & 0x3F;

    // EC 0x15 = SVC instruction from AArch64 (syscall)
    if ec == 0x15 {
        // Debug: Check for suspicious syscalls
        if frame.syscall_number() == 0 && frame.elr_el1 > 0x210000 && frame.elr_el1 < 0x220000 {
            crate::kprintln!("[exception] Suspicious syscall 0:");
            crate::kprintln!("  ELR={:#x}, SP={:#x}, x30={:#x}",
                            frame.elr_el1, frame.sp_el0, frame.x30);
            crate::kprintln!("  x8={:#x}, x0={:#x}, x1={:#x}",
                            frame.x8, frame.x0, frame.x1);
        }
        crate::syscall::handle_syscall(frame);
        return;
    }

    // Check for instruction/prefetch abort
    if ec == 0x20 || ec == 0x21 {  // Instruction abort from lower EL
        crate::kprintln!("[exception] Prefetch/Instruction Abort from EL0:");
        crate::kprintln!("  PC (ELR): {:#x}", frame.elr_el1);
        crate::kprintln!("  Fault Address (FAR): {:#x}", frame.far_el1);
        crate::kprintln!("  ESR: {:#x}", esr);
        crate::kprintln!("  ISS: {:#x}", esr & 0x1FFFFFF);
        panic!("Instruction abort from EL0");
    }

    // Not a syscall - print debug info and panic
    kprintln!("[exception] Unhandled EL0 exception:");
    kprintln!("  EC: {:#x}, ESR: {:#x}", ec, esr);
    kprintln!("  ELR: {:#x}, FAR: {:#x}", frame.elr_el1, frame.far_el1);
    panic!("Unhandled exception from EL0");
}

#[no_mangle]
extern "C" fn exception_lower_el_aarch64_irq() {
    // IRQ while userspace is running
    unsafe {
        // Acknowledge interrupt and get IRQ number from GIC
        if let Some(irq_id) = crate::arch::aarch64::gic::acknowledge_irq() {
            // Check if this is the timer IRQ (special case - handled by kernel)
            if irq_id == crate::generated::memory_config::IRQ_TIMER {
                crate::scheduler::timer::timer_tick();
            } else {
                // Check if a userspace driver has registered for this IRQ
                crate::objects::irq_handler::handle_irq(irq_id);
            }

            // Signal end of interrupt to GIC
            crate::arch::aarch64::gic::end_of_interrupt(irq_id);
        }
        // Spurious IRQ if None - just return
    }
}

#[no_mangle]
extern "C" fn exception_lower_el_aarch64_fiq() {
    kprintln!("[exception] Lower EL (AArch64) - FIQ");
    panic!("Unhandled exception: Lower EL FIQ");
}

#[no_mangle]
extern "C" fn exception_lower_el_aarch64_serror() {
    kprintln!("[exception] Lower EL (AArch64) - SError");
    print_exception_info();
    panic!("Unhandled exception: Lower EL SError");
}

#[no_mangle]
extern "C" fn exception_lower_el_aarch32_sync() {
    panic!("AArch32 not supported");
}

#[no_mangle]
extern "C" fn exception_lower_el_aarch32_irq() {
    panic!("AArch32 not supported");
}

#[no_mangle]
extern "C" fn exception_lower_el_aarch32_fiq() {
    panic!("AArch32 not supported");
}

#[no_mangle]
extern "C" fn exception_lower_el_aarch32_serror() {
    panic!("AArch32 not supported");
}

/// Print exception syndrome and fault address
fn print_exception_info() {
    use crate::arch::aarch64::registers::*;

    let esr = ESR_EL1::read();
    let far = FAR_EL1::read();
    let elr = ELR_EL1::read();
    let spsr = SPSR_EL1::read();

    kprintln!("  ESR_EL1:  0x{:016x}", esr);
    kprintln!("  FAR_EL1:  0x{:016x}", far);
    kprintln!("  ELR_EL1:  0x{:016x}", elr);
    kprintln!("  SPSR_EL1: 0x{:016x}", spsr);

    // Decode ESR_EL1
    let ec = (esr >> 26) & 0x3F;  // Exception Class (bits 31:26)
    let iss = esr & 0x1FFFFFF;     // Instruction Specific Syndrome (bits 24:0)

    kprintln!("  Exception Class: 0x{:02x}", ec);
    kprintln!("  ISS: 0x{:07x}", iss);

    // Common exception classes
    match ec {
        0x00 => kprintln!("    → Unknown reason"),
        0x15 => kprintln!("    → SVC instruction (syscall)"),
        0x20 => kprintln!("    → Instruction abort from lower EL"),
        0x21 => kprintln!("    → Instruction abort from same EL"),
        0x24 => kprintln!("    → Data abort from lower EL"),
        0x25 => kprintln!("    → Data abort from same EL"),
        0x2F => kprintln!("    → SError interrupt"),
        _ => kprintln!("    → Other (see ARMv8 manual)"),
    }

    // For data/instruction aborts, decode fault status
    if ec == 0x20 || ec == 0x21 || ec == 0x24 || ec == 0x25 {
        let dfsc = iss & 0x3F;  // Data Fault Status Code
        kprintln!("  Fault Status Code: 0x{:02x}", dfsc);

        match dfsc {
            0b000100 => kprintln!("    → Translation fault, level 0"),
            0b000101 => kprintln!("    → Translation fault, level 1"),
            0b000110 => kprintln!("    → Translation fault, level 2"),
            0b000111 => kprintln!("    → Translation fault, level 3"),
            0b001001 => kprintln!("    → Access flag fault, level 1"),
            0b001010 => kprintln!("    → Access flag fault, level 2"),
            0b001011 => kprintln!("    → Access flag fault, level 3"),
            0b001101 => kprintln!("    → Permission fault, level 1"),
            0b001110 => kprintln!("    → Permission fault, level 2"),
            0b001111 => kprintln!("    → Permission fault, level 3"),
            _ => kprintln!("    → Other fault (see ARMv8 manual)"),
        }
    }
}

/// Install exception vector table
///
/// Sets VBAR_EL1 to point to our exception vector table.
/// Must be called during kernel initialization before enabling MMU.
/// See docs/chapters/CHAPTER_03_STATUS.md for exception handling details.
pub fn init() {
    use crate::arch::aarch64::registers::VBAR_EL1;

    extern "C" {
        static exception_vector_table: u8;
    }

    let vector_table_addr = unsafe { &exception_vector_table as *const _ as u64 };

    // Verify alignment (must be 2KB aligned)
    if vector_table_addr & 0x7FF != 0 {
        panic!("Exception vector table not 2KB aligned!");
    }

    unsafe {
        VBAR_EL1::write(vector_table_addr);
    }
}
