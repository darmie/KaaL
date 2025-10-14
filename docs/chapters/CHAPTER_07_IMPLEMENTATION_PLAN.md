# Chapter 7: Root Task & Boot Protocol - Implementation Plan

**Date**: 2025-10-14
**Status**: Ready to implement
**Prerequisites**: Chapters 1-6 complete ✅

## Executive Summary

Chapter 7 creates the bridge from kernel-space to user-space by:
1. Storing bootinfo from elfloader
2. Creating the root task thread
3. Transitioning to EL0 (user mode)
4. Proving user-space execution works

**Key Insight**: The elfloader already handles ELF loading! Chapter 7 is much simpler than initially planned.

---

## Current State Analysis

### ✅ What Already Exists

#### Elfloader (`runtime/elfloader/`)
- **Complete ELF parser** ([elf.rs](../../runtime/elfloader/src/elf.rs))
- **Boot protocol** ([boot.rs](../../runtime/elfloader/src/boot.rs))
- **Loads both**: kernel + root task ELFs
- **Passes 6 parameters** to kernel via x0-x5

#### Kernel (`kernel/`)
- **Receives boot params** ([main.rs](../../kernel/src/main.rs) `_start`)
- **Stores params** in x19-x23 (callee-saved registers)
- **Boot module** ([boot/mod.rs](../../kernel/src/boot/mod.rs))
- **Memory management** (Chapter 2)
- **Object model** (Chapter 4)
- **Scheduler** (Chapter 6)

### ❌ What's Missing (Chapter 7 Scope)

1. **Store bootinfo globally** in kernel
2. **Create root task thread** from stored bootinfo
3. **Initialize root task context** for EL0
4. **Minimal root task binary** that prints "Hello from EL0!"
5. **Test end-to-end** flow

---

## Boot Parameter Flow

### Elfloader → Kernel Handoff

```rust
// Elfloader calls kernel with 6 parameters:
type KernelEntry = extern "C" fn(
    usize,  // x0: user_img_start (root task physical start)
    usize,  // x1: user_img_end   (root task physical end)
    usize,  // x2: pv_offset      (physical-virtual offset)
    usize,  // x3: user_entry     (root task entry point)
    usize,  // x4: dtb_addr       (device tree address)
    usize,  // x5: dtb_size       (device tree size)
) -> !;
```

### Current Kernel Reception

```asm
// kernel/src/main.rs _start assembly
mov x19, x4      // x19 = dtb_addr (from x4)
mov x20, x0      // x20 = user_img_start (from x0)
mov x21, x1      // x21 = user_img_end (from x1)
mov x22, x3      // x22 = user_entry (from x3)
mov x23, x2      // x23 = pv_offset (from x2)
// Note: x5 (dtb_size) is not currently saved!
```

---

## Implementation Steps

### Step 1: Create BootInfo Structure (30 min)

Create `kernel/src/boot/bootinfo.rs`:

```rust
//! Boot information from elfloader

use crate::memory::{PhysAddr, VirtAddr};

/// Boot information passed from elfloader
#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    /// Physical start of root task ELF image
    pub root_task_start: PhysAddr,
    /// Physical end of root task ELF image
    pub root_task_end: PhysAddr,
    /// Virtual entry point of root task
    pub root_task_entry: VirtAddr,
    /// Device tree blob physical address
    pub dtb_addr: PhysAddr,
    /// Device tree blob size
    pub dtb_size: usize,
    /// Physical-to-virtual offset (0 for identity mapping)
    pub pv_offset: usize,
}

/// Global boot info storage
static mut BOOT_INFO: Option<BootInfo> = None;

/// Store boot info (called once during boot)
pub unsafe fn store_boot_info(info: BootInfo) {
    BOOT_INFO = Some(info);
}

/// Get stored boot info
pub fn get_boot_info() -> Option<&'static BootInfo> {
    unsafe { BOOT_INFO.as_ref() }
}
```

### Step 2: Update _start to Save All Parameters (15 min)

Update `kernel/src/main.rs`:

```asm
global_asm!(
    ".section .text._start",
    ".global _start",
    "_start:",
    "    // Enable FP/SIMD",
    "    mrs x10, cpacr_el1",
    "    orr x10, x10, #(0x3 << 20)",
    "    msr cpacr_el1, x10",
    "    isb",
    "    // Save boot parameters (all 6!)",
    "    mov x19, x4",      // x19 = dtb_addr (from x4)
    "    mov x20, x0",      // x20 = user_img_start (from x0)
    "    mov x21, x1",      // x21 = user_img_end (from x1)
    "    mov x22, x3",      // x22 = user_entry (from x3)
    "    mov x23, x2",      // x23 = pv_offset (from x2)
    "    mov x24, x5",      // x24 = dtb_size (from x5) NEW!
    "    b {kernel_entry}",
    kernel_entry = sym kaal_kernel::boot::kernel_entry,
);
```

### Step 3: Update get_boot_params (15 min)

Update `kernel/src/boot/mod.rs`:

```rust
/// Extract boot parameters from saved registers
#[inline(never)]
unsafe fn get_boot_params() -> BootParams {
    let dtb_addr: usize;
    let root_p_start: usize;
    let root_p_end: usize;
    let root_v_entry: usize;
    let pv_offset: usize;
    let dtb_size: usize;

    core::arch::asm!(
        "mov {}, x19",
        "mov {}, x20",
        "mov {}, x21",
        "mov {}, x22",
        "mov {}, x23",
        "mov {}, x24",  // NEW!
        out(reg) dtb_addr,
        out(reg) root_p_start,
        out(reg) root_p_end,
        out(reg) root_v_entry,
        out(reg) pv_offset,
        out(reg) dtb_size,
    );

    BootParams {
        dtb_addr,
        root_p_start,
        root_p_end,
        root_v_entry,
        pv_offset,
        dtb_size,  // NEW!
    }
}
```

### Step 4: Store BootInfo During Kernel Init (30 min)

Update `kernel/src/boot/mod.rs` `kernel_entry()`:

```rust
pub fn kernel_entry() -> ! {
    let params = unsafe { get_boot_params() };

    // Store boot info globally
    let boot_info = bootinfo::BootInfo {
        root_task_start: PhysAddr::new(params.root_p_start),
        root_task_end: PhysAddr::new(params.root_p_end),
        root_task_entry: VirtAddr::new(params.root_v_entry),
        dtb_addr: PhysAddr::new(params.dtb_addr),
        dtb_size: params.dtb_size,
        pv_offset: params.pv_offset,
    };

    unsafe {
        bootinfo::store_boot_info(boot_info);
    }

    // Continue with rest of initialization...
    crate::config::init_console();
    // ... rest of kernel_entry code
}
```

### Step 5: Create Root Task Binary (1 hour)

Create `runtime/root-task/`:

```toml
# runtime/root-task/Cargo.toml
[package]
name = "kaal-root-task"
version = "0.1.0"
edition = "2021"

[dependencies]
# Minimal - no component SDK yet (that's Chapter 9)

[profile.release]
panic = "abort"
lto = true
opt-level = "z"  # Optimize for size
```

```rust
// runtime/root-task/src/main.rs
#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Syscall to print (we'll implement this)
    sys_print("Hello from root task (EL0)!\n");

    loop {
        // Idle loop
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

/// Syscall: Print to UART
fn sys_print(msg: &str) {
    unsafe {
        core::arch::asm!(
            "svc #0",  // Syscall number 0 = print
            in("x0") msg.as_ptr(),
            in("x1") msg.len(),
        );
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
```

### Step 6: Implement Root Task Creation (2-3 hours)

Create `kernel/src/boot/root_task.rs`:

```rust
//! Root task initialization

use crate::objects::{TCB, CNode, ThreadState, Capability, CapType};
use crate::memory::{PhysAddr, VirtAddr};
use crate::boot::bootinfo::BootInfo;
use core::mem::MaybeUninit;

// Static allocation for root task (no heap!)
static mut ROOT_TASK_TCB: MaybeUninit<TCB> = MaybeUninit::uninit();
static mut ROOT_TASK_CSPACE: [Capability; 256] = [Capability::null(); 256];
static mut ROOT_TASK_CNODE: MaybeUninit<CNode> = MaybeUninit::uninit();
static mut ROOT_TASK_STACK: [u8; 64 * 1024] = [0; 64 * 1024]; // 64KB stack

pub unsafe fn create_and_start_root_task(boot_info: &BootInfo) -> Result<(), &'static str> {
    crate::kprintln!("═══════════════════════════════════════════════════════════");
    crate::kprintln!("  Chapter 7: Root Task & Boot Protocol");
    crate::kprintln!("═══════════════════════════════════════════════════════════");
    crate::kprintln!("");

    // 1. Create CSpace for root task
    let cspace_paddr = PhysAddr::new(&ROOT_TASK_CSPACE[0] as *const _ as usize);
    let cnode = CNode::new(8, cspace_paddr).map_err(|_| "Failed to create root task CNode")?;
    ROOT_TASK_CNODE.write(cnode);

    crate::kprintln!("✓ Created root task CSpace");

    // 2. Calculate stack top
    let stack_top = ROOT_TASK_STACK.as_ptr() as usize + ROOT_TASK_STACK.len();

    // 3. Create TCB
    let tcb = TCB::new(
        1,  // TID 1 (root task)
        ROOT_TASK_CNODE.as_mut_ptr(),
        0,  // VSpace (identity mapped for now)
        VirtAddr::new(0),  // IPC buffer (not used yet)
        boot_info.root_task_entry.as_usize() as u64,
        stack_top as u64,
    );

    ROOT_TASK_TCB.write(tcb);
    let tcb_ptr = ROOT_TASK_TCB.as_mut_ptr();

    crate::kprintln!("✓ Created root task TCB (TID 1)");
    crate::kprintln!("  Entry: {:#x}", boot_info.root_task_entry.as_usize());
    crate::kprintln!("  Stack: {:#x}", stack_top);

    // 4. Initialize thread context for EL0
    crate::arch::aarch64::context_switch::init_thread_context(
        tcb_ptr,
        boot_info.root_task_entry.as_usize(),
        stack_top,
        0,  // No argument
    );

    // Set to EL0 mode (user mode)
    (*tcb_ptr).context_mut().spsr_el1 = 0x0;  // EL0t, interrupts enabled

    crate::kprintln!("✓ Initialized context for EL0 (user mode)");

    // 5. Set runnable and add to scheduler
    (*tcb_ptr).set_state(ThreadState::Runnable);
    crate::scheduler::add_thread(tcb_ptr);

    crate::kprintln!("✓ Added root task to scheduler");
    crate::kprintln!("");
    crate::kprintln!("Starting root task...");
    crate::kprintln!("═══════════════════════════════════════════════════════════");

    Ok(())
}
```

### Step 7: Call Root Task Creation from kernel_entry (15 min)

Update `kernel/src/boot/mod.rs`:

```rust
pub fn kernel_entry() -> ! {
    // ... existing initialization (console, DTB, memory, objects, scheduler)

    // Chapter 7: Create root task
    if let Some(boot_info) = bootinfo::get_boot_info() {
        unsafe {
            if let Err(e) = root_task::create_and_start_root_task(boot_info) {
                crate::kprintln!("ERROR: Failed to create root task: {}", e);
            }
        }
    }

    // Start scheduler (will run root task)
    crate::scheduler::start();
}
```

### Step 8: Implement sys_print Syscall (1 hour)

Update `kernel/src/syscall/mod.rs`:

```rust
/// Syscall handler
pub fn handle_syscall(syscall_num: u64, args: &[u64]) -> Result<u64, SyscallError> {
    match syscall_num {
        0 => sys_print(args),  // NEW!
        // ... other syscalls
        _ => Err(SyscallError::InvalidSyscall),
    }
}

/// Syscall 0: Print to UART (for root task debugging)
fn sys_print(args: &[u64]) -> Result<u64, SyscallError> {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgs);
    }

    let ptr = args[0] as *const u8;
    let len = args[1] as usize;

    // Validate pointer (basic check)
    if ptr.is_null() || len == 0 {
        return Err(SyscallError::InvalidArgs);
    }

    // Read string from user space
    let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };

    // Print via kernel UART
    if let Ok(s) = core::str::from_utf8(bytes) {
        crate::kprint!("{}", s);
        Ok(0)
    } else {
        Err(SyscallError::InvalidArgs)
    }
}
```

---

## Testing Strategy

### Test 1: Build Kernel
```bash
cd kernel
cargo build --release
```

Expected: Compiles successfully

### Test 2: Build Root Task
```bash
cd runtime/root-task
cargo build --release --target aarch64-unknown-none-elf
```

Expected: Compiles successfully

### Test 3: Elfloader Integration
Update elfloader to embed new root task, rebuild, test in QEMU.

Expected output:
```
KaaL Elfloader v0.1.0
...
Jumping to KaaL kernel...

KaaL Rust Microkernel v0.1.0
...
Chapter 7: Root Task & Boot Protocol
✓ Created root task CSpace
✓ Created root task TCB (TID 1)
✓ Initialized context for EL0 (user mode)
✓ Added root task to scheduler
Starting root task...

Hello from root task (EL0)!
```

---

## Architecture Notes

### Cyclic Dependency Prevention

**Root task does NOT use component SDK** - that's Chapter 9!

```
User Components → Component SDK → IPC Proto
                                      ↑
Root Task  ───────────────────────────┘
    ↑
Kernel (syscalls)
```

### UART Architecture

- **Kernel**: Minimal UART (debug only)
- **Root Task**: Uses syscall to print
- **Chapter 9**: User-space UART driver component

---

## Success Criteria

- ✅ Kernel receives all 6 boot parameters
- ✅ BootInfo stored globally
- ✅ Root task TCB created
- ✅ Context initialized for EL0
- ✅ Root task added to scheduler
- ✅ Root task executes in EL0 (user mode)
- ✅ Root task can print via syscall
- ✅ No kernel panics

---

## Estimated Timeline

| Task | Estimated Time |
|------|----------------|
| BootInfo structure | 30 min |
| Update _start assembly | 15 min |
| Update get_boot_params | 15 min |
| Store BootInfo globally | 30 min |
| Create root task binary | 1 hour |
| Implement root task creation | 2-3 hours |
| Implement sys_print syscall | 1 hour |
| Testing & debugging | 2-3 hours |
| **Total** | **8-10 hours** (1-2 days) |

---

## Next Steps After Chapter 7

**Chapter 8**: Verification & Hardening
**Chapter 9**: Framework Integration (Component SDK, Capability Broker, Full IPC Testing)

---

**Document Version**: 1.0
**Last Updated**: 2025-10-14
**Ready to Implement**: ✅ Yes!
