#![no_std]
#![no_main]

use core::arch::global_asm;

// Kernel entry point - save boot parameters and call kernel_entry
//
// Elfloader passes parameters in x0-x5:
//   x0 = user_img_start, x1 = user_img_end, x2 = pv_offset
//   x3 = user_entry, x4 = dtb_addr, x5 = dtb_size
//
// Kernel saves parameters in x19-x24:
//   x19 = dtb_addr, x20 = root_p_start, x21 = root_p_end
//   x22 = root_v_entry, x23 = pv_offset, x24 = dtb_size
global_asm!(
    ".section .text._start",
    ".global _start",
    ".type _start, @function",
    "_start:",
    "    // Enable FP/SIMD before anything else (CPACR_EL1.FPEN = 0b11)",
    "    mrs x10, cpacr_el1",
    "    orr x10, x10, #(0x3 << 20)",
    "    msr cpacr_el1, x10",
    "    isb",
    "    // Save boot parameters",
    "    mov x19, x4",      // x19 = dtb_addr (from x4)
    "    mov x20, x0",      // x20 = user_img_start (from x0)
    "    mov x21, x1",      // x21 = user_img_end (from x1)
    "    mov x22, x3",      // x22 = user_entry (from x3)
    "    mov x23, x2",      // x23 = pv_offset (from x2)
    "    mov x24, x5",      // x24 = dtb_size (from x5)
    "    b {kernel_entry}", // Jump to kernel_entry
    kernel_entry = sym kaal_kernel::boot::kernel_entry,
);

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
