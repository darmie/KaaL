//! ARM64 Trap Frame and Context Switching
//!
//! This module defines the trap frame structure that holds CPU context during
//! exceptions. The trap frame is saved/restored when transitioning between
//! exception levels (EL0 ↔ EL1).
//!
//! # Trap Frame Layout
//!
//! The trap frame contains all general-purpose registers (x0-x30) plus special
//! registers needed for exception return (ELR_EL1, SPSR_EL1, etc.).
//!
//! Layout matches ARM64 calling convention for efficient syscall argument passing:
//! - x0-x7: Arguments and return values
//! - x8: Syscall number (by convention)
//! - x9-x15: Temporary registers
//! - x16-x17: Intra-procedure-call registers
//! - x18: Platform register
//! - x19-x28: Callee-saved registers
//! - x29: Frame pointer (FP)
//! - x30: Link register (LR)
//! - SP: Stack pointer (separate for EL0/EL1)

use core::fmt;

/// Trap frame - CPU context saved during exception
///
/// This structure is carefully laid out to match the order in which registers
/// are saved/restored in assembly. DO NOT reorder fields without updating the
/// assembly code in exception.rs.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrapFrame {
    // General purpose registers (x0-x30)
    pub x0: u64,
    pub x1: u64,
    pub x2: u64,
    pub x3: u64,
    pub x4: u64,
    pub x5: u64,
    pub x6: u64,
    pub x7: u64,
    pub x8: u64,
    pub x9: u64,
    pub x10: u64,
    pub x11: u64,
    pub x12: u64,
    pub x13: u64,
    pub x14: u64,
    pub x15: u64,
    pub x16: u64,
    pub x17: u64,
    pub x18: u64,
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64,  // Frame pointer
    pub x30: u64,  // Link register

    // Special registers for exception handling
    pub sp_el0: u64,    // User stack pointer
    pub elr_el1: u64,   // Exception link register (return address)
    pub spsr_el1: u64,  // Saved processor status register
    pub esr_el1: u64,   // Exception syndrome register
    pub far_el1: u64,   // Fault address register
}

impl TrapFrame {
    /// Create a new trap frame with all registers zeroed
    pub const fn new() -> Self {
        Self {
            x0: 0, x1: 0, x2: 0, x3: 0, x4: 0, x5: 0, x6: 0, x7: 0,
            x8: 0, x9: 0, x10: 0, x11: 0, x12: 0, x13: 0, x14: 0, x15: 0,
            x16: 0, x17: 0, x18: 0, x19: 0, x20: 0, x21: 0, x22: 0, x23: 0,
            x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0,
            sp_el0: 0,
            elr_el1: 0,
            spsr_el1: 0,
            esr_el1: 0,
            far_el1: 0,
        }
    }

    /// Get syscall number (x8 by convention)
    #[inline]
    pub fn syscall_number(&self) -> u64 {
        self.x8
    }

    /// Get syscall arguments (x0-x5)
    #[inline]
    pub fn syscall_args(&self) -> [u64; 6] {
        [self.x0, self.x1, self.x2, self.x3, self.x4, self.x5]
    }

    /// Set syscall return value (x0)
    #[inline]
    pub fn set_return_value(&mut self, value: u64) {
        self.x0 = value;
    }

    /// Get exception class from ESR_EL1
    #[inline]
    pub fn exception_class(&self) -> u8 {
        ((self.esr_el1 >> 26) & 0x3F) as u8
    }

    /// Get instruction specific syndrome from ESR_EL1
    #[inline]
    pub fn iss(&self) -> u32 {
        (self.esr_el1 & 0x1FFFFFF) as u32
    }

    /// Check if this is a syscall (EC == 0x15)
    #[inline]
    pub fn is_syscall(&self) -> bool {
        self.exception_class() == 0x15
    }

    /// Check if this is a data abort (EC == 0x24 or 0x25)
    #[inline]
    pub fn is_data_abort(&self) -> bool {
        let ec = self.exception_class();
        ec == 0x24 || ec == 0x25
    }

    /// Check if this is an instruction abort (EC == 0x20 or 0x21)
    #[inline]
    pub fn is_instruction_abort(&self) -> bool {
        let ec = self.exception_class();
        ec == 0x20 || ec == 0x21
    }
}

impl fmt::Debug for TrapFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrapFrame")
            .field("x0", &format_args!("0x{:016x}", self.x0))
            .field("x1", &format_args!("0x{:016x}", self.x1))
            .field("x2", &format_args!("0x{:016x}", self.x2))
            .field("x3", &format_args!("0x{:016x}", self.x3))
            .field("x4", &format_args!("0x{:016x}", self.x4))
            .field("x5", &format_args!("0x{:016x}", self.x5))
            .field("x6", &format_args!("0x{:016x}", self.x6))
            .field("x7", &format_args!("0x{:016x}", self.x7))
            .field("x8", &format_args!("0x{:016x}", self.x8))
            .field("x29_fp", &format_args!("0x{:016x}", self.x29))
            .field("x30_lr", &format_args!("0x{:016x}", self.x30))
            .field("sp_el0", &format_args!("0x{:016x}", self.sp_el0))
            .field("elr_el1", &format_args!("0x{:016x}", self.elr_el1))
            .field("spsr_el1", &format_args!("0x{:016x}", self.spsr_el1))
            .field("esr_el1", &format_args!("0x{:016x}", self.esr_el1))
            .field("far_el1", &format_args!("0x{:016x}", self.far_el1))
            .finish()
    }
}

/// Size of trap frame in bytes (for assembly calculations)
pub const TRAP_FRAME_SIZE: usize = core::mem::size_of::<TrapFrame>();

// Compile-time assertions to ensure trap frame layout
const _: () = assert!(TRAP_FRAME_SIZE == 36 * 8); // 31 GPRs + 5 special registers = 36 u64s
const _: () = assert!(core::mem::align_of::<TrapFrame>() == 8); // Must be 8-byte aligned
