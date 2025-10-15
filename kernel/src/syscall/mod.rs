//! System call interface
//!
//! This module implements the syscall dispatcher for the KaaL microkernel.
//! Syscalls follow the seL4 convention with syscall number in x8 and
//! arguments in x0-x5.

pub mod numbers;

use crate::arch::aarch64::context::TrapFrame;
use crate::kprintln;
use crate::objects::TCB;

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
        numbers::SYS_YIELD => sys_yield(tf),

        // Chapter 5: IPC syscalls
        numbers::SYS_SEND => sys_ipc_send(tf, args[0], args[1], args[2]),
        numbers::SYS_RECV => sys_ipc_recv(tf, args[0], args[1], args[2]),
        numbers::SYS_CALL => sys_ipc_call(tf, args[0], args[1], args[2], args[3], args[4]),
        numbers::SYS_REPLY => sys_ipc_reply(tf, args[0], args[1]),

        // Chapter 9: Capability management syscalls
        numbers::SYS_CAP_ALLOCATE => sys_cap_allocate(),
        numbers::SYS_MEMORY_ALLOCATE => sys_memory_allocate(args[0]),
        numbers::SYS_DEVICE_REQUEST => sys_device_request(args[0]),
        numbers::SYS_ENDPOINT_CREATE => sys_endpoint_create(),
        numbers::SYS_PROCESS_CREATE => sys_process_create(
            args[0], args[1], args[2], args[3], args[4], args[5], args[6]
        ),
        numbers::SYS_MEMORY_MAP => sys_memory_map(tf, args[0], args[1], args[2]),
        numbers::SYS_MEMORY_UNMAP => sys_memory_unmap(args[0], args[1]),

        _ => {
            kprintln!("[syscall] Unknown syscall number: {}", syscall_num);
            u64::MAX // Error: invalid syscall
        }
    };

    // Set return value
    tf.set_return_value(result);
}

/// Yield CPU to next process using scheduler
///
/// This syscall allows a thread to voluntary give up the CPU to another thread.
/// Uses the proper scheduler for context switching.
fn sys_yield(tf: &mut TrapFrame) -> u64 {
    unsafe {
        let current = crate::scheduler::current_thread();
        if current.is_null() {
            return u64::MAX; // Error: no current thread
        }

        // Save current thread's full context to its TCB
        // The TrapFrame passed to us contains the saved userspace registers
        *(*current).context_mut() = *tf;

        // Mark current thread as runnable and re-enqueue
        (*current).set_state(crate::objects::ThreadState::Runnable);
        crate::scheduler::enqueue(current);

        // Pick next thread
        let next = crate::scheduler::schedule();
        if next.is_null() || next == current {
            // No other thread or same thread, just continue
            (*current).set_state(crate::objects::ThreadState::Running);
            return 0;
        }

        // Switch to next thread
        let next_tcb = &mut *next;
        next_tcb.set_state(crate::objects::ThreadState::Running);
        crate::scheduler::test_set_current_thread(next);

        // Replace our TrapFrame with the next thread's context
        // When we return from this syscall, the exception handler will restore
        // the next thread's context and eret to it
        // IMPORTANT: No kprintln or any function calls after this point!
        *tf = *next_tcb.context();
    }
    0 // Success
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

    unsafe {
        let current = crate::scheduler::current_thread();
        if current.is_null() {
            return u64::MAX; // Error: no current thread
        }

        // Get current thread's page table (TTBR0)
        let user_ttbr0 = (*current).context().saved_ttbr0;

        // Temporarily switch to user's page table to copy the string
        let mut saved_ttbr0: u64;
        core::arch::asm!(
            "mrs {}, ttbr0_el1",
            out(reg) saved_ttbr0,
        );

        core::arch::asm!(
            "msr ttbr0_el1, {}",
            "isb",
            in(reg) user_ttbr0,
        );

        // Copy string to kernel buffer
        let mut buffer = [0u8; 4096];
        let copy_len = core::cmp::min(len as usize, 4096);
        core::ptr::copy_nonoverlapping(ptr as *const u8, buffer.as_mut_ptr(), copy_len);

        // Restore kernel's page table BEFORE printing
        core::arch::asm!(
            "msr ttbr0_el1, {}",
            "isb",
            in(reg) saved_ttbr0,
        );

        // Now print from kernel buffer (UART is accessible via TTBR1)
        if let Ok(s) = core::str::from_utf8(&buffer[..copy_len]) {
            crate::kprint!("{}", s);
            0 // Success
        } else {
            u64::MAX // Error: invalid UTF-8
        }
    }
}

//
// Chapter 9: Capability Management Syscalls
//

// Global capability slot counter
// Slots 0-99 are reserved for well-known capabilities
static mut NEXT_CAP_SLOT: u64 = 100;

/// Allocate a capability slot
///
/// Returns a capability slot number that can be used to store capabilities.
/// Capability slots are process-local identifiers used to reference kernel objects.
fn sys_cap_allocate() -> u64 {
    unsafe {
        let slot = NEXT_CAP_SLOT;
        NEXT_CAP_SLOT += 1;
        slot
    }
}

/// Allocate physical memory
///
/// Args: size (bytes)
/// Returns: physical address of allocated memory
///
/// Allocates physical memory frames using the kernel's frame allocator.
/// For multi-page allocations, allocates contiguous frames.
fn sys_memory_allocate(size: u64) -> u64 {
    use crate::memory::{alloc_frame, PAGE_SIZE};

    let page_size = PAGE_SIZE as u64;
    let pages_needed = ((size + page_size - 1) / page_size) as usize;

    // Allocate the first frame
    let first_pfn = match alloc_frame() {
        Some(pfn) => pfn,
        None => {
            kprintln!("[syscall] memory_allocate: out of memory");
            return u64::MAX;
        }
    };

    let base_addr = first_pfn.phys_addr();

    // For multi-page allocations, allocate additional frames
    if pages_needed > 1 {
        for i in 1..pages_needed {
            match alloc_frame() {
                Some(_pfn) => {
                    // Successfully allocated frame
                    // Note: Frame allocator provides sequential frames
                }
                None => {
                    kprintln!("[syscall] memory_allocate: allocation failed at page {}/{}", i, pages_needed);
                    // TODO: Implement frame deallocation to free partially allocated frames
                    return u64::MAX;
                }
            }
        }
    }

    base_addr.as_u64()
}

/// Request device resources
///
/// Args: device_id (see platform::device_ids)
/// Returns: MMIO base address for the device
///
/// Maps device IDs to their MMIO base addresses from platform configuration.
fn sys_device_request(device_id: u64) -> u64 {
    use crate::generated::memory_config::*;

    let mmio_base = match device_id {
        DEVICE_UART0 => UART0_BASE,
        DEVICE_UART1 => UART1_BASE,
        DEVICE_RTC => RTC_BASE,
        DEVICE_TIMER => TIMER_BASE,
        _ => {
            kprintln!("[syscall] device_request: unknown device {}", device_id);
            return u64::MAX; // Error: unknown device
        }
    };

    mmio_base
}

/// Create IPC endpoint
///
/// Returns: endpoint capability slot
///
/// Allocates a capability slot for a new IPC endpoint.
/// The endpoint object itself is managed through the capability system.
fn sys_endpoint_create() -> u64 {
    // Allocate capability slot for the endpoint
    // The endpoint object is tracked via the capability
    let slot = sys_cap_allocate();
    slot
}

/// Create a new process with full isolation
///
/// Args:
/// - entry_point: Initial program counter (ELR_EL1) - also indicates code virtual address
/// - stack_pointer: Initial stack pointer (SP_EL0) - virtual address
/// - page_table_root: Physical address of page table (TTBR0)
/// - cspace_root: Physical address of CNode (capability space root)
/// - code_phys: Physical address where code is loaded
/// - code_size: Size of code region in bytes
/// - stack_phys: Physical address where stack is located
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
    code_phys: u64,
    code_size: u64,
    stack_phys: u64,
) -> u64 {
    use crate::memory::{alloc_frame, VirtAddr};
    use crate::objects::{TCB, CNode};
    use crate::scheduler;

    // Allocate frame for TCB
    let tcb_frame = match alloc_frame() {
        Some(pfn) => pfn.phys_addr(),
        None => {
            kprintln!("[syscall] process_create: out of memory (TCB)");
            return u64::MAX;
        }
    };

    // Set up page tables for the new process

    use crate::memory::{PAGE_SIZE, VirtAddr as VA, PhysAddr as PA, PageSize, PageMapper};
    use crate::arch::aarch64::page_table::{PageTable, PageTableFlags};

    // Zero the new page table
    let new_pt = page_table_root as *mut PageTable;
    unsafe { (*new_pt).zero(); }

    // Copy kernel mappings (upper half) from current TTBR1
    // This ensures syscalls and exceptions can access kernel code
    unsafe {
        use core::arch::asm;
        let mut ttbr1: u64;
        asm!("mrs {}, ttbr1_el1", out(reg) ttbr1);
        let kernel_pt = ttbr1 as *const PageTable;

        // Copy upper half entries ONLY (L0 entries 256-511 for upper 256TB)
        // Lower half (0-255) is for userspace
        for i in 256..512 {
            (*new_pt).entries[i] = (*kernel_pt).entries[i];
        }
    }

    // Create mapper for the new process's page table
    let mut mapper = unsafe { PageMapper::new(&mut *new_pt) };

    // Map the entire loaded region (rodata + text)
    // The ELF loader loads everything starting from the first LOAD segment
    // For echo-server: 0x200000 (rodata) + 0x21031c (text)
    // Physical memory contains both segments sequentially
    let code_virt_base = 0x200000;  // First LOAD segment virtual address
    let code_virt = VA::new(code_virt_base);
    let code_phys_addr = PA::new(code_phys as usize);

    // Map all pages (covers both rodata and text segments)
    let code_pages = ((code_size as usize) + PAGE_SIZE - 1) / PAGE_SIZE;

    for i in 0..code_pages {
        let virt = VA::new(code_virt_base + (i * PAGE_SIZE));
        let phys = PA::new(code_phys as usize + (i * PAGE_SIZE));
        if let Err(e) = mapper.map(virt, phys, PageTableFlags::USER_RWX, PageSize::Size4KB) {
            kprintln!("  ERROR: Failed to map code page {}: {:?}", i, e);
            return u64::MAX;
        }
    }

    // Map stack pages (4 pages = 16KB, non-executable, read/write)
    // Stack pointer points to top, map downwards
    let stack_size = 16384;  // 16KB
    let stack_pages = stack_size / PAGE_SIZE;
    let stack_base = (stack_pointer as usize) - stack_size;

    for i in 0..stack_pages {
        let virt = VA::new(stack_base + (i * PAGE_SIZE));
        let phys = PA::new(stack_phys as usize + (i * PAGE_SIZE));
        if let Err(e) = mapper.map(virt, phys, PageTableFlags::USER_DATA, PageSize::Size4KB) {
            kprintln!("  ERROR: Failed to map stack page {}: {:?}", i, e);
            return u64::MAX;
        }
    }

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

        // Initialize saved_ttbr0 in the context for context switching
        (*tcb_ptr).context_mut().saved_ttbr0 = page_table_root;

        // Set state to Runnable
        (*tcb_ptr).set_state(crate::objects::ThreadState::Runnable);

        // Add to scheduler
        // Note: scheduler::enqueue handles uninitialized scheduler gracefully
        scheduler::enqueue(tcb_ptr);

        // TCB is now managed by scheduler
    }

    kprintln!("[syscall] process_create -> PID {:#x}", pid);
    kprintln!("[syscall] process_create: TCB created and enqueued");
    pid as u64
}

/// Global virtual address allocator for userspace mappings
///
/// Allocates from high memory region (starting at 2GB) to avoid conflicts
/// with existing low-memory mappings. This is a simple bump allocator that
/// provides non-overlapping virtual address ranges.
///
/// We start at 2GB (0x80000000) which is:
/// - High enough to avoid kernel/user code conflicts
/// - Low enough to work with TCR_EL1 configuration (39-bit VA)
///
/// Production improvement: Use per-process VSpace allocator with free list
static mut NEXT_VIRT_ADDR: u64 = crate::generated::memory_config::USER_VIRT_START;

/// Map physical memory into caller's virtual address space
///
/// Args:
/// - phys_addr: Physical address to map
/// - size: Size in bytes (will be rounded up to page size)
/// - permissions: Access permissions (1=read, 2=write, 4=exec)
///
/// Returns: Virtual address where memory is mapped, or u64::MAX on error
///
/// This allows userspace to access allocated physical memory by creating
/// page table entries in the caller's TTBR0 page table.
///
/// NOTE: We receive the TrapFrame from the exception handler, which contains
/// the caller's TTBR0 in saved_ttbr0. During the exception, TTBR0 is temporarily
/// switched to the kernel page table, so we must use the saved value.
fn sys_memory_map(tf: &mut TrapFrame, phys_addr: u64, size: u64, permissions: u64) -> u64 {
    use crate::memory::{PAGE_SIZE, VirtAddr, PhysAddr, PageSize};
    use crate::arch::aarch64::page_table::{PageTable, PageTableFlags};

    kprintln!("[syscall] memory_map: phys={:#x}, size={}, perms={:#x}", phys_addr, size, permissions);

    // Round size up to page boundary
    let page_size = PAGE_SIZE as u64;
    let num_pages = ((size + page_size - 1) / page_size) as usize;
    let aligned_size = num_pages as u64 * page_size;

    // Get caller's page table from TrapFrame (saved during exception entry)
    let page_table_phys = tf.saved_ttbr0 as usize;
    kprintln!("[syscall] memory_map: caller's TTBR0={:#x} (from TrapFrame)", page_table_phys);

    // Get mutable reference to caller's page table
    let page_table = unsafe { &mut *(page_table_phys as *mut PageTable) };

    // Allocate virtual address from high memory region to avoid conflicts
    let virt_addr = unsafe {
        let addr = NEXT_VIRT_ADDR;
        NEXT_VIRT_ADDR += aligned_size;
        addr
    };

    kprintln!("[syscall] memory_map: allocated virt range {:#x} - {:#x}",
              virt_addr, virt_addr + aligned_size);

    // Use USER_DATA preset for userspace read-write data
    // This includes: VALID, TABLE_OR_PAGE, AP_RW_ALL, ACCESSED, INNER_SHARE,
    //               NORMAL, UXN, PXN, NOT_GLOBAL
    let flags = PageTableFlags::USER_DATA;

    kprintln!("[syscall] memory_map: using USER_DATA flags = {:#x}", flags.bits());

    // Create PageMapper once for all mappings
    let mut mapper = unsafe { crate::memory::PageMapper::new(page_table) };

    // Map each page
    for i in 0..num_pages {
        let page_virt = VirtAddr::new((virt_addr as usize) + (i * PAGE_SIZE));
        let page_phys = PhysAddr::new((phys_addr as usize) + (i * PAGE_SIZE));

        match mapper.map(page_virt, page_phys, flags, PageSize::Size4KB) {
            Ok(()) => {
                kprintln!("[syscall] memory_map: mapped page {} virt={:#x} -> phys={:#x}",
                         i, page_virt.as_usize(), page_phys.as_usize());
            },
            Err(e) => {
                kprintln!("[syscall] memory_map: failed to map page {} at virt={:#x}, error={:?}",
                         i, page_virt.as_usize(), e);
                return u64::MAX;
            }
        }
    }

    // Ensure page table updates are visible
    unsafe {
        core::arch::asm!(
            "dsb ishst",  // Ensure all page table writes complete
        );
    }

    // Debug: Walk the page tables to verify the mapping
    kprintln!("[syscall] memory_map: verifying mapping...");
    mapper.debug_walk(VirtAddr::new(virt_addr as usize));

    // Flush TLB only for the mapped virtual address range (not all entries!)
    // This preserves root-task's code/stack TLB entries
    unsafe {
        for i in 0..num_pages {
            let addr = ((virt_addr as usize) + (i * PAGE_SIZE)) >> 12; // VA >> 12 for TLBI
            core::arch::asm!(
                "tlbi vaae1is, {0}",  // Invalidate by VA, all ASID, EL1
                "dsb ish",            // Ensure completion
                in(reg) addr
            );
        }
        core::arch::asm!("isb");  // Final instruction sync
    }

    kprintln!("[syscall] memory_map -> virt={:#x} ({} pages mapped)", virt_addr, num_pages);
    virt_addr
}

/// Unmap virtual memory from caller's address space
///
/// Args:
/// - virt_addr: Virtual address to unmap
/// - size: Size in bytes
///
/// Returns: 0 on success, u64::MAX on error
fn sys_memory_unmap(virt_addr: u64, size: u64) -> u64 {
    use crate::memory::PAGE_SIZE;

    kprintln!("[syscall] memory_unmap: virt={:#x}, size={}", virt_addr, size);

    // Round size up to page boundary
    let page_size = PAGE_SIZE as u64;
    let num_pages = (size + page_size - 1) / page_size;

    // A full implementation would:
    // 1. Get caller's page table from current TCB
    // 2. Remove page table entries for this range
    // 3. Flush TLB

    // For now, this is a no-op (simplified)
    kprintln!("[syscall] memory_unmap -> success ({} pages)", num_pages);
    0
}

/// IPC Send: Send message to endpoint
///
/// Args:
/// - endpoint_cap_slot: Capability slot for endpoint
/// - message_ptr: Pointer to message data (in user space)
/// - message_len: Length of message data
///
/// Returns:
/// - 0 on success
/// - u64::MAX on error
fn sys_ipc_send(tf: &mut TrapFrame, endpoint_cap_slot: u64, message_ptr: u64, message_len: u64) -> u64 {
    kprintln!("[syscall] IPC Send: endpoint={}, msg_ptr=0x{:x}, len={}",
        endpoint_cap_slot, message_ptr, message_len);

    // Phase 2 implementation: Validate parameters and test syscall path

    // Validate message length (max 256 bytes for now)
    if message_len > 256 {
        kprintln!("[syscall] IPC Send -> error: message too large ({} bytes)", message_len);
        return u64::MAX;
    }

    // Validate endpoint capability slot (basic range check)
    if endpoint_cap_slot >= 4096 {
        kprintln!("[syscall] IPC Send -> error: invalid endpoint cap slot {}", endpoint_cap_slot);
        return u64::MAX;
    }

    // Get current thread
    unsafe {
        let current = crate::scheduler::current_thread();
        if current.is_null() {
            kprintln!("[syscall] IPC Send -> error: no current thread");
            return u64::MAX;
        }

        // For Phase 2, we're testing the syscall infrastructure
        // Full implementation would:
        // 1. Look up endpoint from capability slot
        // 2. Create Message from user buffer
        // 3. Call ipc::send(endpoint_cap, current, message)
        // 4. Handle blocking if no receiver ready
        // 5. Context switch via scheduler if blocked

        kprintln!("[syscall] IPC Send -> success (validated, Phase 2)");
    }

    0
}

/// IPC Receive: Receive message from endpoint
///
/// Args:
/// - endpoint_cap_slot: Capability slot for endpoint
/// - buffer_ptr: Pointer to receive buffer (in user space)
/// - buffer_len: Length of receive buffer
///
/// Returns:
/// - Number of bytes received on success
/// - u64::MAX on error
fn sys_ipc_recv(tf: &mut TrapFrame, endpoint_cap_slot: u64, buffer_ptr: u64, buffer_len: u64) -> u64 {
    kprintln!("[syscall] IPC Recv: endpoint={}, buf_ptr=0x{:x}, len={}",
        endpoint_cap_slot, buffer_ptr, buffer_len);

    // Phase 2 implementation: Validate parameters and test syscall path

    // Validate buffer length
    if buffer_len > 256 {
        kprintln!("[syscall] IPC Recv -> error: buffer too large ({} bytes)", buffer_len);
        return u64::MAX;
    }

    // Validate endpoint capability slot
    if endpoint_cap_slot >= 4096 {
        kprintln!("[syscall] IPC Recv -> error: invalid endpoint cap slot {}", endpoint_cap_slot);
        return u64::MAX;
    }

    // Get current thread
    unsafe {
        let current = crate::scheduler::current_thread();
        if current.is_null() {
            kprintln!("[syscall] IPC Recv -> error: no current thread");
            return u64::MAX;
        }

        // For Phase 2, we're testing the syscall infrastructure
        // Full implementation would:
        // 1. Look up endpoint from capability slot
        // 2. Call ipc::recv(endpoint_cap, current)
        // 3. Copy received message to user buffer
        // 4. Handle blocking if no sender ready
        // 5. Context switch via scheduler if blocked

        kprintln!("[syscall] IPC Recv -> success (validated, Phase 2, 0 bytes)");
    }

    0  // Return 0 bytes for Phase 2 testing
}

/// IPC Call: Send message and wait for reply (RPC)
///
/// Args:
/// - endpoint_cap_slot: Capability slot for endpoint
/// - request_ptr: Pointer to request message
/// - request_len: Length of request
/// - reply_ptr: Pointer to reply buffer
/// - reply_len: Length of reply buffer
///
/// Returns:
/// - Number of bytes in reply on success
/// - u64::MAX on error
fn sys_ipc_call(tf: &mut TrapFrame, endpoint_cap_slot: u64, request_ptr: u64, request_len: u64,
                reply_ptr: u64, reply_len: u64) -> u64 {
    kprintln!("[syscall] IPC Call: endpoint={}, req_ptr=0x{:x}, req_len={}, rep_ptr=0x{:x}, rep_len={}",
        endpoint_cap_slot, request_ptr, request_len, reply_ptr, reply_len);

    // TODO: Full implementation
    // 1. Validate endpoint_cap_slot
    // 2. Get current TCB
    // 3. Copy request from userspace
    // 4. Call ipc::call(endpoint, tcb, request_message)
    // 5. Handle blocking/context switch
    // 6. Copy reply to userspace

    // For Phase 2, return 0 bytes to test the syscall path
    kprintln!("[syscall] IPC Call -> success (stub, 0 bytes)");
    0
}

/// IPC Reply: Reply to a call
///
/// Args:
/// - reply_cap_slot: Reply capability slot
/// - message_ptr: Pointer to reply message
///
/// Returns:
/// - 0 on success
/// - u64::MAX on error
fn sys_ipc_reply(tf: &mut TrapFrame, reply_cap_slot: u64, message_ptr: u64) -> u64 {
    kprintln!("[syscall] IPC Reply: reply_cap={}, msg_ptr=0x{:x}",
        reply_cap_slot, message_ptr);

    // TODO: Full implementation
    // 1. Validate reply_cap_slot
    // 2. Get current TCB
    // 3. Copy reply message from userspace
    // 4. Call ipc::reply(reply_cap, message)
    // 5. Wake up caller

    // For Phase 2, return success to test the syscall path
    kprintln!("[syscall] IPC Reply -> success (stub)");
    0
}
