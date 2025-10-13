//\! ARM64 system register access

use core::arch::asm;

/// Read MIDR_EL1 (Main ID Register)
#[inline(always)]
pub fn read_midr_el1() -> u64 {
    let val: u64;
    unsafe {
        asm!("mrs {}, midr_el1", out(reg) val);
    }
    val
}

/// Read current exception level
#[inline(always)]
pub fn current_el() -> u8 {
    let val: u64;
    unsafe {
        asm!("mrs {}, CurrentEL", out(reg) val);
    }
    ((val >> 2) & 0x3) as u8
}

/// Exception Syndrome Register (EL1)
pub struct ESR_EL1;
impl ESR_EL1 {
    #[inline(always)]
    pub fn read() -> u64 {
        let val: u64;
        unsafe {
            asm!("mrs {}, esr_el1", out(reg) val);
        }
        val
    }
}

/// Fault Address Register (EL1)
pub struct FAR_EL1;
impl FAR_EL1 {
    #[inline(always)]
    pub fn read() -> u64 {
        let val: u64;
        unsafe {
            asm!("mrs {}, far_el1", out(reg) val);
        }
        val
    }
}

/// Exception Link Register (EL1)
pub struct ELR_EL1;
impl ELR_EL1 {
    #[inline(always)]
    pub fn read() -> u64 {
        let val: u64;
        unsafe {
            asm!("mrs {}, elr_el1", out(reg) val);
        }
        val
    }

    #[inline(always)]
    pub fn write(val: u64) {
        unsafe {
            asm!("msr elr_el1, {}", in(reg) val);
        }
    }
}

/// Saved Program Status Register (EL1)
pub struct SPSR_EL1;
impl SPSR_EL1 {
    #[inline(always)]
    pub fn read() -> u64 {
        let val: u64;
        unsafe {
            asm!("mrs {}, spsr_el1", out(reg) val);
        }
        val
    }

    #[inline(always)]
    pub fn write(val: u64) {
        unsafe {
            asm!("msr spsr_el1, {}", in(reg) val);
        }
    }
}

/// Vector Base Address Register (EL1)
pub struct VBAR_EL1;
impl VBAR_EL1 {
    #[inline(always)]
    pub fn read() -> u64 {
        let val: u64;
        unsafe {
            asm!("mrs {}, vbar_el1", out(reg) val);
        }
        val
    }

    #[inline(always)]
    pub unsafe fn write(val: u64) {
        asm!("msr vbar_el1, {}", in(reg) val);
    }
}
