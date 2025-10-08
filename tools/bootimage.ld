/* Linker script for KaaL bootable image */

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
    .kernel_elf ALIGN(4K) : {
        __kernel_image_start = .;
        _kernel_start = .;
        KEEP(*(.kernel_elf))
        __kernel_image_end = .;
        _kernel_end = .;
    }

    /* Embedded user/root task */
    .roottask_data ALIGN(4K) : {
        __user_image_start = .;
        _roottask_start = .;
        KEEP(*(.roottask_data))
        __user_image_end = .;
        _roottask_end = .;
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
