// ARM64-specific boot code

use core::arch::{asm, naked_asm};

/// ARM64 entry point - called by boot firmware
/// x0 = DTB physical address
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Preserve DTB address in x0
        "mov x19, x0",

        // Set up stack (use end of elfloader as stack base)
        "ldr x1, =__stack_top",
        "mov sp, x1",

        // Clear BSS
        "ldr x1, =__bss_start",
        "ldr x2, =__bss_end",
        "1:",
        "cmp x1, x2",
        "b.eq 2f",
        "str xzr, [x1], #8",
        "b 1b",
        "2:",

        // Restore DTB address to x0 and jump to Rust
        "mov x0, x19",
        "bl _start_rust",

        // Should never return
        "3:",
        "wfe",
        "b 3b",
    )
}

/// Rust entry point - called from assembly _start
#[no_mangle]
extern "C" fn _start_rust(dtb_addr: usize) -> ! {
    // DTB address should be passed from firmware/bootloader in x0
    // If x0 is 0, use platform-specific fallback
    let dtb_addr = if dtb_addr != 0 {
        dtb_addr
    } else {
        #[cfg(feature = "platform-qemu-virt")]
        {
            // QEMU virt machine places DTB at 0x40000000 (RAM base)
            0x40000000
        }

        #[cfg(not(feature = "platform-qemu-virt"))]
        {
            // No fallback available - pass 0 and let main handle the error
            0
        }
    };

    // Call main elfloader entry
    crate::elfloader_main(dtb_addr)
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
