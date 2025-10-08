# KaaL Bootable Demo

Bootable KaaL system demonstrating the Rust elfloader.

## Quick Build

```bash
# From project root
./tools/build-bootimage.sh
```

Or with custom project:

```bash
./tools/build-bootimage.sh \
  --project examples/bootable-demo \
  --lib libkaal_bootable_demo.a
```

## Test in QEMU

```bash
qemu-system-aarch64 -machine virt -cpu cortex-a53 -nographic -kernel bootimage.elf
```

Press Ctrl-A then X to exit.

## What's Inside

This example shows:
- Rust elfloader booting seL4
- KaaL root task with capability broker
- Component spawning
- Debug output via seL4_DebugPutChar

See [src/main.rs](src/main.rs) for implementation.
