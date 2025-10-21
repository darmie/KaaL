/* Component linker script */
ENTRY(_start)

MEMORY
{
  /* Component memory space starts at 2MB */
  RAM : ORIGIN = 0x200000, LENGTH = 2M
}

SECTIONS
{
  /* Code starts at 2MB */
  .text 0x200000 : AT(0x200000)
  {
    /* Ensure _start is placed first */
    KEEP(*(.text._start))
    KEEP(*(.text.entry))
    *(.text .text.*)
  } > RAM

  .rodata : ALIGN(8)
  {
    *(.rodata .rodata.*)
  } > RAM

  .data : ALIGN(8)
  {
    *(.data .data.*)
  } > RAM

  .bss : ALIGN(8)
  {
    *(.bss .bss.*)
    *(COMMON)
  } > RAM

  /* Discard unwanted sections */
  /DISCARD/ :
  {
    *(.ARM.exidx)
    *(.ARM.exidx.*)
    *(.ARM.extab)
    *(.ARM.extab.*)
    *(.comment)
    *(.debug*)
  }
}