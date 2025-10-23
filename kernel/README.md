# KaaL Microkernel

A capability-based microkernel written in Rust for ARM64 (AArch64).

## Overview

The KaaL microkernel is a from-scratch implementation of a capability-based operating system kernel in Rust, inspired by seL4's security model. It provides the minimal mechanism necessary for building secure systems, with all policy decisions delegated to userspace.

**Core Features:**

- **Capability-based security** - All access control via unforgeable capability tokens
- **seL4-style resource delegation** - UntypedMemory can be retyped into kernel objects
- **IPC (Inter-Process Communication)** - Fast synchronous and asynchronous message passing
- **Memory management** - 4-level ARM64 page tables with full virtual memory support
- **Preemptive scheduling** - Round-robin scheduler with configurable time slices
- **Hardware abstraction** - GIC interrupt controller, UART, timers, MMU

## Architecture

```
┌─────────────────────────────────────┐
│  User Space (EL0)                   │
│  ┌──────────┐  ┌──────────────┐    │
│  │Root Task │→ │ system_init  │    │
│  └──────────┘  └──────────────┘    │
│                      ↓              │
│             ┌────────────────┐      │
│             │  Applications  │      │
│             └────────────────┘      │
├─────────────────────────────────────┤ ← syscall boundary (SVC)
│  KaaL Microkernel (EL1)             │
│  ┌─────────────────────────────────┐│
│  │ Capability System (CNode/CDT)   ││
│  ├─────────────────────────────────┤│
│  │ IPC (Endpoint/Notification)     ││
│  ├─────────────────────────────────┤│
│  │ Memory Management (VSpace/Page) ││
│  ├─────────────────────────────────┤│
│  │ Thread Scheduler (TCB/Ready Q)  ││
│  ├─────────────────────────────────┤│
│  │ Interrupt Handling (GIC/IRQ)    ││
│  ├─────────────────────────────────┤│
│  │ Hardware Abstraction (ARM64)    ││
│  └─────────────────────────────────┘│
├─────────────────────────────────────┤
│  Hardware (ARM64)                   │
│  CPU, MMU, GIC, UART, Timers        │
└─────────────────────────────────────┘
```

## Design Philosophy

**Minimal Mechanism, Not Policy:**

The kernel provides only the mechanisms for isolation, communication, and resource management. All policy decisions (which processes to run, how to allocate memory, security policies) are made in userspace.

**Capability-Based Security:**

Every operation requires a capability - an unforgeable token granting specific rights. Capabilities are stored in CSpace (capability space) and looked up by index. The kernel tracks capability derivation through a Capability Derivation Tree (CDT), enabling revocation.

**Resource Delegation Model:**

Instead of the kernel allocating memory for processes, the kernel delegates `UntypedMemory` capabilities to userspace. Userspace can then "retype" untyped memory into kernel objects (TCBs, page tables, endpoints, etc.) via `sys_retype`. This follows seL4's design.

**Verified Core:**

22 kernel modules are verified using Verus (SMT-based verification tool for Rust), ensuring memory safety and key invariants. Verification is incremental and optional.

## Kernel Objects

The kernel manages the following object types:

| Object Type | Purpose |
|-------------|---------|
| **TCB** | Thread Control Block - represents a thread |
| **CNode** | Capability Node - stores capabilities |
| **CNodeCdt** | CNode with Capability Derivation Tree tracking |
| **Endpoint** | Synchronous IPC - blocking send/receive |
| **Notification** | Asynchronous IPC - signals and waiting |
| **VSpace** | Virtual address space root (page table) |
| **Page** | Physical memory page |
| **PageTable** | Page table level 1/2/3 |
| **UntypedMemory** | Raw physical memory for retyping |
| **IRQControl** | Permission to create IRQ handlers |
| **IRQHandler** | Handle specific IRQ number |

## System Calls

The kernel provides a minimal syscall interface (following seL4 design):

### IPC & Synchronization

- `sys_send` (0x01) - Send message to endpoint
- `sys_recv` (0x02) - Receive message from endpoint
- `sys_call` (0x03) - Send + receive (RPC pattern)
- `sys_reply` (0x04) - Reply to caller
- `sys_wait` (0x10) - Wait for notification signal
- `sys_signal` (0x11) - Signal a notification

### Thread Control

- `sys_yield` (0x05) - Yield to scheduler
- `sys_thread_suspend` (0x06) - Suspend thread
- `sys_thread_resume` (0x07) - Resume thread

### Memory Management

- `sys_memory_map` (0x20) - Map page into address space
- `sys_memory_unmap` (0x21) - Unmap page from address space
- `sys_memory_protect` (0x22) - Change page permissions
- `sys_retype` (0x26) - Convert UntypedMemory into kernel object

### Capability Operations

- `sys_cap_copy` (0x30) - Copy capability to another CSpace slot
- `sys_cap_move` (0x31) - Move capability to another CSpace slot
- `sys_cap_delete` (0x32) - Delete capability
- `sys_cap_revoke` (0x33) - Revoke all derived capabilities

### Interrupt Handling

- `sys_irq_handler_get` (0x40) - Create IRQ handler from IRQControl
- `sys_irq_handler_ack` (0x41) - Acknowledge handled interrupt

### Debug

- `sys_debug_putchar` (0x50) - Print character (debug builds only)

## Capability-Based Resource Allocation

KaaL implements seL4's capability-based resource allocation model:

```
Kernel Boot
  ↓
Allocates UntypedMemory from physical RAM
  ↓
Delegates to root-task via initial CSpace
  ↓
root-task uses sys_retype to create:
  - Child UntypedMemory for system_init
  - TCBs, VSpaces, CNodes, Endpoints
  ↓
system_init spawns applications using delegated Untyped
```

### Example: Creating a Thread

```rust
// 1. Retype UntypedMemory into a TCB
sys_retype(
    untyped_cap,     // Source: UntypedMemory capability
    ObjectType::Tcb, // Target type
    tcb_cap,         // Destination slot for new TCB cap
)?;

// 2. Configure the TCB
sys_tcb_configure(tcb_cap, cspace_root, vspace_root, entry_point)?;

// 3. Resume the thread
sys_thread_resume(tcb_cap)?;
```

## Memory Layout

### Kernel Memory (QEMU virt platform)

```
0x40400000  ┌─────────────────┐  _kernel_start
            │  .text           │  ← Code, entry point (_start)
            ├─────────────────┤
            │  .rodata         │  ← Constants, strings
            ├─────────────────┤
            │  .data           │  ← Initialized globals
            ├─────────────────┤
            │  .bss            │  ← Uninitialized data (zeroed)
            ├─────────────────┤
            │  Kernel heap     │  ← Dynamic allocations
            ├─────────────────┤
            │  Page allocator  │  ← Physical page tracking
            └─────────────────┘  _kernel_end
```

### Virtual Address Space Layout

```
User Space (TTBR0_EL1):
0x0000_0000_0000_0000  ┌─────────────────┐
                       │  User code       │
                       ├─────────────────┤
                       │  User heap       │
                       ├─────────────────┤
                       │  User stack      │
0x0000_FFFF_FFFF_FFFF  └─────────────────┘

Kernel Space (TTBR1_EL1):
0xFFFF_0000_0000_0000  ┌─────────────────┐
                       │  Kernel code     │
                       ├─────────────────┤
                       │  Kernel heap     │
                       ├─────────────────┤
                       │  Device MMIO     │
0xFFFF_FFFF_FFFF_FFFF  └─────────────────┘
```

## Boot Sequence

1. **Elfloader** (EL2) loads kernel into memory
2. **Elfloader** sets up boot parameters in registers:
   - `x0` = DTB (Device Tree Blob) address
   - `x1` = Root task image start
   - `x2` = Root task image end
   - `x3` = Root task entry point
   - `x4` = Physical-virtual offset
3. **Elfloader** jumps to kernel `_start`
4. **Kernel** (_start assembly):
   - Clear BSS section
   - Set up stack pointer
   - Jump to Rust entry point
5. **Kernel** (Rust initialization):
   - Initialize UART for debug output
   - Parse device tree for memory regions
   - Set up page tables (TTBR0/TTBR1)
   - Enable MMU
   - Initialize GIC interrupt controller
   - Set up exception vectors
   - Initialize page allocator
   - Create root-task's initial capabilities
   - Load root-task ELF into userspace
   - Switch to EL0 and jump to root-task

## IPC Fast Path

The kernel implements a fast path for common IPC patterns:

```rust
// Fast path conditions:
// 1. Endpoint has exactly one waiting receiver
// 2. Receiver priority >= sender priority
// 3. No capability transfer
// 4. Message fits in registers (4 words)

// Fast path execution:
// 1. Copy message registers directly (no buffering)
// 2. Switch context to receiver
// 3. No scheduler involvement
// ~1000 cycles on ARM64
```

## Building

The kernel is built via the project-level config-driven build system:

```bash
# Build for QEMU virt platform
cd /path/to/kaal
nu build.nu

# Build for specific platform
nu build.nu --platform rpi4

# Build with verification
nu build.nu --verify
```

The build system:

1. Reads platform config from `build-config.toml`
2. Generates `kernel/kernel.ld` with correct memory addresses
3. Builds kernel with `cargo build --target aarch64-unknown-none`
4. Optionally runs Verus verification
5. Embeds kernel into elfloader bootimage

## Code Structure

```text
kernel/
├── src/
│   ├── arch/
│   │   └── aarch64/
│   │       ├── boot.rs          # Boot assembly (_start)
│   │       ├── exception.rs     # Exception vectors
│   │       ├── mmu.rs           # MMU/page table setup
│   │       ├── gic.rs           # GIC interrupt controller
│   │       ├── context.rs       # Context switching
│   │       └── uart.rs          # UART driver
│   ├── objects/
│   │   ├── tcb.rs               # Thread Control Block
│   │   ├── cnode.rs             # Capability Node
│   │   ├── cnode_cdt.rs         # CNode with CDT tracking
│   │   ├── endpoint.rs          # Synchronous IPC
│   │   ├── notification.rs      # Asynchronous IPC
│   │   ├── vspace.rs            # Virtual address space
│   │   ├── page.rs              # Physical page
│   │   └── untyped.rs           # UntypedMemory
│   ├── syscall/
│   │   └── mod.rs               # System call dispatcher
│   ├── sched/
│   │   └── mod.rs               # Round-robin scheduler
│   ├── ipc/
│   │   └── mod.rs               # IPC implementation
│   └── lib.rs                   # Kernel entry point
├── Cargo.toml                   # Dependencies
└── kernel.ld                    # Linker script (generated)
```

## Verification

22 kernel modules are verified using [Verus](https://github.com/verus-lang/verus):

```rust
// Example: Verified IPC endpoint
verus! {
    pub fn endpoint_send(ep: &mut Endpoint, msg: Message)
        requires ep.invariant(),
        ensures ep.invariant(),
        ensures ep.has_message(),
    {
        ep.queue.push(msg);
    }
}
```

Verified properties include:

- Memory safety (no null/dangling pointers)
- Capability invariants (no forging, proper derivation)
- IPC correctness (messages not lost/corrupted)
- Scheduler fairness (every thread runs eventually)

Run verification:

```bash
nu build.nu --verify
```

## Platform Support

| Platform | Status | CPU | Memory | UART |
|----------|--------|-----|--------|------|
| QEMU virt | ✅ Working | Cortex-A53 | 128MB @ 0x40000000 | PL011 @ 0x09000000 |
| Raspberry Pi 4 | 🔧 In progress | Cortex-A72 | 1GB @ 0x0 | Mini UART @ 0xFE201000 |

Add new platforms by configuring `build-config.toml`.

## Testing

```bash
# Build and run
nu build.nu
nu run.nu

# Expected: Boot to root-task, spawn system_init, then applications
```

## Performance Characteristics

Based on design and benchmarks from similar systems:

| Operation | Approximate Cycles |
|-----------|-------------------|
| IPC fast path | ~1000 |
| Context switch | ~500 |
| Syscall overhead | ~200 |
| Page fault | ~1500 |
| Capability lookup | ~50 |

Rust's zero-cost abstractions ensure C-like performance.

## Debugging

### QEMU with GDB

```bash
# Terminal 1: Start QEMU with GDB stub
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M -nographic \
  -kernel build/output/bootimage.elf -s -S

# Terminal 2: Connect GDB
aarch64-none-elf-gdb kernel/target/aarch64-unknown-none/release/kaal-kernel
(gdb) target remote :1234
(gdb) b kernel_entry
(gdb) c
```

### Debug Output

```rust
use crate::kprintln;

kprintln!("Syscall: sys_send ep={}", ep_cap);
```

## Dependencies

```toml
[dependencies]
spin = "0.9"              # Spinlocks
bitflags = "2.9"          # Bitfield helpers
verus-macros = "0.1"      # Verification annotations
```

## License

MIT OR Apache-2.0

## See Also

- [BUILD_SYSTEM.md](../BUILD_SYSTEM.md) - Build system documentation
- [docs/MICROKERNEL_CHAPTERS.md](../docs/MICROKERNEL_CHAPTERS.md) - Development roadmap
- [docs/RUST_MICROKERNEL_DESIGN.md](../docs/RUST_MICROKERNEL_DESIGN.md) - Design details
- [runtime/root-task/README.md](../runtime/root-task/README.md) - Root task documentation
