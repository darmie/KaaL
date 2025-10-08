// ARM64-specific boot code

use core::arch::{asm, naked_asm};

/// ARM64 entry point - called by boot firmware
/// x0 = DTB physical address
#[unsafe(naked)]
#[no_mangle]
#[link_section = ".text.boot"]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Preserve DTB address in x0
        "mov x19, x0",

        // Set up stack (use end of elfloader as stack base)
        "adrp x1, __stack_top",
        "add sp, x1, #0",

        // Clear BSS
        "adrp x1, __bss_start",
        "add x1, x1, #0",
        "adrp x2, __bss_end",
        "add x2, x2, #0",
        "1:",
        "cmp x1, x2",
        "b.eq 2f",
        "str xzr, [x1], #8",
        "b 1b",
        "2:",

        // Restore DTB address to x0 and jump to Rust
        "mov x0, x19",
        "bl elfloader_main",

        // Should never return
        "3:",
        "wfe",
        "b 3b",
    )
}

/// Get current exception level
pub fn get_current_el() -> u8 {
    let el: u64;
    unsafe {
        asm!("mrs {}, CurrentEL", out(reg) el, options(nomem, nostack));
    }
    ((el >> 2) & 3) as u8
}

/// Disable MMU
pub fn disable_mmu() {
    unsafe {
        asm!(
            "mrs x1, sctlr_el1",
            "bic x1, x1, #1",  // Clear M bit
            "msr sctlr_el1, x1",
            "isb",
            options(nomem, nostack)
        );
    }
}

/// Enable MMU
pub fn enable_mmu(ttbr0: usize, ttbr1: usize, mair: u64, tcr: u64) {
    unsafe {
        asm!(
            // Set translation table base registers
            "msr ttbr0_el1, {ttbr0}",
            "msr ttbr1_el1, {ttbr1}",

            // Set memory attribute indirection register
            "msr mair_el1, {mair}",

            // Set translation control register
            "msr tcr_el1, {tcr}",

            // Ensure changes are visible
            "isb",

            // Enable MMU
            "mrs x1, sctlr_el1",
            "orr x1, x1, #1",  // Set M bit
            "orr x1, x1, #(1 << 2)",  // Set C bit (data cache)
            "orr x1, x1, #(1 << 12)",  // Set I bit (instruction cache)
            "msr sctlr_el1, x1",
            "isb",

            ttbr0 = in(reg) ttbr0,
            ttbr1 = in(reg) ttbr1,
            mair = in(reg) mair,
            tcr = in(reg) tcr,
            options(nomem, nostack)
        );
    }
}

/// Data synchronization barrier
#[inline(always)]
pub fn dsb() {
    unsafe {
        asm!("dsb sy", options(nostack, preserves_flags));
    }
}

/// Instruction synchronization barrier
#[inline(always)]
pub fn isb() {
    unsafe {
        asm!("isb", options(nostack, preserves_flags));
    }
}

/// Invalidate entire TLB
pub fn tlb_invalidate_all() {
    unsafe {
        asm!(
            "tlbi vmalle1",
            "dsb sy",
            "isb",
            options(nostack)
        );
    }
}
