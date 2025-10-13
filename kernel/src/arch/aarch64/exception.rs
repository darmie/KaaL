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
    "    b exception_curr_el_spx_sync",

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
    "    b exception_lower_el_aarch64_sync",

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

// Rust exception handlers (called from assembly stubs)

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
extern "C" fn exception_curr_el_spx_sync() {
    kprintln!("[exception] Current EL with SP_ELx - Synchronous");
    print_exception_info();
    panic!("Unhandled exception: Current EL SPx Sync");
}

#[no_mangle]
extern "C" fn exception_curr_el_spx_irq() {
    kprintln!("[exception] Current EL with SP_ELx - IRQ");
    panic!("Unhandled exception: Current EL SPx IRQ");
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

#[no_mangle]
extern "C" fn exception_lower_el_aarch64_sync() {
    kprintln!("[exception] Lower EL (AArch64) - Synchronous");
    print_exception_info();

    // TODO: Decode ESR_EL1 to determine if this is a syscall or fault
    panic!("Unhandled exception: Lower EL Sync (syscall or fault)");
}

#[no_mangle]
extern "C" fn exception_lower_el_aarch64_irq() {
    kprintln!("[exception] Lower EL (AArch64) - IRQ");
    panic!("Unhandled exception: Lower EL IRQ");
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
pub fn init() {
    use crate::arch::aarch64::registers::VBAR_EL1;

    extern "C" {
        static exception_vector_table: u8;
    }

    let vector_table_addr = unsafe { &exception_vector_table as *const _ as u64 };

    kprintln!("[exception] Installing exception vector table at 0x{:016x}", vector_table_addr);

    // Verify alignment (must be 2KB aligned)
    if vector_table_addr & 0x7FF != 0 {
        panic!("Exception vector table not 2KB aligned!");
    }

    unsafe {
        VBAR_EL1::write(vector_table_addr);
    }

    kprintln!("[exception] Exception handlers installed");
}
