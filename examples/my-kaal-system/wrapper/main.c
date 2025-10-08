/*
 * KaaL seL4 Wrapper - C Entry Point
 *
 * This wrapper integrates KaaL with seL4's boot infrastructure.
 * It receives boot info from seL4 and passes it to the Rust runtime.
 */

#include <stdio.h>
#include <sel4/sel4.h>
#include <sel4platsupport/bootinfo.h>

/* Rust entry point */
extern void kaal_main(seL4_BootInfo *bootinfo);

int main(void) {
    seL4_BootInfo *info = platsupport_get_bootinfo();

    printf("\n");
    printf("===========================================\n");
    printf("  KaaL Root Task Starting\n");
    printf("===========================================\n");
    printf("  Boot Info:\n");
    printf("    IPC Buffer:      %p\n", (void*)info->ipcBuffer);
    printf("    Empty Slots:     [%lu-%lu)\n",
           (unsigned long)info->empty.start,
           (unsigned long)info->empty.end);
    printf("    User Image:      [%p-%p)\n",
           (void*)info->userImageFrames.start,
           (void*)info->userImageFrames.end);
    printf("===========================================\n\n");

    /* Transfer control to Rust KaaL runtime */
    kaal_main(info);

    /* Should never return */
    printf("ERROR: kaal_main returned!\n");
    return 1;
}
