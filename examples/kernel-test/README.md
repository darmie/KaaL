# KaaL Rust Microkernel - Chapter 1 Test

Tests the pure Rust microkernel (Chapter 1: Bare Metal Boot & Early Init) on QEMU ARM64.

## What This Tests

This is **Chapter 1** of the KaaL microkernel development:

✅ **Bare Metal Boot** - ARM64 boot sequence without any dependencies
✅ **UART Driver** - PL011 UART for serial console output
✅ **DTB Parsing** - Device Tree Blob parsing to discover hardware
✅ **Debug Logging** - Compile-time configurable logging system
✅ **Elfloader Integration** - Boots via Rust elfloader

## Quick Test

```bash
./test-kernel.sh
```

This script:
1. Builds the kernel staticlib
2. Packages it into a bootable image with our Rust elfloader
3. Runs it in QEMU ARM64 virt machine
4. Displays kernel output

## Expected Output

```
═══════════════════════════════════════════════════════════
  KaaL Rust Microkernel v0.1.0
  Chapter 1: Bare Metal Boot & Early Init
═══════════════════════════════════════════════════════════

Device tree parsed successfully:
  Model:       QEMU ARM virt
  Memory:      Region 0: 0x40000000 - 0x60000000 (512 MB)

═══════════════════════════════════════════════════════════
  Kernel Initialized - Entering Idle Loop
═══════════════════════════════════════════════════════════
```

## Architecture

```
┌─────────────────────────────────────────┐
│       Rust Elfloader                    │
│  - Loads kernel to 0x40000000           │
│  - Sets up MMU & page tables            │
│  - Passes DTB address in x0             │
│  - Jumps to kernel _start               │
└───────────────┬─────────────────────────┘
                ↓
┌─────────────────────────────────────────┐
│       KaaL Rust Microkernel             │
│  - Pure Rust, no_std                    │
│  - ARM64 boot sequence                  │
│  - PL011 UART driver                    │
│  - DTB parser                           │
│  - Configurable logging                 │
│  - No dependencies (standalone)         │
└─────────────────────────────────────────┘
```

## Build Details

The kernel is built as a staticlib (`libkaal_kernel.a`) with:
- Target: `aarch64-unknown-none`
- Optimization: Size (`opt-level = "z"`)
- LTO: Enabled
- Features: Compile-time log levels (ERROR/WARN/INFO/DEBUG/TRACE)

Link with custom linker script: `kernel/kernel.ld`

## Log Levels

Build with different log levels:

```bash
# Only ERROR messages
cd kernel && cargo build --release --features log-error

# INFO and above (default)
cd kernel && cargo build --release --features log-info

# All messages including TRACE
cd kernel && cargo build --release --features log-trace
```

## Next Steps (Chapter 2)

- [ ] Implement exception handlers
- [ ] Add timer interrupt support
- [ ] Memory management (physical page allocator)
- [ ] Capability table setup
- [ ] IPC foundations

## See Also

- [kernel/](../../kernel/) - Kernel source code
- [kernel/src/boot/mod.rs](../../kernel/src/boot/mod.rs) - Boot sequence
- [kernel/src/boot/dtb.rs](../../kernel/src/boot/dtb.rs) - DTB parser
- [kernel/src/arch/aarch64/uart.rs](../../kernel/src/arch/aarch64/uart.rs) - UART driver
- [kernel/src/debug/mod.rs](../../kernel/src/debug/mod.rs) - Logging system
- [docs/MICROKERNEL_CHAPTERS.md](../../docs/MICROKERNEL_CHAPTERS.md) - Development roadmap
