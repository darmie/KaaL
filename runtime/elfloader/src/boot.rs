// Boot sequence management

use crate::uart_println;
use crate::BootInfo;

/// Symbols provided by linker script for embedded kernel
extern "C" {
    static __kernel_image_start: u8;
    static __kernel_image_end: u8;
}

/// Symbols provided by linker script for embedded root task
extern "C" {
    static __user_image_start: u8;
    static __user_image_end: u8;
}

pub fn load_images() -> BootInfo {
    uart_println!("Loading embedded images from ELF sections...");

    // Get kernel image from .kernel_elf section
    let (kernel_start, kernel_end) = unsafe {
        (
            &__kernel_image_start as *const u8 as usize,
            &__kernel_image_end as *const u8 as usize,
        )
    };

    let kernel_size = kernel_end - kernel_start;
    uart_println!("  Kernel: {:#x} - {:#x} ({} KB)", kernel_start, kernel_end, kernel_size / 1024);

    // Get root task from .roottask_data section
    let (user_start, user_end) = unsafe {
        (
            &__user_image_start as *const u8 as usize,
            &__user_image_end as *const u8 as usize,
        )
    };

    let user_size = user_end - user_start;
    uart_println!("  User:   {:#x} - {:#x} ({} KB)", user_start, user_end, user_size / 1024);

    // For now, we'll load kernel at fixed physical address 0x40000000
    // This matches seL4's expected kernel load address for QEMU ARM virt
    let kernel_paddr = 0x40000000;

    uart_println!("Copying kernel to physical address {:#x}...", kernel_paddr);

    // Copy kernel to target physical address
    unsafe {
        let src = kernel_start as *const u8;
        let dst = kernel_paddr as *mut u8;
        core::ptr::copy_nonoverlapping(src, dst, kernel_size);
    }

    uart_println!("Images loaded successfully!");

    // Return kernel entry point and user image location
    // seL4 kernel entry is at the start of kernel image
    BootInfo {
        user_img_start: user_start,
        user_img_end: user_end,
        pv_offset: 0, // Identity mapped for now
        user_entry: kernel_paddr, // Jump to kernel entry
        dtb_addr: 0, // Will be set from dtb parameter
        dtb_size: 0,
    }
}
