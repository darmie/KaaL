/*
 * KaaL seL4 Wrapper - C Entry Point
 *
 * This is the C wrapper that sel4test builds. It calls into our Rust root task.
 */

#include <stdio.h>
#include <sel4/sel4.h>
#include <sel4platsupport/bootinfo.h>

/* Rust entry point from src/main.rs */
extern void _start(void) __attribute__((noreturn));

int main(void) {
    seL4_BootInfo *info = platsupport_get_bootinfo();

    printf("\n");
    printf("===========================================\n");
    printf("  KaaL Root Task Wrapper\n");
    printf("===========================================\n");
    printf("  Boot Info:\n");
    printf("    IPC Buffer:      %p\n", (void*)info->ipcBuffer);
    printf("    Empty Slots:     [%lu-%lu)\n",
           (unsigned long)info->empty.start,
           (unsigned long)info->empty.end);
    printf("===========================================\n");
    printf("  Calling Rust _start()...\n\n");

    /* Transfer control to Rust KaaL root task */
    _start();

    /* Should never return */
    printf("ERROR: _start returned!\n");
    while(1);
}
