# KaaL Rust Microkernel Design

## Overview

This document outlines the architecture for integrating a pure-Rust seL4-compatible microkernel into the KaaL operating system, replacing the C-based seL4 kernel while maintaining capability-based security and formal verification potential.

## Repository Structure

```
kaal/
├── kernel/                          # NEW: Pure Rust microkernel
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                   # Kernel entry point
│   │   ├── boot/                    # Boot and initialization
│   │   │   ├── mod.rs
│   │   │   ├── arm64.rs             # ARM64-specific boot code
│   │   │   ├── dtb.rs               # Device tree parsing
│   │   │   └── early_init.rs        # Early kernel initialization
│   │   ├── arch/                    # Architecture-specific code
│   │   │   ├── mod.rs
│   │   │   └── aarch64/
│   │   │       ├── mod.rs
│   │   │       ├── exception.rs     # Exception vectors (inline asm)
│   │   │       ├── mmu.rs           # MMU/page tables
│   │   │       ├── gic.rs           # ARM GIC (interrupt controller)
│   │   │       ├── context.rs       # Context switching
│   │   │       └── registers.rs     # System register access
│   │   ├── objects/                 # Kernel object model
│   │   │   ├── mod.rs
│   │   │   ├── capability.rs        # Capability representation
│   │   │   ├── cnode.rs             # Capability nodes (CSpace)
│   │   │   ├── tcb.rs               # Thread control blocks
│   │   │   ├── endpoint.rs          # IPC endpoints
│   │   │   ├── notification.rs      # Async notifications
│   │   │   ├── vspace.rs            # Virtual address spaces
│   │   │   ├── page.rs              # Physical/virtual pages
│   │   │   └── untyped.rs           # Untyped memory
│   │   ├── ipc/                     # Inter-process communication
│   │   │   ├── mod.rs
│   │   │   ├── fastpath.rs          # Optimized IPC fastpath
│   │   │   ├── message.rs           # Message passing
│   │   │   └── transfer.rs          # Capability transfer
│   │   ├── scheduler/               # Thread scheduling
│   │   │   ├── mod.rs
│   │   │   ├── round_robin.rs       # Round-robin scheduler
│   │   │   ├── priority.rs          # Priority scheduling
│   │   │   └── domain.rs            # Domain scheduling (security)
│   │   ├── memory/                  # Memory management
│   │   │   ├── mod.rs
│   │   │   ├── allocator.rs         # Physical memory allocator
│   │   │   ├── vspace.rs            # Virtual space management
│   │   │   └── caching.rs           # Cache management
│   │   ├── syscall/                 # System call interface
│   │   │   ├── mod.rs
│   │   │   ├── dispatch.rs          # Syscall dispatcher
│   │   │   ├── invocation.rs        # Object invocations
│   │   │   └── api.rs               # seL4 API compatibility
│   │   ├── debug/                   # Debugging support
│   │   │   ├── mod.rs
│   │   │   ├── uart.rs              # Serial console
│   │   │   └── panic.rs             # Panic handler
│   │   └── verification/            # Formal verification (optional)
│   │       ├── mod.rs
│   │       └── proofs.rs            # Verus proofs
│   └── kernel.ld                    # Kernel linker script
│
├── runtime/                         # Existing KaaL runtime
│   ├── elfloader/                   # ✓ Already exists (Rust!)
│   ├── cap_broker/                  # Enhanced to use new kernel
│   ├── ipc/                         # Native Rust IPC (no FFI!)
│   ├── root-task/                   # Root task runtime
│   └── kaal-sys/                    # NEW: Direct kernel syscalls
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── syscalls.rs          # Type-safe syscall wrappers
│           ├── caps.rs              # Capability types
│           └── objects.rs           # Object abstractions
│
├── components/                      # Userspace components
│   └── ... (unchanged)
│
├── examples/
│   └── bootable-demo/              # Updated to use new kernel
│
└── tools/
    ├── build-kernel.sh             # NEW: Build Rust kernel
    └── build-bootimage.sh          # Updated: Use Rust kernel
```

## Core Kernel Architecture

### 1. Boot Sequence

```rust
// kernel/src/lib.rs
#![no_std]
#![no_main]
#![feature(asm_const)]
#![feature(naked_functions)]

use crate::boot::KernelBootInfo;
use crate::arch::aarch64;

/// Kernel entry point, called by elfloader
///
/// Parameters (from ARM64 boot protocol):
/// - x0: DTB physical address
/// - x1: Root task physical region start
/// - x2: Root task physical region end
/// - x3: Root task virtual entry point
/// - x4: Physical-virtual offset
#[no_mangle]
#[naked]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::asm!(
        // Save boot parameters
        "mov x19, x0",  // DTB
        "mov x20, x1",  // root task p_start
        "mov x21, x2",  // root task p_end
        "mov x22, x3",  // root task v_entry
        "mov x23, x4",  // pv_offset

        // Set up temporary stack
        "adr x9, __stack_top",
        "mov sp, x9",

        // Jump to Rust entry
        "b kernel_main",
        options(noreturn)
    )
}

/// Main kernel initialization (Rust!)
#[no_mangle]
extern "C" fn kernel_main(
    dtb_addr: usize,
    root_p_start: usize,
    root_p_end: usize,
    root_v_entry: usize,
    pv_offset: usize,
) -> ! {
    // 1. Initialize debug UART (for early logging)
    debug::uart::init();
    kprintln!("KaaL Kernel v0.1.0 - Pure Rust Microkernel");
    kprintln!("Boot parameters:");
    kprintln!("  DTB: {:#x}", dtb_addr);
    kprintln!("  Root task: {:#x} - {:#x}", root_p_start, root_p_end);

    // 2. Parse device tree
    let boot_info = boot::parse_dtb(dtb_addr)
        .expect("Failed to parse device tree");

    // 3. Initialize memory management
    memory::init(boot_info.memory_regions);

    // 4. Initialize interrupt controller (GIC)
    arch::aarch64::gic::init();

    // 5. Create initial kernel objects
    let root_cnode = objects::create_root_cnode();
    let root_vspace = objects::create_root_vspace();
    let root_tcb = objects::create_root_tcb(
        root_v_entry,
        root_vspace,
        root_cnode,
    );

    // 6. Map root task into virtual memory
    boot::map_root_task(root_p_start, root_p_end, root_vspace);

    // 7. Initialize scheduler
    scheduler::init(root_tcb);

    // 8. Enable interrupts
    arch::aarch64::enable_interrupts();

    kprintln!("Kernel initialization complete!");
    kprintln!("Starting root task at {:#x}", root_v_entry);

    // 9. Jump to scheduler (never returns)
    scheduler::start() // -> jumps to root task
}
```

### 2. Capability Model

```rust
// kernel/src/objects/capability.rs

/// seL4-compatible capability representation
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Capability {
    /// Capability type and rights
    cap_type: CapType,
    /// Object pointer (physical address)
    object: usize,
    /// Rights mask (Read/Write/Grant)
    rights: Rights,
    /// Guard and guard size (for CNodes)
    guard: u64,
}

#[repr(u8)]
pub enum CapType {
    Null = 0,
    UntypedMemory = 1,
    Endpoint = 2,
    Notification = 3,
    ThreadControlBlock = 4,
    CNode = 5,
    VSpace = 6,
    PageTable = 7,
    Page = 8,
    IRQHandler = 9,
    IRQControl = 10,
}

bitflags! {
    pub struct Rights: u8 {
        const READ = 0b001;
        const WRITE = 0b010;
        const GRANT = 0b100;
        const RW = Self::READ.bits | Self::WRITE.bits;
        const ALL = Self::READ.bits | Self::WRITE.bits | Self::GRANT.bits;
    }
}

impl Capability {
    /// Derive a new capability from this one (with reduced rights)
    pub fn derive(&self, new_rights: Rights) -> Option<Self> {
        if !self.rights.contains(Rights::GRANT) {
            return None;
        }

        Some(Capability {
            cap_type: self.cap_type,
            object: self.object,
            rights: self.rights & new_rights,
            guard: self.guard,
        })
    }
}
```

### 3. IPC Fastpath

```rust
// kernel/src/ipc/fastpath.rs

/// Optimized IPC fastpath (most common case)
///
/// Conditions for fastpath:
/// - Sender and receiver both runnable
/// - Same priority
/// - Simple message (no caps)
/// - No fault handling needed
pub fn fastpath_call(
    ep_cap: Capability,
    msg_info: MessageInfo,
) -> Result<MessageInfo, SyscallError> {
    // 1. Validate endpoint capability
    let ep = ep_cap.as_endpoint()?;

    // 2. Check if receiver is waiting
    let receiver = match ep.waiting_thread() {
        Some(tcb) => tcb,
        None => return slowpath_call(ep_cap, msg_info), // Fallback
    };

    // 3. Check fastpath conditions
    if !can_use_fastpath(&receiver) {
        return slowpath_call(ep_cap, msg_info);
    }

    // 4. Transfer message registers directly (no copy!)
    // This is FAST because we just swap register contexts
    let current = scheduler::current_tcb();
    transfer_registers(current, receiver, msg_info.length);

    // 5. Switch context (no scheduling!)
    arch::aarch64::context::switch(current, receiver);

    // 6. Receiver runs, returns reply
    Ok(receiver.reply_message())
}

/// Transfer message registers between TCBs
#[inline(always)]
fn transfer_registers(from: &Tcb, to: &Tcb, count: usize) {
    let count = count.min(64); // Max message registers

    unsafe {
        // Direct memory copy of registers (zero-copy!)
        core::ptr::copy_nonoverlapping(
            from.registers.msg_regs.as_ptr(),
            to.registers.msg_regs.as_mut_ptr(),
            count,
        );
    }
}
```

### 4. ARM64 Exception Handling

```rust
// kernel/src/arch/aarch64/exception.rs

/// Exception vector table
#[repr(C, align(2048))]
pub struct ExceptionVectors {
    // Current EL with SP0
    sync_sp0: [u8; 128],
    irq_sp0: [u8; 128],
    fiq_sp0: [u8; 128],
    serror_sp0: [u8; 128],

    // Current EL with SPx
    sync_spx: [u8; 128],
    irq_spx: [u8; 128],
    fiq_spx: [u8; 128],
    serror_spx: [u8; 128],

    // Lower EL using AArch64
    sync_lower_64: [u8; 128],
    irq_lower_64: [u8; 128],
    fiq_lower_64: [u8; 128],
    serror_lower_64: [u8; 128],

    // Lower EL using AArch32
    sync_lower_32: [u8; 128],
    irq_lower_32: [u8; 128],
    fiq_lower_32: [u8; 128],
    serror_lower_32: [u8; 128],
}

#[link_section = ".text.vectors"]
#[no_mangle]
pub static EXCEPTION_VECTORS: ExceptionVectors = unsafe {
    core::mem::transmute([
        // Sync exception from lower EL (syscalls!)
        *b"\
        stp x0, x1, [sp, #-16]!
        stp x2, x3, [sp, #-16]!
        stp x4, x5, [sp, #-16]!
        stp x6, x7, [sp, #-16]!
        stp x8, x9, [sp, #-16]!
        stp x10, x11, [sp, #-16]!
        stp x12, x13, [sp, #-16]!
        stp x14, x15, [sp, #-16]!
        stp x16, x17, [sp, #-16]!
        stp x18, x19, [sp, #-16]!
        stp x20, x21, [sp, #-16]!
        stp x22, x23, [sp, #-16]!
        stp x24, x25, [sp, #-16]!
        stp x26, x27, [sp, #-16]!
        stp x28, x29, [sp, #-16]!
        stp x30, xzr, [sp, #-16]!
        b sync_exception_handler
        ",
        // ... more exception handlers
    ])
};

/// Sync exception handler (syscalls and faults)
#[no_mangle]
extern "C" fn sync_exception_handler(context: &mut TrapFrame) {
    let esr_el1: u64;
    unsafe {
        core::arch::asm!("mrs {}, esr_el1", out(reg) esr_el1);
    }

    let exception_class = (esr_el1 >> 26) & 0x3F;

    match exception_class {
        0x15 => {
            // SVC (system call)
            handle_syscall(context);
        },
        0x24 | 0x25 => {
            // Data abort / Page fault
            handle_page_fault(context, esr_el1);
        },
        _ => {
            kprintln!("Unexpected exception: EC={:#x}", exception_class);
            panic!("Unhandled exception");
        }
    }
}
```

### 5. Type-Safe Syscall Interface

```rust
// runtime/kaal-sys/src/syscalls.rs

/// Safe Rust wrapper for seL4-compatible syscalls
pub mod syscalls {
    use super::*;

    /// Send a message to an endpoint (blocking)
    pub fn send(ep: Endpoint, msg: Message) -> Result<(), Error> {
        let msg_info = msg.info();

        unsafe {
            syscall_send(
                ep.cptr(),
                msg_info.words(),
                msg.regs[0],
                msg.regs[1],
                msg.regs[2],
                msg.regs[3],
            )
        }
    }

    /// Call operation: send + receive (IPC)
    pub fn call(ep: Endpoint, msg: Message) -> Result<Message, Error> {
        let msg_info = msg.info();

        let reply = unsafe {
            syscall_call(
                ep.cptr(),
                msg_info.words(),
                msg.regs[0],
                msg.regs[1],
                msg.regs[2],
                msg.regs[3],
            )
        };

        Ok(Message::from_raw(reply))
    }

    /// Raw syscall implementations (inline assembly)
    #[inline(always)]
    unsafe fn syscall_send(
        dest: usize,
        msg_info: u64,
        mr0: u64,
        mr1: u64,
        mr2: u64,
        mr3: u64,
    ) {
        core::arch::asm!(
            "svc #0",
            in("x0") SYSCALL_SEND,
            in("x1") dest,
            in("x2") msg_info,
            in("x3") mr0,
            in("x4") mr1,
            in("x5") mr2,
            in("x6") mr3,
        );
    }
}
```

## Integration with Existing KaaL Components

### 1. Cap Broker Enhancement

```rust
// runtime/cap_broker/src/lib.rs

// Now using native Rust kernel syscalls (no FFI!)
use kaal_sys::syscalls;
use kaal_sys::caps::{Endpoint, CNode, Untyped};

pub struct CapabilityBroker {
    root_cnode: CNode,
    untyped_pool: Vec<Untyped>,
}

impl CapabilityBroker {
    /// Allocate a new endpoint (pure Rust!)
    pub fn alloc_endpoint(&mut self) -> Result<Endpoint, Error> {
        // Get untyped memory
        let untyped = self.untyped_pool.pop()
            .ok_or(Error::OutOfMemory)?;

        // Retype into endpoint (direct kernel call)
        let ep = syscalls::untyped_retype(
            untyped,
            ObjectType::Endpoint,
            0, // size
            self.root_cnode,
            0, // index
            1, // depth
        )?;

        Ok(Endpoint::from_cap(ep))
    }
}
```

### 2. Build System Integration

```toml
# Cargo.toml (workspace)
[workspace]
members = [
    "kernel",              # NEW: Rust microkernel
    "runtime/elfloader",
    "runtime/kaal-sys",    # NEW: Kernel syscall wrapper
    "runtime/cap_broker",
    "runtime/ipc",
    "runtime/dddk",
    "examples/bootable-demo",
]

[workspace.dependencies]
# Shared across kernel and userspace
spin = { version = "0.9", default-features = false }
bitflags = "2.4"
```

```toml
# kernel/Cargo.toml
[package]
name = "kaal-kernel"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib"]

[dependencies]
bitflags = { workspace = true }
spin = { workspace = true }

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"

[features]
default = ["debug"]
debug = []              # Enable debug printing
verification = []       # Enable Verus proofs
```

### 3. Build Script

```bash
#!/bin/bash
# tools/build-kernel.sh

set -e

echo "Building KaaL Rust Microkernel..."

# Build kernel
cargo build \
    --manifest-path kernel/Cargo.toml \
    --release \
    --target aarch64-unknown-none \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem

# Link kernel into ELF
aarch64-linux-gnu-ld \
    -T kernel/kernel.ld \
    --whole-archive target/aarch64-unknown-none/release/libkaal_kernel.a \
    --no-whole-archive \
    -o build/kernel.elf

echo "Kernel built: build/kernel.elf"
ls -lh build/kernel.elf
```

## Build Flow

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Build Kernel (Pure Rust, No CMake!)                     │
│    $ cargo build --target aarch64-unknown-none             │
│    Output: target/.../libkaal_kernel.a                     │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. Link Kernel ELF                                          │
│    $ aarch64-linux-gnu-ld -T kernel.ld libkaal_kernel.a    │
│    Output: build/kernel.elf                                 │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. Build Elfloader (Rust, existing!)                        │
│    $ cargo build runtime/elfloader                          │
│    Output: libelfloader.a                                   │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. Build Root Task (Rust + kaal-sys)                        │
│    $ cargo build examples/bootable-demo                     │
│    Output: root-task.elf                                    │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. Assemble Bootimage (elfloader + kernel + root task)     │
│    $ ./tools/build-bootimage.sh                             │
│    Output: bootimage.elf                                    │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│ 6. Test in QEMU                                             │
│    $ qemu-system-aarch64 -kernel bootimage.elf              │
└─────────────────────────────────────────────────────────────┘

ALL RUST! NO CMAKE! 🎉
```

## Migration Strategy

### Phase 1: Minimal Kernel (Weeks 1-8)
- Boot on QEMU ARM64
- Basic MMU and exception handling
- Serial console output
- Single-threaded execution
- **Milestone**: Kernel boots and prints "Hello from Rust kernel!"

### Phase 2: Object Model (Weeks 9-16)
- Implement all object types
- Capability operations
- Basic IPC (no fastpath)
- **Milestone**: Can create TCB and send IPC message

### Phase 3: Scheduling & Multicore (Weeks 17-24)
- Round-robin scheduler
- Priority scheduling
- Context switching
- **Milestone**: Multiple threads running

### Phase 4: Performance & Fastpath (Weeks 25-32)
- IPC fastpath optimization
- Cache optimization
- Zero-copy IPC
- **Milestone**: Performance matches C seL4

### Phase 5: Full API Compatibility (Weeks 33-40)
- All seL4 syscalls
- Domain scheduling
- MCS (if needed)
- **Milestone**: Can run seL4 test suite

### Phase 6: Verification (Optional, Months 11-18)
- Verus proofs for core invariants
- Memory safety proofs
- IPC correctness
- **Milestone**: Core kernel verified

## Advantages Over C seL4

| Aspect | C seL4 + CMake | KaaL Rust Kernel |
|--------|----------------|------------------|
| **Build System** | CMake + Make + Python | Pure Cargo |
| **Memory Safety** | Manual (prone to bugs) | Automatic (compiler) |
| **Type Safety** | Weak (casts everywhere) | Strong (no casts) |
| **Verification** | 20 person-years | 2 person-years (Verus) |
| **Integration** | FFI boundary | Native Rust |
| **Development** | Complex toolchain | Standard Rust tools |
| **Debugging** | GDB + manual | Rust panic + GDB |
| **Maintainability** | High complexity | Lower complexity |
| **Performance** | Excellent | Excellent (zero-cost) |

## Performance Expectations

Based on Atmosphere microkernel results:

- **IPC Latency**: ~1000 cycles (comparable to seL4)
- **Context Switch**: ~500 cycles
- **Syscall Overhead**: ~200 cycles
- **Memory Footprint**: ~100KB (smaller than seL4)
- **Code Size**: ~10-15K LOC (vs 30K for seL4)

## Verification Strategy (Optional)

Using Verus for incremental verification:

```rust
// kernel/src/objects/capability.rs
use vstd::prelude::*;

verus! {
    /// Proof: Capability derivation preserves object identity
    proof fn derive_preserves_object(cap: Capability, rights: Rights)
        requires cap.rights.contains(Rights::GRANT)
        ensures cap.derive(rights).map(|c| c.object) == Some(cap.object)
    {
        // Verus proves this automatically!
    }

    /// Proof: Cannot derive with GRANT unless original has GRANT
    proof fn derive_requires_grant(cap: Capability, rights: Rights)
        requires !cap.rights.contains(Rights::GRANT)
        ensures cap.derive(rights).is_none()
    {
        // Automatic proof
    }
}
```

## Conclusion

A pure-Rust seL4-compatible microkernel integrated into KaaL is:

1. **Highly Feasible**: Proven by Atmosphere (2 person-years)
2. **Strategically Smart**: Eliminates CMake, enables full Rust ecosystem
3. **Performance Equivalent**: Zero-cost abstractions match C
4. **More Maintainable**: Type safety + borrow checker = fewer bugs
5. **Easier to Verify**: Verus reduces effort by 10x

**Estimated Timeline**: 9-12 months for production-ready kernel
**Estimated Effort**: 1-2 full-time developers
**Risk**: Low (proven by Atmosphere)
**Reward**: HIGH - Full Rust OS stack, no CMake, easier verification

This design positions KaaL as one of the first **pure-Rust capability-based operating systems** with a path to formal verification.
