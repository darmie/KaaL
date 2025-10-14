# KaaL Rust Microkernel - Development Chapters

> **A chapter-by-chapter guide to building the KaaL capability-based microkernel in Rust**
>
> Each chapter is a self-contained milestone with documentation, implementation, and testing.

---

## Overview

This document structures the KaaL microkernel development into **8 distinct chapters**, each building upon the previous one. Each chapter has:
- Clear objectives and deliverables
- Step-by-step implementation guide
- Testing criteria
- Documentation requirements
- Estimated timeline

The KaaL microkernel is built from scratch in Rust, implementing capability-based security, IPC, memory management, and scheduling for ARM64 platforms.

**Total Timeline**: 9-12 months
**Commitment**: 1-2 developers full-time

---

## Chapter Index

| Chapter | Title | Duration | Status |
|---------|-------|----------|--------|
| [Chapter 0](#chapter-0) | Project Setup & Infrastructure | 1 week | ✅ Complete |
| [Chapter 1](#chapter-1) | Bare Metal Boot & Early Init | 2-3 weeks | ✅ Complete |
| [Chapter 2](#chapter-2) | Memory Management & MMU | 3-4 weeks | ✅ Complete |
| [Chapter 3](#chapter-3) | Exception Handling & Syscalls | 2-3 weeks | ✅ Complete |
| [Chapter 4](#chapter-4) | Kernel Object Model | 4-5 weeks | ✅ Complete |
| [Chapter 5](#chapter-5) | IPC & Message Passing | 3-4 weeks | ✅ Complete* |
| [Chapter 6](#chapter-6) | Scheduling & Context Switching | 3-4 weeks | ✅ Complete |
| [Chapter 7](#chapter-7) | Root Task & Boot Protocol | 2-3 weeks | ✅ Complete |
| [Chapter 8](#chapter-8) | Verification & Hardening | 4-6 weeks | 📋 Planned |
| [Chapter 9](#chapter-9) | Framework Integration & Runtime Services | 6-8 weeks | 📋 Planned |

*\*Chapter 5: Implementation complete (~1,630 LOC), full IPC operation tests deferred to Chapter 9*

**Note**:

- **Chapters 0-8**: Core microkernel (kernel-space)
- **Chapter 9**: KaaL Framework integration (user-space components)

**Status Legend**: 📋 Planned | 🚧 In Progress | ✅ Complete | ⏸️ Blocked

---

## Chapter 0: Project Setup & Infrastructure

**Duration**: 1 week
**Status**: ✅ Complete

### Objectives

1. Set up kernel workspace structure
2. Configure build system (pure Cargo)
3. Establish development workflow
4. Create initial documentation

### Deliverables

```
kaal/
├── kernel/
│   ├── Cargo.toml           # Kernel crate configuration
│   ├── kernel.ld            # Linker script
│   ├── rust-toolchain.toml  # Toolchain specification
│   └── src/
│       ├── lib.rs           # Kernel entry point
│       └── arch/
│           └── aarch64/
│               └── mod.rs   # ARM64 stub
├── runtime/kaal-sys/        # Syscall wrapper (skeleton)
└── docs/
    └── chapters/
        └── chapter-00-setup.md
```

### Implementation Steps

#### Step 1: Create Kernel Crate

```bash
# Create kernel directory
mkdir -p kernel/src/arch/aarch64

# Create Cargo.toml
cat > kernel/Cargo.toml << 'EOF'
[package]
name = "kaal-kernel"
version = "0.1.0"
edition = "2021"
authors = ["KaaL Contributors"]

[lib]
crate-type = ["staticlib"]

[dependencies]
bitflags = { version = "2.4", default-features = false }
spin = { version = "0.9", default-features = false }

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
overflow-checks = true

[features]
default = ["debug"]
debug = []
EOF

# Create rust-toolchain
cat > kernel/rust-toolchain.toml << 'EOF'
[toolchain]
channel = "nightly-2024-10-01"
components = ["rust-src", "rustfmt", "clippy"]
targets = ["aarch64-unknown-none"]
EOF
```

#### Step 2: Minimal Kernel Stub

```rust
// kernel/src/lib.rs
#![no_std]
#![no_main]
#![feature(naked_functions)]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// Kernel entry point (will be implemented in Chapter 1)
#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
```

#### Step 3: Build Script

```bash
#!/bin/bash
# tools/build-kernel.sh

set -e

echo "Building KaaL Rust Microkernel..."

cd kernel

cargo build \
    --release \
    --target aarch64-unknown-none \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem

echo "✓ Kernel built successfully"
ls -lh target/aarch64-unknown-none/release/libkaal_kernel.a
```

### Testing Criteria

- ✅ Kernel crate compiles without errors
- ✅ Build script succeeds
- ✅ `libkaal_kernel.a` is generated
- ✅ No dependencies on std library

### Documentation

Create `docs/chapters/chapter-00-setup.md` documenting:
- Project structure rationale
- Build system architecture
- Development workflow
- Tool requirements

---

## Chapter 1: Bare Metal Boot & Early Init

**Duration**: 2-3 weeks
**Status**: 📋 Planned
**Prerequisites**: Chapter 0

### Objectives

1. Boot on QEMU ARM64 virt platform
2. Initialize serial UART output
3. Print "Hello from KaaL Kernel!"
4. Parse device tree (DTB)
5. Detect memory regions

### Deliverables

```
kernel/src/
├── boot/
│   ├── mod.rs           # Boot module
│   ├── arm64.rs         # ARM64 boot protocol
│   └── dtb.rs           # Device tree parsing
├── arch/aarch64/
│   ├── mod.rs
│   ├── uart.rs          # PL011 UART driver
│   └── registers.rs     # System register access
└── debug/
    ├── mod.rs
    └── uart.rs          # Debug output via UART
```

### Implementation Steps

#### Step 1: Boot Assembly

```rust
// kernel/src/boot/arm64.rs
use core::arch::asm;

/// ARM64 boot entry point
///
/// Called by elfloader with:
/// - x0: DTB physical address
/// - x1: Root task p_start
/// - x2: Root task p_end
/// - x3: Root task v_entry
/// - x4: PV offset
#[naked]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    asm!(
        // Save boot parameters
        "mov x19, x0",
        "mov x20, x1",
        "mov x21, x2",
        "mov x22, x3",
        "mov x23, x4",

        // Set up stack
        "adrp x9, __stack_top",
        "add x9, x9, :lo12:__stack_top",
        "mov sp, x9",

        // Call Rust entry
        "mov x0, x19",
        "mov x1, x20",
        "mov x2, x21",
        "mov x3, x22",
        "mov x4, x23",
        "b rust_entry",
        options(noreturn)
    )
}

#[no_mangle]
extern "C" fn rust_entry(
    dtb_addr: usize,
    _root_p_start: usize,
    _root_p_end: usize,
    _root_v_entry: usize,
    _pv_offset: usize,
) -> ! {
    // Initialize UART
    crate::debug::uart::init();

    // First message!
    kprintln!("═══════════════════════════════════════");
    kprintln!("  KaaL Rust Microkernel v0.1.0");
    kprintln!("═══════════════════════════════════════");
    kprintln!("Boot parameters:");
    kprintln!("  DTB: {:#x}", dtb_addr);

    // Parse DTB
    match crate::boot::dtb::parse(dtb_addr) {
        Ok(info) => {
            kprintln!("Device tree parsed:");
            kprintln!("  Model: {}", info.model);
            kprintln!("  Memory: {:#x} - {:#x}",
                     info.memory_start, info.memory_end);
        }
        Err(e) => {
            kprintln!("ERROR: Failed to parse DTB: {:?}", e);
        }
    }

    kprintln!("\nChapter 1 Complete!");

    loop {}
}
```

#### Step 2: UART Driver

```rust
// kernel/src/arch/aarch64/uart.rs

/// PL011 UART registers
#[repr(C)]
struct Pl011Uart {
    dr: u32,        // Data register
    rsr: u32,       // Receive status
    _reserved1: [u32; 4],
    fr: u32,        // Flag register
    _reserved2: u32,
    ilpr: u32,
    ibrd: u32,      // Integer baud rate divisor
    fbrd: u32,      // Fractional baud rate divisor
    lcrh: u32,      // Line control
    cr: u32,        // Control
    // ... more registers
}

const UART_BASE: usize = 0x0900_0000; // QEMU virt

pub fn init() {
    let uart = UART_BASE as *mut Pl011Uart;
    unsafe {
        // Disable UART
        core::ptr::write_volatile(&mut (*uart).cr, 0);

        // Configure: 8n1, FIFO enabled
        core::ptr::write_volatile(&mut (*uart).lcrh, (1 << 4) | (3 << 5));

        // Enable UART
        core::ptr::write_volatile(&mut (*uart).cr, (1 << 0) | (1 << 8) | (1 << 9));
    }
}

pub fn putc(c: u8) {
    let uart = UART_BASE as *mut Pl011Uart;
    unsafe {
        // Wait until TX FIFO not full
        while (core::ptr::read_volatile(&(*uart).fr) & (1 << 5)) != 0 {}

        // Write byte
        core::ptr::write_volatile(&mut (*uart).dr, c as u32);
    }
}
```

#### Step 3: Linker Script

```ld
/* kernel/kernel.ld */
OUTPUT_FORMAT("elf64-littleaarch64")
OUTPUT_ARCH(aarch64)
ENTRY(_start)

SECTIONS
{
    /* Kernel loads at 1GB */
    . = 0x40000000;

    .text : {
        KEEP(*(.text._start))
        *(.text .text.*)
    }

    .rodata : ALIGN(4096) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4096) {
        *(.data .data.*)
    }

    .bss : ALIGN(4096) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* Stack (16KB) */
    .stack (NOLOAD) : ALIGN(4096) {
        . = . + 0x4000;
        __stack_top = .;
    }
}
```

### Testing Criteria

- ✅ Kernel boots in QEMU
- ✅ UART output appears: "KaaL Rust Microkernel v0.1.0"
- ✅ DTB parsed successfully
- ✅ Memory regions detected

### Test Command

```bash
# Link kernel
aarch64-linux-gnu-ld -T kernel/kernel.ld \
    --whole-archive kernel/target/aarch64-unknown-none/release/libkaal_kernel.a \
    -o build/kernel.elf

# Test in QEMU (with elfloader)
qemu-system-aarch64 \
    -machine virt,virtualization=on \
    -cpu cortex-a53 \
    -m 512M \
    -nographic \
    -kernel bootimage.elf
```

### Documentation

Create `docs/chapters/chapter-01-boot.md` covering:
- ARM64 boot protocol
- UART initialization
- DTB parsing
- Memory detection

---

## Chapter 2: Memory Management & MMU

**Duration**: 3-4 weeks
**Status**: 📋 Planned
**Prerequisites**: Chapter 1

### Objectives

1. Set up page tables (4KB pages)
2. Enable MMU with identity mapping
3. Implement physical memory allocator
4. Create virtual address space abstraction

### Deliverables

```
kernel/src/
├── memory/
│   ├── mod.rs
│   ├── allocator.rs     # Physical page allocator
│   ├── paging.rs        # Page table management
│   └── vspace.rs        # Virtual address spaces
└── arch/aarch64/
    └── mmu.rs           # ARM64 MMU control
```

### Key Concepts

- **4-level page tables**: L0 → L1 → L2 → L3
- **Physical allocator**: Bitmap-based page allocator
- **Virtual spaces**: Per-process address spaces

### Implementation Highlights

```rust
// kernel/src/memory/allocator.rs

pub struct PhysicalAllocator {
    start: PhysAddr,
    end: PhysAddr,
    bitmap: &'static mut [u64],
}

impl PhysicalAllocator {
    pub fn alloc_page(&mut self) -> Option<PhysAddr> {
        // Find free page in bitmap
        // Mark as allocated
        // Return physical address
    }

    pub fn free_page(&mut self, addr: PhysAddr) {
        // Mark page as free in bitmap
    }
}
```

### Testing Criteria

- ✅ MMU enabled successfully
- ✅ Can allocate/free physical pages
- ✅ Page table walks work correctly
- ✅ Identity mapping functional

### Documentation

Create `docs/chapters/chapter-02-memory.md`

---

## Chapter 3: Exception Handling & Syscalls

**Duration**: 2-3 weeks
**Status**: 📋 Planned
**Prerequisites**: Chapter 2

### Objectives

1. Set up exception vector table
2. Handle synchronous exceptions
3. Implement syscall interface
4. Handle page faults

### Deliverables

```
kernel/src/
├── arch/aarch64/
│   ├── exception.rs     # Exception vectors
│   └── context.rs       # Trap frame
└── syscall/
    ├── mod.rs
    └── dispatch.rs      # Syscall dispatcher
```

### Key Features

- **Exception vectors**: 16 entry points
- **Trap frame**: Saved register context
- **Syscall numbers**: seL4-compatible
- **Page fault handler**: Demand paging support

### Testing Criteria

- ✅ Exception vectors installed
- ✅ Can handle syscalls from user mode
- ✅ Page faults handled correctly
- ✅ Context saved/restored properly

### Documentation

Create `docs/chapters/chapter-03-exceptions.md`

---

## Chapter 4: Kernel Object Model

**Duration**: 4-5 weeks
**Status**: 📋 Planned
**Prerequisites**: Chapter 3

### Objectives

1. Implement capability representation
2. Create all kernel objects (TCB, CNode, Endpoint, etc.)
3. Implement object invocations
4. Build capability derivation

### Deliverables

```
kernel/src/objects/
├── mod.rs
├── capability.rs     # Capability type
├── cnode.rs          # Capability nodes
├── tcb.rs            # Thread control blocks
├── endpoint.rs       # IPC endpoints
├── notification.rs   # Async notifications
├── vspace.rs         # Virtual spaces
├── page.rs           # Physical pages
└── untyped.rs        # Untyped memory
```

### Core Types

```rust
#[repr(C)]
pub struct Capability {
    cap_type: CapType,
    object: usize,
    rights: Rights,
    guard: u64,
}

pub enum CapType {
    Null,
    UntypedMemory,
    Endpoint,
    Notification,
    Tcb,
    CNode,
    VSpace,
    Page,
}
```

### Testing Criteria

- ✅ Can create all object types
- ✅ Capability derivation works
- ✅ Object invocations succeed
- ✅ Rights enforcement correct

### Documentation

Create `docs/chapters/chapter-04-objects.md`

---

## Chapter 5: IPC & Message Passing

**Duration**: 3-4 weeks
**Status**: 📋 Planned
**Prerequisites**: Chapter 4

### Objectives

1. Implement basic IPC (send/receive)
2. Add call/reply semantics
3. Implement capability transfer
4. Build IPC fastpath

### Deliverables

```
kernel/src/ipc/
├── mod.rs
├── message.rs       # Message structure
├── transfer.rs      # Data transfer
├── endpoint.rs      # Endpoint operations
└── fastpath.rs      # Optimized fastpath
```

### Key Features

- **Synchronous IPC**: Rendezvous
- **Message registers**: 64 registers
- **Cap transfer**: Move/grant/mint
- **Fastpath**: Optimized common case

### Testing Criteria

- ✅ Can send/receive messages
- ✅ Call/reply works
- ✅ Capability transfer successful
- ✅ Fastpath functional

### Documentation

Create `docs/chapters/chapter-05-ipc.md`

---

## Chapter 6: Scheduling & Context Switching

**Duration**: 3-4 weeks
**Status**: 📋 Planned
**Prerequisites**: Chapter 5

### Objectives

1. Implement round-robin scheduler
2. Add priority scheduling
3. Build context switcher
4. Support multiple threads

### Deliverables

```
kernel/src/scheduler/
├── mod.rs
├── round_robin.rs
├── priority.rs
└── domain.rs        # Security domains
```

### Testing Criteria

- ✅ Multiple threads run
- ✅ Context switching works
- ✅ Priority respected
- ✅ Preemption functional

### Documentation

Create `docs/chapters/chapter-06-scheduler.md`

---

## Chapter 7: Root Task & Boot Protocol

**Duration**: 2-3 weeks
**Status**: 📋 Planned
**Prerequisites**: Chapters 1-6

### Objectives

1. Implement ELF loader for root task
2. Create boot info structure (memory regions, DTB, initial capabilities)
3. Load and start root task (first user-space component)
4. Establish initial capability delegation protocol
5. Basic root task that prints "Hello from user-space!"

### Deliverables

```text
kernel/src/boot/
├── elf_loader.rs      # ELF64 parser and loader
├── bootinfo.rs        # Boot info structure
└── root_task.rs       # Root task initialization

runtime/root-task/     # Minimal root task (user-space)
├── Cargo.toml
└── src/
    └── main.rs        # Hello world from user-space
```

### Testing Criteria

- ✅ ELF loader parses and loads valid ELF64 binaries
- ✅ Root task receives correct boot info
- ✅ Root task can print to UART via syscall
- ✅ Initial capability space correctly set up

### Documentation

Create `docs/chapters/CHAPTER_07_STATUS.md`

**Note**: This chapter focuses on **microkernel-side boot protocol only**. The root task is a minimal stub. Full Runtime Services (Capability Broker, Memory Manager) are part of the **KaaL Framework**, developed separately after microkernel completion.

---

## Chapter 8: Verification & Hardening

**Duration**: 6-8 weeks
**Status**: 📋 Planned
**Prerequisites**: Chapter 7

### Objectives

1. Add Verus proofs for core invariants
2. Prove memory safety
3. Verify IPC correctness
4. Stress testing

### Deliverables

```
kernel/src/verification/
├── mod.rs
├── proofs.rs        # Verus proofs
└── invariants.rs    # Kernel invariants
```

### Testing Criteria

- ✅ Core invariants proven
- ✅ Memory safety verified
- ✅ IPC correctness proven
- ✅ Stress tests pass

### Documentation

Create `docs/chapters/CHAPTER_08_STATUS.md`

---

## Chapter 9: Framework Integration & Runtime Services

**Duration**: 6-8 weeks
**Status**: 📋 Planned
**Prerequisites**: Chapters 0-8 complete

### Chapter Overview

Chapter 9 bridges the microkernel with user-space components, implementing the **KaaL Framework** - the ecosystem of services, drivers, and applications that run on top of the microkernel. This is where we test full IPC, build the capability broker, and create the runtime environment.

**Architecture Layers** (from ARCHITECTURE.md):

- **Layer 1: Runtime Services** (~8K LOC)
  - Capability Broker (5K LOC)
  - Memory Manager (3K LOC)
- **Layer 2: Driver & Device Layer** (~5-50K per driver)
  - DDDK (Device Driver Development Kit)
  - DDE-Linux compatibility layer
- **Layer 3: System Services** (~75K LOC)
  - VFS, Network Stack, Display Manager, Audio
- **Layer 4: Compatibility Shims** (~20K LOC)
  - LibC implementation, POSIX server
- **Layer 5: Applications**
  - POSIX programs, native Rust/C apps

### Objectives

#### Phase 1: Runtime Services Foundation (2 weeks)

1. Implement Capability Broker service
   - Hide seL4/KaaL capability complexity
   - Device resource allocation
   - Untyped memory management
   - IPC endpoint creation
2. Implement Memory Manager service
   - Physical memory allocation
   - Virtual address space management
   - Page table management

#### Phase 2: IPC Integration Testing (1 week)

1. **Full IPC end-to-end tests** (deferred from Chapter 5)
   - Multi-component send/receive
   - Capability transfer (grant/mint/derive)
   - Call/reply RPC semantics
   - FIFO ordering verification
2. IPC performance benchmarking
   - Measure IPC latency
   - Compare with seL4 baseline
   - Optimize fastpath

#### Phase 3: DDDK & Basic Drivers (2 weeks)

1. Device Driver Development Kit
   - Driver trait abstractions
   - Interrupt handling framework
   - DMA buffer management
2. Implement example drivers
   - UART driver (user-space)
   - Timer driver
   - GPIO driver

#### Phase 4: System Services (2-3 weeks)

1. Basic VFS implementation
   - File abstraction layer
   - Mount point management
   - Simple RAM filesystem
2. Network stack foundation
   - Socket abstractions
   - Protocol handlers
   - Buffer management

### Deliverables

```text
components/
├── runtime/
│   ├── capability-broker/     # ~5K LOC
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── device_manager.rs
│   │       ├── memory_manager.rs
│   │       └── endpoint_manager.rs
│   └── memory-manager/         # ~3K LOC
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── physical.rs
│           └── virtual.rs
├── drivers/
│   ├── dddk/                   # Driver Development Kit
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs
│   │       ├── interrupt.rs
│   │       └── dma.rs
│   └── uart/                   # Example: UART driver
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
└── services/
    └── vfs/                    # Basic VFS
        ├── Cargo.toml
        └── src/
            ├── main.rs
            └── ramfs.rs

tests/
└── integration/
    ├── ipc_full_test.rs        # Complete IPC testing
    ├── capability_transfer.rs  # Cap transfer tests
    └── benchmark.rs            # Performance tests
```

### Testing Criteria

#### IPC Testing (Chapter 5 deferred tests)

- ✅ Send/receive with blocking works between components
- ✅ Message data transfers correctly
- ✅ Capability transfer (grant/mint/derive) works
- ✅ Call/reply RPC semantics work
- ✅ FIFO ordering maintained
- ✅ IPC latency < 1000 cycles (target)

#### Runtime Services

- ✅ Capability Broker can allocate resources
- ✅ Memory Manager provides memory to components
- ✅ Components can communicate via IPC

#### Drivers & Services

- ✅ DDDK simplifies driver development
- ✅ Example drivers work
- ✅ Basic VFS functional

### Documentation

- Create `docs/chapters/CHAPTER_09_STATUS.md`
- Create `docs/FRAMEWORK_ARCHITECTURE.md`
- Create `components/README.md` (guide to Framework structure)
- Update `docs/ARCHITECTURE.md` with implementation details

### Key Achievements

By completing Chapter 9, we achieve:

1. **Full IPC Validation** - All deferred Chapter 5 tests pass with real components
2. **Framework Foundation** - Runtime services enable higher-level components
3. **Driver Framework** - DDDK reduces driver complexity
4. **Microkernel + Framework Integration** - Proven end-to-end system

**Note**: Chapter 9 marks the transition from **microkernel development** to **ecosystem building**. Chapters 0-8 built the kernel; Chapter 9 builds the world on top of it.

---

## Chapter Documentation Template

Each chapter document should follow this structure:

```markdown
# Chapter N: [Title]

## Overview
Brief description of chapter goals

## Prerequisites
- What you need before starting

## Concepts
Key concepts introduced

## Implementation
Step-by-step guide

## Testing
How to verify completion

## Troubleshooting
Common issues and solutions

## Next Steps
What comes next

## References
- Related documentation
- External resources
```

---

## Progress Tracking

Each chapter should be tracked with:

```yaml
chapter: N
title: "Chapter Title"
status: planned | in_progress | blocked | complete
started: YYYY-MM-DD
completed: YYYY-MM-DD
assignee: Developer Name
blockers: []
notes: ""
```

---

## Success Metrics

### Chapter Completion Criteria

A chapter is **COMPLETE** when:
1. ✅ All deliverables implemented
2. ✅ All tests passing
3. ✅ Documentation written
4. ✅ Code reviewed
5. ✅ Committed to main branch

### Overall Project Success

The microkernel project is **SUCCESSFUL** when:
1. ✅ All 8 chapters complete
2. ✅ Root task boots and runs
3. ✅ Performance matches C seL4
4. ✅ Core invariants verified
5. ✅ Production-ready

---

## References

- [RUST_MICROKERNEL_DESIGN.md](RUST_MICROKERNEL_DESIGN.md) - Detailed architecture
- [MICROKERNEL_COMPARISON.md](MICROKERNEL_COMPARISON.md) - Comparison with C seL4
- [Atmosphere Microkernel](https://mars-research.github.io/projects/atmo/) - Reference implementation
- [seL4 Manual](https://sel4.systems/Info/Docs/seL4-manual-latest.pdf) - API reference

---

**Last Updated**: 2025-10-12
**Version**: 1.0
