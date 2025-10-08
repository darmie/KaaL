/* Linker script for KaaL elfloader */

ENTRY(_start)

SECTIONS
{
    /* Load at 256MB - below typical kernel load address */
    . = 0x10000000;

    .text.boot : {
        KEEP(*(.text.boot))
    }

    .text : {
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    /* Embedded kernel image */
    .rodata.kernel : {
        __kernel_image_start = .;
        KEEP(*(.rodata.kernel))
        __kernel_image_end = .;
    }

    /* Embedded user image */
    .rodata.user : {
        __user_image_start = .;
        KEEP(*(.rodata.user))
        __user_image_end = .;
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* Stack grows downward */
    . = ALIGN(4096);
    __stack_bottom = .;
    . += 0x100000; /* 1MB stack */
    __stack_top = .;

    __elfloader_end = .;

    /DISCARD/ : {
        *(.note*)
        *(.comment*)
        *(.eh_frame*)
    }
}
