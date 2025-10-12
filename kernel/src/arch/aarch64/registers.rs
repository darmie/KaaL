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
