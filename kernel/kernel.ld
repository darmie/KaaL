OUTPUT_FORMAT("elf64-littleaarch64")
OUTPUT_ARCH(aarch64)
ENTRY(_start)

SECTIONS
{
    . = 0x40400000;
    _kernel_start = .;

    .text : {
        KEEP(*(.text._start))
        *(.text .text.*)
    }

    .rodata : ALIGN(4096) { *(.rodata .rodata.*) }
    .data : ALIGN(4096) { *(.data .data.*) }

    .bss : ALIGN(4096) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    .stack (NOLOAD) : ALIGN(4096) {
        . = . + 0x4000;
        __stack_top = .;
    }

    _kernel_end = .;
}
