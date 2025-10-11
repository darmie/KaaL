/* KaaL Rust Microkernel Linker Script for ARM64 */

OUTPUT_FORMAT("elf64-littleaarch64")
OUTPUT_ARCH(aarch64)
ENTRY(_start)

SECTIONS
{
    /* Kernel loads at 1GB physical address */
    . = 0x40000000;

    _kernel_start = .;

    /* Text section - executable code */
    .text : {
        KEEP(*(.text._start))
        *(.text .text.*)
    }

    /* Read-only data */
    .rodata : ALIGN(4096) {
        *(.rodata .rodata.*)
    }

    /* Initialized data */
    .data : ALIGN(4096) {
        *(.data .data.*)
    }

    /* Uninitialized data (BSS) */
    .bss : ALIGN(4096) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* Stack (16KB for now) */
    .stack (NOLOAD) : ALIGN(4096) {
        . = . + 0x4000;
        __stack_top = .;
    }

    _kernel_end = .;

    /* Discard unwanted sections */
    /DISCARD/ : {
        *(.comment)
        *(.eh_frame)
        *(.note*)
    }
}
