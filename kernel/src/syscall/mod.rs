//! System call interface
//!
//! This module implements the syscall dispatcher for the KaaL microkernel.
//! Syscalls follow the seL4 convention with syscall number in x8 and
//! arguments in x0-x5.

pub mod numbers;

use crate::arch::aarch64::context::TrapFrame;
use crate::kprintln;

/// Syscall dispatcher - called from exception handler
///
/// Decodes the syscall number from the trap frame and dispatches to the
/// appropriate handler. Returns the result in x0.
pub fn handle_syscall(tf: &mut TrapFrame) {
    let syscall_num = tf.syscall_number();
    let args = tf.syscall_args();

    // Dispatch based on syscall number
    let result = match syscall_num {
        numbers::SYS_DEBUG_PUTCHAR => sys_debug_putchar(args[0]),
        numbers::SYS_DEBUG_PRINT => sys_debug_print(args[0], args[1]),
        numbers::SYS_YIELD => sys_yield(),

        // Chapter 9: Capability management syscalls
        numbers::SYS_CAP_ALLOCATE => sys_cap_allocate(),
        numbers::SYS_MEMORY_ALLOCATE => sys_memory_allocate(args[0]),
        numbers::SYS_DEVICE_REQUEST => sys_device_request(args[0]),
        numbers::SYS_ENDPOINT_CREATE => sys_endpoint_create(),
        numbers::SYS_PROCESS_CREATE => sys_process_create(args[0], args[1], args[2], args[3]),

        _ => {
            kprintln!("[syscall] Unknown syscall number: {}", syscall_num);
            u64::MAX // Error: invalid syscall
        }
    };

    // Set return value
    tf.set_return_value(result);
}

/// Debug syscall: print a single character
fn sys_debug_putchar(ch: u64) -> u64 {
    if ch <= 0x7F {
        crate::kprint!("{}", ch as u8 as char);
        0 // Success
    } else {
        u64::MAX // Error: invalid character
    }
}

/// Debug syscall: print a string
///
/// This is a simple implementation that reads from the user's address space.
/// In a production kernel, this would need proper memory validation and
/// page table walking to ensure the address is valid and mapped.
///
/// For Chapter 7, we assume the root task has identity-mapped memory,
/// so we can directly access the pointer.
fn sys_debug_print(ptr: u64, len: u64) -> u64 {
    // Validate length (prevent abuse)
    if len > 4096 {
        return u64::MAX; // Error: string too long
    }

    // Safety: We're assuming identity-mapped memory for now.
    // TODO Chapter 8: Add proper memory validation via page table walk
    unsafe {
        let slice = core::slice::from_raw_parts(ptr as *const u8, len as usize);

        // Validate UTF-8 (optional, but prevents panic)
        if let Ok(s) = core::str::from_utf8(slice) {
            crate::kprint!("{}", s);
            0 // Success
        } else {
            u64::MAX // Error: invalid UTF-8
        }
    }
}

/// Yield syscall: give up CPU time slice
fn sys_yield() -> u64 {
    kprintln!("[syscall] yield (no-op, scheduler not implemented)");
    0 // Success
}

//
// Chapter 9: Capability Management Syscalls
//

// Global capability slot counter (simplified for Chapter 9 Phase 1)
static mut NEXT_CAP_SLOT: u64 = 100;

/// Allocate a capability slot
///
/// Returns a capability slot number that can be used to store capabilities.
/// This is a simplified implementation; a full implementation would track
/// allocated slots and support deallocation.
fn sys_cap_allocate() -> u64 {
    unsafe {
        let slot = NEXT_CAP_SLOT;
        NEXT_CAP_SLOT += 1;
        kprintln!("[syscall] cap_allocate -> slot {}", slot);
        slot
    }
}

/// Allocate physical memory
///
/// Args: size (bytes)
/// Returns: physical address of allocated memory
///
/// This allocates physical memory frames using the kernel's frame allocator.
/// For simplicity in Chapter 9 Phase 1, we only allocate single frames.
/// Multi-frame allocation will be added in Phase 2.
fn sys_memory_allocate(size: u64) -> u64 {
    use crate::memory::{alloc_frame, PAGE_SIZE};

    let page_size = PAGE_SIZE as u64;
    let pages_needed = ((size + page_size - 1) / page_size);

    kprintln!("[syscall] memory_allocate: {} bytes ({} pages)", size, pages_needed);

    // For now, only support single-page allocations
    // TODO Chapter 9 Phase 2: Support multi-page allocations
    if pages_needed > 1 {
        kprintln!("[syscall] memory_allocate: multi-page allocation not yet supported");
        return u64::MAX; // Error: too large
    }

    // Allocate a physical frame
    match alloc_frame() {
        Some(pfn) => {
            let phys_addr = pfn.phys_addr().as_u64();
            kprintln!("[syscall] memory_allocate -> 0x{:x}", phys_addr);
            phys_addr
        }
        None => {
            kprintln!("[syscall] memory_allocate: out of memory");
            u64::MAX // Error: out of memory
        }
    }
}

/// Request device resources
///
/// Args: device_id (0 = UART0, 1 = Timer, etc.)
/// Returns: MMIO base address for the device
///
/// This is a simplified implementation that returns known MMIO addresses
/// for QEMU virt platform devices.
fn sys_device_request(device_id: u64) -> u64 {
    let mmio_base = match device_id {
        0 => 0x0900_0000, // UART0
        1 => 0x0901_0000, // UART1
        2 => 0x0A00_0000, // RTC
        _ => {
            kprintln!("[syscall] device_request: unknown device {}", device_id);
            return u64::MAX; // Error: unknown device
        }
    };

    kprintln!("[syscall] device_request(device={}) -> MMIO 0x{:x}", device_id, mmio_base);
    mmio_base
}

/// Create IPC endpoint
///
/// Returns: endpoint capability slot
///
/// This allocates a capability slot and associates it with a new IPC endpoint.
/// The actual endpoint data structure would be created in a full implementation.
fn sys_endpoint_create() -> u64 {
    // For now, just allocate a capability slot
    // In a full implementation, this would create an actual endpoint object
    let slot = sys_cap_allocate();
    kprintln!("[syscall] endpoint_create -> slot {}", slot);
    slot
}

/// Create a new process with full isolation
///
/// Args:
/// - entry_point: Initial program counter (ELR_EL1)
/// - stack_pointer: Initial stack pointer (SP_EL0)
/// - page_table_root: Physical address of page table (TTBR0)
/// - cspace_root: Physical address of CNode (capability space root)
///
/// Returns: Process ID (TID), or u64::MAX on error
///
/// This creates a fully isolated process with:
/// - Separate address space (VSpace via page_table_root)
/// - Separate capability space (CSpace via cspace_root)
/// - Dedicated stack
/// - Independent execution context
fn sys_process_create(
    entry_point: u64,
    stack_pointer: u64,
    page_table_root: u64,
    cspace_root: u64,
) -> u64 {
    use crate::memory::{alloc_frame, VirtAddr};
    use crate::objects::{TCB, CNode};
    use crate::scheduler;

    kprintln!("[syscall] process_create:");
    kprintln!("  entry: {:#x}", entry_point);
    kprintln!("  stack: {:#x}", stack_pointer);
    kprintln!("  page_table: {:#x}", page_table_root);
    kprintln!("  cspace: {:#x}", cspace_root);

    // Allocate frame for TCB
    let tcb_frame = match alloc_frame() {
        Some(pfn) => pfn.phys_addr(),
        None => {
            kprintln!("[syscall] process_create: out of memory (TCB)");
            return u64::MAX;
        }
    };

    kprintln!("  allocated TCB at: {:#x}", tcb_frame.as_usize());

    // Generate process ID (use frame address for now - unique per process)
    let pid = tcb_frame.as_usize();

    // Get CNode pointer
    let cspace_ptr = cspace_root as *mut CNode;

    // Allocate IPC buffer (for now, placeholder address)
    // TODO: Should allocate actual IPC buffer frame
    let ipc_buffer = VirtAddr::new(0x8000_0000);

    // Create TCB
    let tcb_ptr = tcb_frame.as_usize() as *mut TCB;
    unsafe {
        let tcb = TCB::new(
            pid,
            cspace_ptr,
            page_table_root as usize,
            ipc_buffer,
            entry_point,
            stack_pointer,
        );
        core::ptr::write(tcb_ptr, tcb);

        // Set state to Runnable
        (*tcb_ptr).set_state(crate::objects::ThreadState::Runnable);

        // Add to scheduler
        scheduler::enqueue(tcb_ptr);
    }

    kprintln!("[syscall] process_create -> PID {}", pid);
    pid as u64
}
