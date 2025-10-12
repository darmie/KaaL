# KaaL Microkernel

A capability-based microkernel written in Rust for ARM64 (AArch64).

## Overview

The KaaL microkernel is a from-scratch implementation of a capability-based operating system kernel in Rust. It provides:

- **Capability-based security** - All access control via unforgeable tokens
- **IPC (Inter-Process Communication)** - Message passing between user-space processes
- **Memory management** - Virtual memory, page tables, and address spaces
- **Thread scheduling** - Preemptive multitasking
- **Hardware abstraction** - ARM64 registers, UART, interrupts, timers

## Current Status: Chapter 1 Complete ✓

**Chapter 1: Bare Metal Boot & Early Init**

Implemented:
- ✅ ARM64 boot sequence (_start entry point)
- ✅ Early UART initialization for debug output
- ✅ Device tree (DTB) parsing
- ✅ Boot parameter passing from elfloader
- ✅ Basic kernel initialization

## Architecture

```
┌─────────────────────────────────────┐
│  User Space                          │
│  ┌──────────┐  ┌──────────┐         │
│  │Root Task │  │  Apps    │         │
│  └──────────┘  └──────────┘         │
├─────────────────────────────────────┤ ← syscall boundary
│  KaaL Microkernel                   │
│  ┌─────────────────────────────────┐│
│  │ Capability System                ││
│  ├─────────────────────────────────┤│
│  │ IPC & Message Passing           ││
│  ├─────────────────────────────────┤│
│  │ Memory Management (MMU)         ││
│  ├─────────────────────────────────┤│
│  │ Thread Scheduler                ││
│  ├─────────────────────────────────┤│
│  │ Hardware Abstraction (ARM64)    ││
│  └─────────────────────────────────┘│
├─────────────────────────────────────┤
│  Hardware (ARM64)                   │
│  CPU, MMU, GIC, UART, Timers        │
└─────────────────────────────────────┘
```

## Building

The kernel is built via the project-level config-driven build system:

```bash
# Build for QEMU virt platform (default)
cd /path/to/kaal
./build.sh

# Build for Raspberry Pi 4
./build.sh --platform rpi4

# Build for custom platform
./build.sh --platform my-platform
```

The build system:
1. Reads platform config from `build-config.toml`
2. Generates `kernel/kernel.ld` with correct memory addresses
3. Builds kernel with `cargo build --target aarch64-unknown-none`
4. Embeds kernel into elfloader for booting

### Manual Build (Advanced)

```bash
cd kernel

# Generate linker script (normally done by build.sh)
cat > kernel.ld << 'EOF'
OUTPUT_FORMAT("elf64-littleaarch64")
OUTPUT_ARCH(aarch64)
ENTRY(_start)

SECTIONS {
    . = 0x40400000;  /* Platform-specific kernel address */
    _kernel_start = .;
    .text : { KEEP(*(.text._start)) *(.text .text.*) }
    .rodata : ALIGN(4096) { *(.rodata .rodata.*) }
    .data : ALIGN(4096) { *(.data .data.*) }
    .bss : ALIGN(4096) { __bss_start = .; *(.bss .bss.*) *(COMMON) __bss_end = .; }
    .stack (NOLOAD) : ALIGN(4096) { . = . + 0x4000; __stack_top = .; }
    _kernel_end = .;
}
EOF

# Build
RUSTFLAGS="-C link-arg=-Tkernel.ld" \
cargo build --release --target aarch64-unknown-none -Z build-std=core,alloc
```

## Memory Layout (QEMU virt)

```
0x40400000  ┌─────────────────┐  _kernel_start
            │  .text           │  ← Entry point (_start)
            ├─────────────────┤
0x40402000  │  .rodata         │  ← Read-only data, strings
            ├─────────────────┤
0x40403000  │  .data           │  ← Initialized data
            ├─────────────────┤
            │  .bss            │  ← Uninitialized data (zeroed)
            ├─────────────────┤
            │  .stack          │  ← Kernel stack (16KB)
            │  (grows down)    │
            └─────────────────┘  _kernel_end
```

## Boot Sequence

1. **Elfloader** loads kernel ELF into memory at kernel address
2. **Elfloader** parses ELF segments and copies to virtual addresses
3. **Elfloader** sets up boot parameters in registers:
   - `x0` = DTB (Device Tree Blob) address
   - `x1` = Root task image address
   - `x2` = Root task image end
   - `x3` = Root task entry point
   - `x4` = Physical-virtual offset
4. **Elfloader** jumps to kernel entry point (`_start`)
5. **Kernel** initializes:
   - Clear BSS section
   - Set up stack pointer
   - Initialize UART for debug output
   - Parse device tree
   - Print boot banner
   - Enter main kernel initialization

## Code Structure

```
kernel/
├── src/
│   ├── arch/
│   │   └── aarch64/
│   │       ├── mod.rs          # ARM64 architecture module
│   │       ├── registers.rs    # CPU register access
│   │       └── uart.rs         # UART driver
│   ├── boot/
│   │   ├── mod.rs              # Boot sequence and entry point
│   │   └── dtb.rs              # Device tree parsing (Chapter 1 stub)
│   ├── debug/
│   │   └── mod.rs              # Debug output (println!)
│   └── lib.rs                  # Kernel library root
├── Cargo.toml                  # Kernel dependencies
└── rust-toolchain.toml         # Rust nightly version pin
```

## Key Files

### [src/boot/mod.rs](src/boot/mod.rs)
- `_start` - Kernel entry point (called by elfloader)
- `kernel_entry` - Rust entry point after early init
- Boot parameter parsing
- DTB parsing (basic in Chapter 1)

### [src/arch/aarch64/uart.rs](src/arch/aarch64/uart.rs)
- UART initialization
- Character output (`putc`)
- String output (`puts`)
- Used by debug macros

### [src/arch/aarch64/registers.rs](src/arch/aarch64/registers.rs)
- ARM64 system register definitions
- UART register mapping
- Memory-mapped I/O helpers

### [src/debug/mod.rs](src/debug/mod.rs)
- `println!` macro for kernel debugging
- `DebugWriter` implementing `core::fmt::Write`

## Boot Parameters

The kernel receives these parameters from the elfloader:

| Register | Parameter | Description |
|----------|-----------|-------------|
| `x0` | DTB address | Device tree blob physical address |
| `x1` | Root task start | User-space root task image start |
| `x2` | Root task end | User-space root task image end |
| `x3` | Root entry | Root task entry point address |
| `x4` | PV offset | Physical to virtual offset (0 for Chapter 1) |

## Debug Output

The kernel uses UART for debug output:

```rust
use crate::debug::println;

println!("Kernel booting...");
println!("DTB at: {:#x}", dtb_addr);
```

UART is memory-mapped at:
- **QEMU virt**: `0x09000000` (PL011 UART)
- **RPi4**: `0xFE201000` (Mini UART)

## Device Tree (DTB)

The kernel receives a device tree blob from the elfloader containing:
- System model name
- Memory regions and sizes
- Device addresses and interrupts
- CPU topology

Chapter 1 implements basic DTB parsing to extract:
- Model string
- Memory base and size

Full DTB parsing will be implemented in later chapters.

## Dependencies

```toml
[dependencies]
spin = "0.9"           # Spinlock for synchronization
bitflags = "2.9"       # Bitfield helpers

[build-dependencies]
# None - linker script generated by build system
```

## Development Roadmap

### ✅ Chapter 1: Bare Metal Boot & Early Init
- ARM64 boot sequence
- UART debug output
- Device tree parsing (basic)
- Boot parameter handling

### 🔲 Chapter 2: Memory Management
- Page table setup (TTBR0/TTBR1)
- Virtual memory mapping
- Memory allocator
- Kernel heap

### 🔲 Chapter 3: Exception Handling
- Exception vectors
- Interrupt handling
- System call interface
- Timer interrupts

### 🔲 Chapter 4: Scheduling & IPC
- Thread scheduler
- Context switching
- IPC endpoints
- Message passing

### 🔲 Chapter 5: Capabilities
- Capability space
- CNode (capability nodes)
- Capability derivation
- Access control

### 🔲 Chapter 6: User Space
- Root task startup
- User-space page tables
- System call implementation
- ELF loading

## Platform Support

Currently supported platforms:

| Platform | Status | CPU | Memory | UART |
|----------|--------|-----|--------|------|
| QEMU virt | ✅ Working | Cortex-A53 | 128MB @ 0x40000000 | PL011 @ 0x09000000 |
| Raspberry Pi 4 | 🔧 In progress | Cortex-A72 | 1GB @ 0x0 | Mini UART @ 0xFE201000 |
| Generic ARM64 | 📝 Template | Configurable | Configurable | Configurable |

Add new platforms by configuring `build-config.toml` at the project root.

## Testing

```bash
# Build and run in QEMU
cd /path/to/kaal
./build.sh
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader

# Expected output:
# ═══════════════════════════════════════════════════════════
#   KaaL Rust Microkernel v0.1.0
#   Chapter 1: Bare Metal Boot & Early Init
# ═══════════════════════════════════════════════════════════
#
# Boot parameters:
#   DTB:         0x40000000
#   Root task:   0x4021a000 - 0x4021a428
#   Entry:       0x210120
#   PV offset:   0x0
#
# Parsing device tree...
# [DTB parsing output...]
```

## Debugging

### QEMU with GDB

```bash
# Terminal 1: Start QEMU with GDB stub
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel elfloader -s -S

# Terminal 2: Connect GDB
aarch64-none-elf-gdb kernel/target/aarch64-unknown-none/release/kaal-kernel
(gdb) target remote :1234
(gdb) b kernel_entry
(gdb) c
```

### Adding Debug Output

```rust
use crate::debug::println;

println!("Debug: value = {:#x}", value);
```

## License

MIT OR Apache-2.0

## See Also

- [../BUILD_SYSTEM.md](../BUILD_SYSTEM.md) - Build system documentation
- [../runtime/elfloader/README.md](../runtime/elfloader/README.md) - Bootloader documentation
- [../build-config.toml](../build-config.toml) - Platform configurations
