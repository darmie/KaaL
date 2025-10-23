//! System call interface
//!
//! This module implements the syscall dispatcher for the KaaL microkernel.
//! Syscalls follow the seL4 convention with syscall number in x8 and
//! arguments in x0-x5.

pub mod numbers;
pub mod channel;

use crate::arch::aarch64::context::TrapFrame;
use crate::{kprintln, ksyscall_debug};
use crate::objects::{TCB, Endpoint, Notification};
use core::ptr;

/// Shared memory registry entry
#[derive(Copy, Clone)]
struct ShmemEntry {
    name: [u8; 32],      // Channel name (null-terminated)
    name_len: usize,     // Actual length of name
    phys_addr: usize,    // Physical address of shared memory
    size: usize,         // Size in bytes
    valid: bool,         // Whether this entry is in use
}

impl ShmemEntry {
    const fn new() -> Self {
        ShmemEntry {
            name: [0; 32],
            name_len: 0,
            phys_addr: 0,
            size: 0,
            valid: false,
        }
    }
}

/// Global shared memory registry (kernel-managed)
///
/// # Architecture Note
///
/// This registry is currently implemented in the kernel for simplicity, but in a
/// proper microkernel architecture, it belongs in the userspace capability broker.
///
/// ## Migration Path to Userspace Broker
///
/// The syscalls SYS_SHMEM_REGISTER and SYS_SHMEM_QUERY should become IPC calls to
/// a broker endpoint. The broker (in runtime/ipc/src/broker.rs) already has the
/// shmem_registry infrastructure ready for this migration.
///
/// Benefits of userspace broker:
/// - Keeps kernel minimal (microkernel principle)
/// - Policy decisions (access control, quotas) in userspace
/// - Easier testing and debugging
///
/// The current implementation works correctly for Phase 6 demonstration.
static mut SHMEM_REGISTRY: [ShmemEntry; 16] = [ShmemEntry::new(); 16];

/// Look up an endpoint capability from the current thread's CSpace
///
/// Returns pointer to Endpoint object, or null if:
/// - cap_slot is invalid
/// - capability not found in CSpace
/// - capability is not an Endpoint type
unsafe fn lookup_endpoint_capability(cap_slot: usize) -> *mut Endpoint {
    use crate::objects::CapType;

    // Get current thread's CSpace root
    let current_tcb = crate::scheduler::current_thread();
    if current_tcb.is_null() {
        ksyscall_debug!("[syscall] lookup_endpoint: no current thread");
        return ptr::null_mut();
    }

    let cspace_root = (*current_tcb).cspace_root();
    if cspace_root.is_null() {
        ksyscall_debug!("[syscall] lookup_endpoint: thread has no CSpace root");
        return ptr::null_mut();
    }

    // Look up capability in CSpace
    let cnode = &*cspace_root;
    let cap = match cnode.lookup(cap_slot) {
        Some(c) => c,
        None => {
            ksyscall_debug!("[syscall] lookup_endpoint: cap_slot {} not found in CSpace", cap_slot);
            return ptr::null_mut();
        }
    };

    // Verify it's an Endpoint capability
    if cap.cap_type() != CapType::Endpoint {
        ksyscall_debug!("[syscall] lookup_endpoint: cap_slot {} is not an Endpoint (type={:?})",
                 cap_slot, cap.cap_type());
        return ptr::null_mut();
    }

    // Return pointer to Endpoint object
    cap.object_ptr() as *mut Endpoint
}

/// Insert an endpoint capability into the current thread's CSpace
///
/// Returns true on success, false on error
unsafe fn insert_endpoint_capability(cap_slot: usize, endpoint: *mut Endpoint) -> bool {
    use crate::objects::cnode_cdt::CNodeCdt;
    use crate::objects::{Capability, CapType};

    // Get current thread's CSpace root
    let current_tcb = crate::scheduler::current_thread();
    if current_tcb.is_null() {
        ksyscall_debug!("[syscall] insert_endpoint: no current thread");
        return false;
    }

    let cspace_root = (*current_tcb).cspace_root();
    if cspace_root.is_null() {
        ksyscall_debug!("[syscall] insert_endpoint: thread has no CSpace root");
        return false;
    }

    // Create Endpoint capability
    let cap = Capability::new(CapType::Endpoint, endpoint as usize);

    // Insert into CSpace using CNodeCdt (insert_root for root capabilities)
    let cnode = &mut *(cspace_root as *mut CNodeCdt);
    match cnode.insert_root(cap_slot, cap) {
        Ok(()) => {
            ksyscall_debug!("[syscall] insert_endpoint: cap_slot {} -> endpoint {:p}", cap_slot, endpoint);
            true
        }
        Err(e) => {
            ksyscall_debug!("[syscall] insert_endpoint: failed to insert at cap_slot {}: {:?}", cap_slot, e);
            false
        }
    }
}

/// Copy data from userspace to kernel space
///
/// Temporarily switches to the caller's TTBR0 to access userspace memory.
/// This is safe because we're running in EL1 with kernel permissions.
///
/// # Safety
/// - user_ptr must be a valid userspace pointer
/// - len must not exceed buffer sizes
/// - caller_ttbr0 must be the physical address of a valid page table
unsafe fn copy_from_user(user_ptr: u64, kernel_buf: &mut [u8], len: usize, caller_ttbr0: u64) -> bool {
    if len == 0 || len > kernel_buf.len() {
        return false;
    }

    // Save current TTBR0
    let saved_ttbr0: u64;
    core::arch::asm!(
        "mrs {}, ttbr0_el1",
        out(reg) saved_ttbr0,
    );

    // Switch to caller's TTBR0 to access userspace memory
    core::arch::asm!(
        "msr ttbr0_el1, {}",
        "isb",
        in(reg) caller_ttbr0,
    );

    // Copy data from userspace
    let user_slice = core::slice::from_raw_parts(user_ptr as *const u8, len);
    kernel_buf[..len].copy_from_slice(user_slice);

    // Restore kernel's TTBR0
    core::arch::asm!(
        "msr ttbr0_el1, {}",
        "isb",
        in(reg) saved_ttbr0,
    );

    true
}

/// Copy data from kernel space to userspace
///
/// Temporarily switches to the caller's TTBR0 to access userspace memory.
///
/// # Safety
/// - user_ptr must be a valid userspace pointer
/// - len must not exceed buffer sizes
/// - caller_ttbr0 must be the physical address of a valid page table
unsafe fn copy_to_user(kernel_buf: &[u8], user_ptr: u64, len: usize, caller_ttbr0: u64) -> bool {
    if len == 0 || len > kernel_buf.len() {
        return false;
    }

    // Save current TTBR0
    let saved_ttbr0: u64;
    core::arch::asm!(
        "mrs {}, ttbr0_el1",
        out(reg) saved_ttbr0,
    );

    // Switch to caller's TTBR0 to access userspace memory
    core::arch::asm!(
        "msr ttbr0_el1, {}",
        "isb",
        in(reg) caller_ttbr0,
    );

    // Copy data to userspace
    let user_slice = core::slice::from_raw_parts_mut(user_ptr as *mut u8, len);
    user_slice.copy_from_slice(&kernel_buf[..len]);

    // Restore kernel's TTBR0
    core::arch::asm!(
        "msr ttbr0_el1, {}",
        "isb",
        in(reg) saved_ttbr0,
    );

    true
}

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
        numbers::SYS_DEBUG_PRINT => sys_debug_print(tf, args[0], args[1]),
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
            tf,  // Pass TrapFrame to set extra return values
            args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7],
            tf.x9,  // Priority passed in x9
            tf.x10  // Capabilities passed in x10
        ),
        numbers::SYS_MEMORY_MAP => sys_memory_map(tf, args[0], args[1], args[2]),
        numbers::SYS_MEMORY_UNMAP => sys_memory_unmap(args[0], args[1]),
        numbers::SYS_MEMORY_REMAP => sys_memory_remap(args[0], args[1], args[2]),
        numbers::SYS_MEMORY_SHARE => sys_memory_share(args[0], args[1], args[2], args[3], args[4]),
        numbers::SYS_RETYPE => sys_retype(args[0], args[1], args[2], args[3], args[4]),
        numbers::SYS_MEMORY_MAP_INTO => sys_memory_map_into(args[0], args[1], args[2], args[3], args[4]),
        numbers::SYS_CAP_INSERT_INTO => sys_cap_insert_into(args[0], args[1], args[2], args[3]),
        numbers::SYS_CAP_INSERT_SELF => sys_cap_insert_self(args[0], args[1], args[2]),
        numbers::SYS_CAP_REVOKE => sys_cap_revoke(args[0], args[1]),
        numbers::SYS_CAP_DERIVE => sys_cap_derive(args[0], args[1], args[2], args[3]),
        numbers::SYS_CAP_MINT => sys_cap_mint(args[0], args[1], args[2], args[3]),
        numbers::SYS_CAP_COPY => sys_cap_copy(args[0], args[1], args[2], args[3]),
        numbers::SYS_CAP_DELETE => sys_cap_delete(args[0], args[1]),
        numbers::SYS_CAP_MOVE => sys_cap_move(args[0], args[1], args[2], args[3]),

        // Chapter 9 Phase 2: Notification syscalls for shared memory IPC
        numbers::SYS_NOTIFICATION_CREATE => sys_notification_create(),
        numbers::SYS_SIGNAL => sys_signal(args[0], args[1]),
        numbers::SYS_WAIT => sys_wait(tf, args[0]),
        numbers::SYS_POLL => sys_poll(args[0]),

        // Chapter 9 Phase 6: Channel management syscalls
        numbers::SYS_CHANNEL_ESTABLISH => channel::sys_channel_establish(tf, args[0], args[1], args[2]),
        numbers::SYS_CHANNEL_QUERY => channel::sys_channel_query(args[0]),
        numbers::SYS_CHANNEL_CLOSE => channel::sys_channel_close(args[0]),

        // Shared memory registry syscalls
        numbers::SYS_SHMEM_REGISTER => sys_shmem_register(tf, args[0], args[1], args[2], args[3]),
        numbers::SYS_SHMEM_QUERY => sys_shmem_query(tf, args[0], args[1]),

        // IRQ handling syscalls
        numbers::SYS_IRQ_HANDLER_GET => sys_irq_handler_get(tf, args[0], args[1], args[2], args[3]),
        numbers::SYS_IRQ_HANDLER_ACK => sys_irq_handler_ack(tf, args[0]),

        // System control syscalls
        numbers::SYS_SHUTDOWN => sys_shutdown(),

        _ => {
            ksyscall_debug!("[syscall] Unknown syscall number: {} from ELR={:#x}, x8={:#x}",
                     syscall_num, tf.elr_el1, tf.syscall_number());
            // If this is happening repeatedly, stop spamming
            static mut UNKNOWN_COUNT: u32 = 0;
            unsafe {
                UNKNOWN_COUNT += 1;
                if UNKNOWN_COUNT > 10 {
                    panic!("Too many unknown syscalls");
                }
            }
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

        let current_tid = (*current).tid();
        crate::kprintln!("[syscall] sys_yield: current TCB {:p} (TID {}) yielding", current, current_tid);

        // Save current thread's full context to its TCB
        // The TrapFrame passed to us contains the saved userspace registers
        *(*current).context_mut() = *tf;

        // Mark current thread as runnable and re-enqueue
        (*current).set_state(crate::objects::ThreadState::Runnable);
        crate::scheduler::enqueue(current);

        // Pick next thread
        let next = crate::scheduler::schedule();
        if next.is_null() {
            // No threads available - this shouldn't happen with idle thread
            crate::kprintln!("[syscall] sys_yield: schedule() returned null, continuing current");
            (*current).set_state(crate::objects::ThreadState::Running);
            return 0;
        }

        let next_tid = (*next).tid();

        if next == current {
            // Scheduler picked the same thread because:
            // - We just enqueued current
            // - schedule() immediately dequeued it (was head of queue)
            // - This means current was the ONLY thread at its priority
            // Keep it running, no context switch needed
            crate::kprintln!("[syscall] sys_yield: schedule() returned SAME thread {}, no switch", next_tid);
            (*current).set_state(crate::objects::ThreadState::Running);
            // NOTE: thread is NO LONGER in queue after schedule() dequeued it
            // This is correct - running thread shouldn't be in ready queue
            return 0;
        }

        crate::kprintln!("[syscall] sys_yield: switching from TCB {:p} (TID {}) to TCB {:p} (TID {})",
                         current, current_tid, next, next_tid);

        // Switch to next thread
        let next_tcb = &mut *next;
        next_tcb.set_state(crate::objects::ThreadState::Running);
        crate::scheduler::test_set_current_thread(next);

        // Replace our TrapFrame with the next thread's context
        // When we return from this syscall, the exception handler will restore
        // the next thread's context and eret to it
        let next_context = next_tcb.context();
        crate::kprintln!("[syscall] sys_yield: next TCB={:p}, context at {:p}", next, next_context as *const _);
        crate::kprintln!("[syscall] sys_yield: next thread PC=0x{:x}, SP=0x{:x}, x29=0x{:x}, TTBR0=0x{:x}",
                         next_context.elr_el1, next_context.sp_el0, next_context.x29, next_context.saved_ttbr0);
        *tf = *next_context;

        // CRITICAL: Ensure TrapFrame write completes before exception handler reads it
        unsafe {
            core::arch::asm!(
                "dsb sy",  // Data Synchronization Barrier (full system)
                options(nostack),
            );
        }

        // DEBUG: Verify TrapFrame was copied correctly
        crate::kprintln!("[syscall] sys_yield: AFTER COPY - tf.sp_el0=0x{:x}, tf.elr_el1=0x{:x}, tf.x29=0x{:x}",
                         tf.sp_el0, tf.elr_el1, tf.x29);

        // CRITICAL: Switch TTBR0 NOW to the next thread's page table
        // The exception handler will restore this same value when we eret,
        // but we need to switch now so any kernel operations use the correct
        // page table (e.g., when kernel reads from user memory).
        unsafe {
            core::arch::asm!(
                "msr ttbr0_el1, {ttbr0}",    // Switch to next thread's page table
                "dsb ish",                     // Ensure page table switch completes
                "tlbi vmalle1is",              // Invalidate all TLB entries
                "dsb ish",                     // Ensure TLB invalidation completes
                "isb",                         // Synchronize instruction fetch
                ttbr0 = in(reg) next_context.saved_ttbr0,
            );
        }
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
/// Uses copy_from_user to safely access userspace memory by temporarily
/// switching to the calling process's TTBR0 page table.
fn sys_debug_print(tf: &TrapFrame, ptr: u64, len: u64) -> u64 {
    // Debug: log the syscall (commented out to reduce noise)
    // crate::kprintln!("[syscall] sys_debug_print: ptr={:#x}, len={}, ttbr0={:#x}",
    //                 ptr, len, tf.saved_ttbr0);

    // Validate length (prevent abuse)
    if len > 4096 {
        ksyscall_debug!("[syscall] sys_debug_print: string too long ({})", len);
        return u64::MAX; // Error: string too long
    }

    // Allocate kernel buffer
    let mut buffer = [0u8; 4096];
    let copy_len = core::cmp::min(len as usize, 4096);

    // Get caller's TTBR0 from TrapFrame
    let caller_ttbr0 = tf.saved_ttbr0;

    // Debug: Check if TTBR0 is valid
    if caller_ttbr0 == 0 {
        crate::kprintln!("[ERROR] sys_debug_print: saved_ttbr0 is 0!");
        return u64::MAX;
    }

    // Copy from userspace using TTBR0 switching
    if !unsafe { copy_from_user(ptr, &mut buffer, copy_len, caller_ttbr0) } {
        ksyscall_debug!("[syscall] sys_debug_print: failed to copy from user");
        return u64::MAX; // Error: failed to copy from user
    }

    // Print from kernel buffer
    if let Ok(s) = core::str::from_utf8(&buffer[..copy_len]) {
        crate::kprint!("{}", s);
        0 // Success
    } else {
        ksyscall_debug!("[syscall] sys_debug_print: invalid UTF-8");
        u64::MAX // Error: invalid UTF-8
    }
}

//
// Chapter 9: Capability Management Syscalls
//

/// Allocate a capability slot
///
/// Returns a capability slot number that can be used to store capabilities.
/// Capability slots are process-local identifiers used to reference kernel objects.
/// Each process has its own capability slot namespace starting at slot 100.
fn sys_cap_allocate() -> u64 {
    // Check if caller has capability management capability
    unsafe {
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            ksyscall_debug!("[syscall] cap_allocate: no current thread");
            return u64::MAX;
        }

        if !(*current_tcb).has_capability(TCB::CAP_CAPS) {
            ksyscall_debug!("[syscall] cap_allocate: caller lacks CAP_CAPS capability");
            return u64::MAX; // Permission denied
        }

        // Allocate from the thread's per-process capability slot allocator
        let tcb_mut = &mut *current_tcb;
        tcb_mut.alloc_cap_slot()
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

    // Check if caller has memory allocation capability
    unsafe {
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            ksyscall_debug!("[syscall] memory_allocate: no current thread");
            return u64::MAX;
        }

        if !(*current_tcb).has_capability(TCB::CAP_MEMORY) {
            ksyscall_debug!("[syscall] memory_allocate: caller lacks CAP_MEMORY capability");
            return u64::MAX; // Permission denied
        }
    }

    let page_size = PAGE_SIZE as u64;
    let pages_needed = size.div_ceil(page_size) as usize;

    // Allocate the first frame
    let first_pfn = match alloc_frame() {
        Some(pfn) => pfn,
        None => {
            ksyscall_debug!("[syscall] memory_allocate: out of memory");
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
                    ksyscall_debug!("[syscall] memory_allocate: allocation failed at page {}/{}", i, pages_needed);
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

    

    match device_id {
        DEVICE_UART0 => UART0_BASE,
        DEVICE_UART1 => UART1_BASE,
        DEVICE_RTC => RTC_BASE,
        DEVICE_TIMER => TIMER_BASE,
        _ => {
            ksyscall_debug!("[syscall] device_request: unknown device {}", device_id);
            return u64::MAX; // Error: unknown device
        }
    }
}

/// Create IPC endpoint
///
/// Returns: endpoint capability slot
///
/// Allocates a capability slot for a new IPC endpoint.
/// The endpoint object itself is managed through the capability system.
fn sys_endpoint_create() -> u64 {
    use crate::objects::Endpoint;
    use crate::memory::alloc_frame;
    use core::ptr;

    // Allocate a physical frame for the Endpoint object
    let endpoint_frame = match unsafe { alloc_frame() } {
        Some(pfn) => pfn,
        None => {
            ksyscall_debug!("[syscall] endpoint_create: out of memory");
            return u64::MAX;
        }
    };

    let endpoint_phys = endpoint_frame.phys_addr();
    ksyscall_debug!("[syscall] endpoint_create: allocated frame at phys 0x{:x}", endpoint_phys.as_u64());

    // Create the Endpoint object
    let endpoint_ptr = endpoint_phys.as_u64() as *mut Endpoint;
    unsafe {
        ptr::write(endpoint_ptr, Endpoint::new());
        ksyscall_debug!("[syscall] endpoint_create: created Endpoint at 0x{:x}", endpoint_ptr as u64);
    }

    // Allocate capability slot for the endpoint
    let slot = sys_cap_allocate();

    // Insert endpoint capability into current thread's CSpace
    unsafe {
        if !insert_endpoint_capability(slot as usize, endpoint_ptr) {
            ksyscall_debug!("[syscall] endpoint_create: failed to insert capability into CSpace");
            return u64::MAX;
        }
    }

    ksyscall_debug!("[syscall] endpoint_create -> cap_slot={}, endpoint capability inserted into CSpace", slot);
    slot
}

/// Create a new process with full isolation
///
/// Args:
/// - entry_point: Initial program counter (ELR_EL1)
/// - stack_pointer: Initial stack pointer (SP_EL0) - virtual address
/// - page_table_root: Physical address of page table (TTBR0)
/// - cspace_root: Physical address of CNode (capability space root)
/// - code_phys: Physical address where code is loaded
/// - code_vaddr: Virtual address where code should be mapped (from ELF min_vaddr)
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
    tf: &mut TrapFrame,  // TrapFrame to set extra return values
    entry_point: u64,
    stack_pointer: u64,
    page_table_root: u64,
    cspace_root: u64,
    code_phys: u64,
    code_vaddr: u64,
    code_size: u64,
    stack_phys: u64,
    priority: u64,  // Priority parameter from x9
    capabilities: u64,  // Capabilities parameter from x10
) -> u64 {
    use crate::memory::{alloc_frame, VirtAddr};
    use crate::objects::{TCB, CNode};
    use crate::scheduler;

    // Check if caller has process creation capability
    unsafe {
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            ksyscall_debug!("[syscall] process_create: no current thread");
            return u64::MAX;
        }

        if !(*current_tcb).has_capability(TCB::CAP_PROCESS) {
            ksyscall_debug!("[syscall] process_create: caller lacks CAP_PROCESS capability");
            return u64::MAX; // Permission denied
        }
    }

    // Debug output (always show for debugging spawned components)
    crate::kprintln!("[syscall] sys_process_create: entry={:#x}, stack={:#x}, pt={:#x}, priority={}",
                     entry_point, stack_pointer, page_table_root, priority);
    crate::kprintln!("[syscall] sys_process_create: code_phys={:#x}, code_vaddr={:#x}, code_size={:#x}, stack_phys={:#x}",
                     code_phys, code_vaddr, code_size, stack_phys);

    // Allocate frame for TCB
    let tcb_frame = match alloc_frame() {
        Some(pfn) => pfn.phys_addr(),
        None => {
            ksyscall_debug!("[syscall] process_create: out of memory (TCB)");
            return u64::MAX;
        }
    };

    // Set up page tables for the new process with unified kernel+user mappings

    use crate::memory::{PAGE_SIZE, VirtAddr as VA, PhysAddr as PA, PageSize, PageMapper};
    use crate::arch::aarch64::page_table::{PageTable, PageTableFlags};

    // Zero the new page table
    let new_pt = page_table_root as *mut PageTable;
    unsafe { (*new_pt).zero(); }

    // Create mapper for the new process's page table
    let mut mapper = unsafe { PageMapper::new(&mut *new_pt) };

    // Map kernel regions with EL1-only permissions (same as root task)
    // This allows exception handlers to run while preventing user code from accessing kernel memory
    extern "C" {
        static _kernel_start: u8;
        static _kernel_end: u8;
    }
    let kernel_start = unsafe { &_kernel_start as *const u8 as usize };
    let kernel_end = unsafe { &_kernel_end as *const u8 as usize };

    // Map kernel code/data
    if let Err(e) = crate::memory::paging::identity_map_region(
        &mut mapper,
        kernel_start,
        kernel_end - kernel_start,
        PageTableFlags::KERNEL_RWX,
    ) {
        ksyscall_debug!("[syscall] process_create: failed to map kernel code: {:?}", e);
        return u64::MAX;
    }

    // Map kernel data region (where kernel data structures live)
    use crate::boot::bootinfo::get_boot_info;
    let boot_info = get_boot_info().expect("[FATAL] Boot info not available");
    let memory_end = boot_info.dtb_addr.as_usize() + 0x8000000; // RAM end (128MB)

    if let Err(e) = crate::memory::paging::identity_map_region(
        &mut mapper,
        kernel_end,
        memory_end - kernel_end,
        PageTableFlags::KERNEL_DATA,
    ) {
        ksyscall_debug!("[syscall] process_create: failed to map kernel data: {:?}", e);
        return u64::MAX;
    }

    // Map UART device for syscall output
    use crate::generated::memory_config;
    if let Err(e) = crate::memory::paging::identity_map_region(
        &mut mapper,
        memory_config::UART0_BASE as usize,
        4096,
        PageTableFlags::KERNEL_DEVICE,
    ) {
        ksyscall_debug!("[syscall] process_create: failed to map UART: {:?}", e);
        return u64::MAX;
    }

    // Map GIC (interrupt controller) for IRQ handling in syscalls
    if let Err(e) = crate::memory::paging::identity_map_region(
        &mut mapper,
        memory_config::GIC_DIST_BASE,
        memory_config::GIC_DIST_SIZE,
        PageTableFlags::KERNEL_DEVICE,
    ) {
        ksyscall_debug!("[syscall] process_create: failed to map GIC distributor: {:?}", e);
        return u64::MAX;
    }

    if let Err(e) = crate::memory::paging::identity_map_region(
        &mut mapper,
        memory_config::GIC_CPU_BASE,
        memory_config::GIC_CPU_SIZE,
        PageTableFlags::KERNEL_DEVICE,
    ) {
        ksyscall_debug!("[syscall] process_create: failed to map GIC CPU interface: {:?}", e);
        return u64::MAX;
    }

    // Map the code region at the virtual address expected by the ELF
    // The caller has:
    // - Loaded the ELF binary at code_phys (physical address)
    // - Parsed the ELF to find min_vaddr (code_vaddr)
    // - We map: code_vaddr -> code_phys
    ksyscall_debug!("[syscall] process_create: entry={:#x}, code_phys={:#x}, code_vaddr={:#x}, code_size={:#x}",
        entry_point, code_phys, code_vaddr, code_size);

    let code_virt_base = code_vaddr as usize;
    let code_pages = (code_size as usize).div_ceil(PAGE_SIZE);

    ksyscall_debug!("[syscall] process_create: mapping {} code pages at virt={:#x} -> phys={:#x}",
        code_pages, code_virt_base, code_phys);

    for i in 0..code_pages {
        let virt = VA::new(code_virt_base + (i * PAGE_SIZE));
        let phys = PA::new(code_phys as usize + (i * PAGE_SIZE));
        crate::kprintln!("[syscall] Mapping page {}: virt={:#x} -> phys={:#x}", i, virt.as_usize(), phys.as_usize());
        if let Err(e) = mapper.map(virt, phys, PageTableFlags::USER_RWX, PageSize::Size4KB) {
            kprintln!("  ERROR: Failed to map code page {}: {:?}", i, e);
            return u64::MAX;
        }
    }

    // Flush instruction cache for code pages to ensure CPU sees latest mappings
    unsafe {
        core::arch::asm!(
            "ic iallu",           // Invalidate all instruction caches
            "dsb ish",            // Data synchronization barrier
            "isb",                // Instruction synchronization barrier
        );
    }

    ksyscall_debug!("[syscall] process_create: code mapped successfully");
    ksyscall_debug!("[syscall] process_create: entry_point={:#x} should be in mapped range {:#x}-{:#x}",
             entry_point, code_virt_base, code_virt_base + (code_pages * PAGE_SIZE));

    // Map stack pages (4 pages = 16KB, non-executable, read/write)
    // Stack pointer points to top, map downwards
    let stack_size = 16384;  // 16KB
    let stack_pages = stack_size / PAGE_SIZE;
    let stack_base = (stack_pointer as usize) - stack_size;

    ksyscall_debug!("[syscall] process_create: mapping stack at {:#x}-{:#x} (SP={:#x})",
             stack_base, stack_pointer, stack_pointer);
    for i in 0..stack_pages {
        let virt = VA::new(stack_base + (i * PAGE_SIZE));
        let phys = PA::new(stack_phys as usize + (i * PAGE_SIZE));
        if let Err(e) = mapper.map(virt, phys, PageTableFlags::USER_DATA, PageSize::Size4KB) {
            kprintln!("  ERROR: Failed to map stack page {}: {:?}", i, e);
            return u64::MAX;
        }
    }

    // Flush TLB and ensure page table updates are visible
    unsafe {
        core::arch::asm!(
            "dsb ishst",           // Ensure all page table writes complete
            "tlbi vmalle1is",      // Invalidate all TLB entries for EL1
            "dsb ish",             // Ensure TLB invalidation completes
            "isb",                 // Synchronize context
        );
    }

    ksyscall_debug!("[syscall] process_create: page tables set up and TLB flushed");

    // Generate process ID (use frame address for now - unique per process)
    let pid = tcb_frame.as_usize();

    // Initialize CNode at the allocated physical address
    // Memory layout from caller (root-task):
    // - cspace_root + 0x0: CNode struct (~24 bytes)
    // - cspace_root + 0x1000: Capability slots array (256 × 32 = 8KB = 2 pages)
    // This avoids overlap with TCB which is allocated immediately after
    let cnode_phys = PA::new(cspace_root as usize);
    let cspace_ptr = cspace_root as *mut CNode;
    let slots_phys = PA::new((cspace_root as usize) + 0x1000); // Slots start 1 page after CNode

    // Create CNode with 256 slots (2^8 = 256 capabilities)
    unsafe {
        let cnode = CNode::new(8, slots_phys)  // Use separate slots address!
            .expect("[FATAL] Failed to create CNode for new process");

        // Write initialized CNode to allocated memory
        core::ptr::write(cspace_ptr, cnode);
    }

    ksyscall_debug!("[syscall] process_create: CNode initialized with 256 slots at {:#x}", cspace_root);

    // Allocate IPC buffer (for now, placeholder address)
    // TODO: Should allocate actual IPC buffer frame
    let ipc_buffer = VirtAddr::new(0x8000_0000);

    // Create TCB
    let tcb_ptr = tcb_frame.as_usize() as *mut TCB;
    unsafe {
        // Use capabilities specified by caller
        let tcb = TCB::new(
            pid,
            cspace_ptr,
            page_table_root as usize,
            ipc_buffer,
            entry_point,
            stack_pointer,
            capabilities,  // Capabilities passed from caller
        );
        core::ptr::write(tcb_ptr, tcb);

        // Initialize saved_ttbr0 in the context for context switching
        (*tcb_ptr).context_mut().saved_ttbr0 = page_table_root;
        crate::kprintln!("[syscall] process_create: set saved_ttbr0={:#x} for TCB={:#x}",
                        page_table_root, tcb_ptr as usize);

        // DEBUG: Verify TCB context was initialized correctly
        let ctx = (*tcb_ptr).context();
        crate::kprintln!("[syscall] process_create: TCB context AFTER init - PC={:#x}, SP={:#x}, x29={:#x}, TTBR0={:#x}",
                         ctx.elr_el1, ctx.sp_el0, ctx.x29, ctx.saved_ttbr0);

        // Set the priority from the component manifest
        // NOTE: In our scheduler, lower numbers = higher priority!
        (*tcb_ptr).set_priority(priority as u8);
        crate::kprintln!("[syscall] process_create: set priority {} for component", priority);

        // Set state to Runnable
        (*tcb_ptr).set_state(crate::objects::ThreadState::Runnable);

        // Add to scheduler
        // Note: scheduler::enqueue handles uninitialized scheduler gracefully
        crate::kprintln!("[syscall] process_create: enqueuing TCB at {:#x}", tcb_ptr as usize);
        scheduler::enqueue(tcb_ptr);

        // TCB is now managed by scheduler
    }

    crate::kprintln!("[syscall] process_create: SUCCESS - PID={:#x}", pid);

    // Store capability information in TrapFrame for caller
    // x0 = PID (return value - set by dispatcher)
    // x1 = TCB physical address
    // x2 = Page table root
    // x3 = CSpace root
    tf.x1 = tcb_frame.as_usize() as u64;
    tf.x2 = page_table_root;
    tf.x3 = cspace_root;

    crate::kprintln!("[syscall] process_create: set TrapFrame - x1={:#x}, x2={:#x}, x3={:#x}",
                     tf.x1, tf.x2, tf.x3);

    // Full TLB and cache flush to ensure all processes see correct memory
    unsafe {
        core::arch::asm!(
            "dsb ishst",          // Data sync barrier
            "tlbi vmalle1is",     // Invalidate all TLB entries for all PEs
            "dsb ish",            // Ensure TLB invalidation completes
            "isb",                // Instruction sync barrier
        );
    }

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

    // Check if caller has memory mapping capability
    let current_tcb = unsafe { crate::scheduler::current_thread() };
    if current_tcb.is_null() {
        ksyscall_debug!("[syscall] memory_map: no current thread");
        return u64::MAX;
    }

    unsafe {
        if !(*current_tcb).has_capability(TCB::CAP_MEMORY) {
            ksyscall_debug!("[syscall] memory_map: caller lacks CAP_MEMORY capability");
            return u64::MAX; // Permission denied
        }
    }

    crate::kprintln!("[syscall] memory_map: phys={:#x}, size={:#x}, perms={:#x}", phys_addr, size, permissions);

    // Round size up to page boundary
    let page_size = PAGE_SIZE as u64;
    let num_pages = size.div_ceil(page_size) as usize;
    let aligned_size = num_pages as u64 * page_size;

    // Get caller's page table from TrapFrame (saved during exception entry)
    let page_table_phys = tf.saved_ttbr0 as usize;
    ksyscall_debug!("[syscall] memory_map: caller's TTBR0={:#x} (from TrapFrame)", page_table_phys);

    // Get mutable reference to caller's page table
    let page_table = unsafe { &mut *(page_table_phys as *mut PageTable) };

    // Allocate virtual address from the caller's per-thread allocator
    let virt_addr = unsafe { (*current_tcb).alloc_virt_range(aligned_size) };

    crate::kprintln!("[syscall] memory_map: allocated virt range {:#x} - {:#x}, mapping {} pages",
              virt_addr, virt_addr + aligned_size, num_pages);

    // Use USER_DATA preset for userspace read-write data
    // This includes: VALID, TABLE_OR_PAGE, AP_RW_ALL, ACCESSED, INNER_SHARE,
    //               NORMAL, UXN, PXN, NOT_GLOBAL
    let flags = PageTableFlags::USER_DATA;

    ksyscall_debug!("[syscall] memory_map: using USER_DATA flags = {:#x}", flags.bits());

    // Create PageMapper once for all mappings
    let mut mapper = unsafe { crate::memory::PageMapper::new(page_table) };

    // Map each page
    for i in 0..num_pages {
        let page_virt = VirtAddr::new((virt_addr as usize) + (i * PAGE_SIZE));
        let page_phys = PhysAddr::new((phys_addr as usize) + (i * PAGE_SIZE));

        match mapper.map(page_virt, page_phys, flags, PageSize::Size4KB) {
            Ok(()) => {
                ksyscall_debug!("[syscall] memory_map: mapped page {} virt={:#x} -> phys={:#x}",
                         i, page_virt.as_usize(), page_phys.as_usize());
            },
            Err(e) => {
                crate::kprintln!("[syscall] memory_map: failed to map page {} at virt={:#x}, error={:?}",
                         i, page_virt.as_usize(), e);
                crate::kprintln!("[syscall] memory_map: phys_addr={:#x}, num_pages={}, failed at page {}",
                         phys_addr, num_pages, i);
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

    // Note: TLB will be naturally flushed on context switches
    // For new mappings, the TLB won't have stale entries since these addresses weren't mapped before
    // So we don't need explicit TLB invalidation here

    crate::kprintln!("[syscall] memory_map: SUCCESS - returning virt={:#x}", virt_addr);
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
    use crate::memory::{PAGE_SIZE, VirtAddr as VA, PageSize, PageMapper};
    use crate::arch::aarch64::page_table::PageTable;

    ksyscall_debug!("[syscall] memory_unmap: virt={:#x}, size={}", virt_addr, size);

    // Check if caller has memory mapping capability
    let current_tcb = unsafe { crate::scheduler::current_thread() };
    if current_tcb.is_null() {
        ksyscall_debug!("[syscall] memory_unmap: no current thread");
        return u64::MAX;
    }

    unsafe {
        if !(*current_tcb).has_capability(TCB::CAP_MEMORY) {
            ksyscall_debug!("[syscall] memory_unmap: caller lacks CAP_MEMORY capability");
            return u64::MAX; // Permission denied
        }
    }

    // Round size up to page boundary
    let page_size = PAGE_SIZE as u64;
    let num_pages = size.div_ceil(page_size) as usize;

    // Get caller's page table from current TCB

    let page_table_phys = unsafe { (*current_tcb).vspace_root() };
    let page_table = page_table_phys as *mut PageTable;

    // Create mapper for caller's page table
    let mut mapper = unsafe { PageMapper::new(&mut *page_table) };

    // CRITICAL: Clean data cache to Point of Coherency before unmapping
    // This ensures any writes to this memory are visible to instruction fetches
    // Without this, newly written code may not be visible when executed!
    unsafe {
        for i in 0..num_pages {
            let cache_line_size = 64; // ARM64 typical cache line size
            let page_addr = virt_addr + (i as u64 * PAGE_SIZE as u64);
            for offset in (0..PAGE_SIZE).step_by(cache_line_size) {
                let addr = page_addr + offset as u64;
                core::arch::asm!(
                    "dc cvac, {addr}",     // Clean data cache by VA to PoC
                    addr = in(reg) addr,
                );
            }
        }
        core::arch::asm!(
            "dsb ish",                // Ensure cache cleaning completes
            "ic iallu",               // Invalidate all instruction caches
            "dsb ish",                // Ensure instruction cache invalidation completes
            "isb",                    // Synchronize context
        );
    }

    // Unmap each page in the range
    for i in 0..num_pages {
        let virt = VA::new(virt_addr as usize + (i * PAGE_SIZE));
        if let Err(e) = mapper.unmap(virt, PageSize::Size4KB) {
            ksyscall_debug!("[syscall] memory_unmap: failed to unmap page {}: {:?}", i, e);
            // Continue unmapping other pages even if one fails
        }
    }

    // Flush TLB to ensure unmapped pages are not cached
    unsafe {
        core::arch::asm!(
            "dsb ishst",           // Ensure page table writes complete
            "tlbi vmalle1is",      // Invalidate all TLB entries for EL1
            "dsb ish",             // Ensure TLB invalidation completes
            "isb",                 // Synchronize context
        );
    }

    ksyscall_debug!("[syscall] memory_unmap -> success ({} pages)", num_pages);
    0
}

/// Change memory protection flags for existing mapping
///
/// Args:
/// - virt_addr: Virtual address of the region
/// - size: Size in bytes
/// - new_permissions: New permission bits (read=1, write=2, exec=4)
///
/// Returns: 0 on success, u64::MAX on error
fn sys_memory_remap(virt_addr: u64, size: u64, new_permissions: u64) -> u64 {
    use crate::memory::{PAGE_SIZE, VirtAddr, PageSize, PageMapper};
    use crate::arch::aarch64::page_table::{PageTable, PageTableFlags};

    ksyscall_debug!("[syscall] memory_remap: virt={:#x}, size={:#x}, perms={:#x}",
                   virt_addr, size, new_permissions);

    // Check if caller has memory mapping capability
    let current_tcb = unsafe { crate::scheduler::current_thread() };
    if current_tcb.is_null() {
        ksyscall_debug!("[syscall] memory_remap: no current thread");
        return u64::MAX;
    }

    unsafe {
        if !(*current_tcb).has_capability(TCB::CAP_MEMORY) {
            ksyscall_debug!("[syscall] memory_remap: caller lacks CAP_MEMORY capability");
            return u64::MAX;
        }
    }

    // Round size up to page boundary
    let page_size = PAGE_SIZE as u64;
    let num_pages = size.div_ceil(page_size) as usize;

    // Get caller's page table from current TCB
    let page_table_phys = unsafe { (*current_tcb).vspace_root() };
    let page_table = page_table_phys as *mut PageTable;

    // Create mapper for caller's page table
    let mut mapper = unsafe { PageMapper::new(&mut *page_table) };

    // Determine new flags based on permissions
    let mut flags = PageTableFlags::VALID | PageTableFlags::ACCESSED | PageTableFlags::TABLE_OR_PAGE;

    // Add permission flags
    if new_permissions & 0x1 != 0 { // Read
        flags |= PageTableFlags::AP_RW_ALL; // Allow all ELs access
    }
    if new_permissions & 0x2 == 0 { // NOT Write (read-only)
        flags |= PageTableFlags::AP_RO_ALL; // Make read-only
    }
    if new_permissions & 0x4 == 0 { // NOT Execute
        flags |= PageTableFlags::UXN; // User execute never
        flags |= PageTableFlags::PXN; // Privileged execute never
    }

    // Add default flags for userspace
    flags |= PageTableFlags::INNER_SHARE | PageTableFlags::NORMAL | PageTableFlags::NOT_GLOBAL;

    ksyscall_debug!("[syscall] memory_remap: new flags = {:#x}", flags.bits());

    // Remap each page with new permissions
    // Since PageMapper doesn't have remap(), we unmap and remap
    for i in 0..num_pages {
        let virt = VirtAddr::new(virt_addr as usize + (i * PAGE_SIZE));

        // Get physical address before unmapping
        let phys = match mapper.translate(virt) {
            Some(paddr) => paddr,
            None => {
                ksyscall_debug!("[syscall] memory_remap: page {} not mapped", i);
                return u64::MAX;
            }
        };

        // Unmap the page
        if let Err(e) = mapper.unmap(virt, PageSize::Size4KB) {
            ksyscall_debug!("[syscall] memory_remap: failed to unmap page {}: {:?}", i, e);
            return u64::MAX;
        }

        // Remap with new permissions
        if let Err(e) = mapper.map(virt, phys, flags, PageSize::Size4KB) {
            ksyscall_debug!("[syscall] memory_remap: failed to map page {} with new perms: {:?}", i, e);
            return u64::MAX;
        }
    }

    // Flush TLB to ensure new permissions take effect
    unsafe {
        core::arch::asm!(
            "dsb ishst",           // Ensure page table writes complete
            "tlbi vmalle1is",      // Invalidate all TLB entries for EL1
            "dsb ish",             // Ensure TLB invalidation completes
            "isb",                 // Synchronize context
        );
    }

    ksyscall_debug!("[syscall] memory_remap -> success ({} pages)", num_pages);
    0
}

/// Share memory between processes
///
/// Args:
/// - target_tcb_cap: Capability slot for target process's TCB
/// - source_virt_addr: Virtual address in caller's address space
/// - size: Size in bytes
/// - dest_virt_addr: Virtual address in target's address space
/// - permissions: Permission bits for target (read=1, write=2, exec=4)
///
/// Returns: 0 on success, u64::MAX on error
fn sys_memory_share(target_tcb_cap: u64, source_virt_addr: u64, size: u64,
                    dest_virt_addr: u64, permissions: u64) -> u64 {
    use crate::memory::{PAGE_SIZE, VirtAddr, PhysAddr, PageSize, PageMapper};
    use crate::arch::aarch64::page_table::{PageTable, PageTableFlags};
    use crate::objects::cnode_cdt::CNodeCdt;
    use crate::objects::CapType;

    ksyscall_debug!("[syscall] memory_share: target_cap={}, src_virt={:#x}, size={:#x}, dest_virt={:#x}, perms={:#x}",
                   target_tcb_cap, source_virt_addr, size, dest_virt_addr, permissions);

    // Check if caller has memory mapping capability
    let current_tcb = unsafe { crate::scheduler::current_thread() };
    if current_tcb.is_null() {
        ksyscall_debug!("[syscall] memory_share: no current thread");
        return u64::MAX;
    }

    unsafe {
        if !(*current_tcb).has_capability(TCB::CAP_MEMORY) {
            ksyscall_debug!("[syscall] memory_share: caller lacks CAP_MEMORY capability");
            return u64::MAX;
        }

        // Lookup target TCB capability
        let cspace_root = (*current_tcb).cspace_root();
        if cspace_root.is_null() {
            ksyscall_debug!("[syscall] memory_share: no CSpace root");
            return u64::MAX;
        }

        let caller_cspace = &*(cspace_root as *const CNodeCdt);
        let tcb_capability = match caller_cspace.lookup(target_tcb_cap as usize) {
            Some(cap) => cap,
            None => {
                ksyscall_debug!("[syscall] memory_share: TCB capability not found");
                return u64::MAX;
            }
        };

        // Verify it's a TCB capability
        if tcb_capability.cap_type() != CapType::Tcb {
            ksyscall_debug!("[syscall] memory_share: not a TCB capability");
            return u64::MAX;
        }

        // Get target TCB
        let target_tcb = tcb_capability.object_ptr() as *mut TCB;
        if target_tcb.is_null() {
            ksyscall_debug!("[syscall] memory_share: target TCB is null");
            return u64::MAX;
        }

        // Get source and target page tables
        let source_page_table = (*current_tcb).vspace_root() as *mut PageTable;
        let target_page_table = (*target_tcb).vspace_root() as *mut PageTable;

        let mut source_mapper = PageMapper::new(&mut *source_page_table);
        let mut target_mapper = PageMapper::new(&mut *target_page_table);

        // Calculate number of pages
        let page_size = PAGE_SIZE as u64;
        let num_pages = size.div_ceil(page_size) as usize;

        // Build flags for target mapping
        let mut flags = PageTableFlags::USER_DATA;
        if permissions & 0x2 == 0 { // NOT Write (read-only)
            flags |= PageTableFlags::AP_RO_ALL;
        }
        if permissions & 0x4 == 0 { // NOT Execute
            flags |= PageTableFlags::UXN;
            flags |= PageTableFlags::PXN;
        }

        // For each page, translate source virt -> phys, then map into target
        for i in 0..num_pages {
            let src_virt = VirtAddr::new(source_virt_addr as usize + (i * PAGE_SIZE));
            let dest_virt = VirtAddr::new(dest_virt_addr as usize + (i * PAGE_SIZE));

            // Translate source virtual address to physical
            let phys_addr = match source_mapper.translate(src_virt) {
                Some(paddr) => paddr,
                None => {
                    ksyscall_debug!("[syscall] memory_share: failed to translate src virt={:#x}",
                                   src_virt.as_usize());
                    return u64::MAX;
                }
            };

            // Map into target's address space
            if let Err(e) = target_mapper.map(dest_virt, phys_addr, flags, PageSize::Size4KB) {
                ksyscall_debug!("[syscall] memory_share: failed to map page {} into target: {:?}", i, e);
                return u64::MAX;
            }

            ksyscall_debug!("[syscall] memory_share: shared page {} src={:#x} -> phys={:#x} -> dest={:#x}",
                           i, src_virt.as_usize(), phys_addr.as_usize(), dest_virt.as_usize());
        }

        // Flush TLB for target process
        core::arch::asm!(
            "dsb ishst",
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
        );

        ksyscall_debug!("[syscall] memory_share -> success ({} pages)", num_pages);
        0
    }
}

/// Map physical memory into target process's virtual address space (Phase 5)
///
/// Args:
/// - target_tcb_cap: Capability slot for target process's TCB
/// - phys_addr: Physical address to map
/// - size: Size in bytes
/// - virt_addr: Target virtual address in target process (caller specifies)
/// - permissions: Permission bits (read=1, write=2, exec=4)
///
/// Returns: 0 on success, u64::MAX on error
///
/// This allows one process (e.g., root-task) to map shared memory into another
/// process's address space at a specific virtual address, enabling inter-process
/// IPC via shared memory. The caller must have a TCB capability for the target.
fn sys_memory_map_into(target_tcb_cap: u64, phys_addr: u64, size: u64, virt_addr: u64, permissions: u64) -> u64 {
    use crate::memory::{PAGE_SIZE, VirtAddr, PhysAddr, PageSize};
    use crate::arch::aarch64::page_table::{PageTable, PageTableFlags};
    use crate::objects::CapType;

    crate::kprintln!("[syscall] memory_map_into: target_tcb_cap={}, phys={:#x}, size={}, virt={:#x}, perms={:#x}",
              target_tcb_cap, phys_addr, size, virt_addr, permissions);

    // Round size up to page boundary
    let page_size = PAGE_SIZE as u64;
    let num_pages = size.div_ceil(page_size) as usize;
    let aligned_size = num_pages as u64 * page_size;

    // Look up target TCB capability from caller's CSpace
    unsafe {
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            ksyscall_debug!("[syscall] memory_map_into: no current thread");
            return u64::MAX;
        }

        let cspace_root = (*current_tcb).cspace_root();
        if cspace_root.is_null() {
            ksyscall_debug!("[syscall] memory_map_into: thread has no CSpace root");
            return u64::MAX;
        }

        // Look up target TCB capability
        let cnode = &*cspace_root;
        let cap = match cnode.lookup(target_tcb_cap as usize) {
            Some(c) => c,
            None => {
                crate::kprintln!("[syscall] memory_map_into: ✗ cap_slot {} not found in CSpace", target_tcb_cap);
                return u64::MAX;
            }
        };

        // Verify it's a TCB capability
        if cap.cap_type() != CapType::Tcb {
            crate::kprintln!("[syscall] memory_map_into: ✗ cap_slot {} is not a TCB (type={:?})",
                     target_tcb_cap, cap.cap_type());
            return u64::MAX;
        }

        crate::kprintln!("[syscall] memory_map_into: ✓ found TCB cap at slot {}", target_tcb_cap);

        // Get target TCB pointer
        let target_tcb_ptr = cap.object_ptr() as *mut TCB;
        crate::kprintln!("[syscall] memory_map_into: target_tcb_ptr={:#x}", target_tcb_ptr as usize);
        if target_tcb_ptr.is_null() {
            crate::kprintln!("[syscall] memory_map_into: ✗ null target TCB pointer");
            return u64::MAX;
        }

        // Get target process's page table (TTBR0)
        let target_ttbr0 = (*target_tcb_ptr).context().saved_ttbr0;
        crate::kprintln!("[syscall] memory_map_into: target TTBR0={:#x}", target_ttbr0);

        let target_page_table = &mut *(target_ttbr0 as *mut PageTable);

        // Use caller-provided virtual address
        // Caller is responsible for choosing non-conflicting addresses
        crate::kprintln!("[syscall] memory_map_into: mapping to virt range {:#x} - {:#x} in target process",
                  virt_addr, virt_addr + aligned_size);

        // Use USER_DATA preset for userspace read-write data
        let flags = PageTableFlags::USER_DATA;

        ksyscall_debug!("[syscall] memory_map_into: using USER_DATA flags = {:#x}", flags.bits());

        // Create PageMapper for target's page table
        let mut mapper = crate::memory::PageMapper::new(target_page_table);

        // Map each page into target's address space
        for i in 0..num_pages {
            let page_virt = VirtAddr::new((virt_addr as usize) + (i * PAGE_SIZE));
            let page_phys = PhysAddr::new((phys_addr as usize) + (i * PAGE_SIZE));

            match mapper.map(page_virt, page_phys, flags, PageSize::Size4KB) {
                Ok(()) => {
                    crate::kprintln!("[syscall] memory_map_into: ✓ mapped page {} virt={:#x} -> phys={:#x}",
                             i, page_virt.as_usize(), page_phys.as_usize());
                },
                Err(e) => {
                    crate::kprintln!("[syscall] memory_map_into: ✗ failed to map page {}: {:?}",
                             i, e);
                    return u64::MAX;
                }
            }
        }

        // Ensure page table updates are visible
        core::arch::asm!(
            "dsb ishst",  // Ensure all page table writes complete
        );

        crate::kprintln!("[syscall] memory_map_into: ✓ SUCCESS ({} pages mapped)", num_pages);
        0  // Success
    }
}

/// Retype untyped memory into a kernel object (seL4-style capability-based spawning)
///
/// Args:
/// - untyped_cap_slot: Capability slot containing UntypedMemory capability
/// - object_type: Type of object to create (1=Untyped, 4=TCB, 5=CNode, 8=Page, etc.)
/// - size_bits: Size of object as log2 (e.g., 12 = 4KB, 20 = 1MB)
/// - dest_cnode_cap: CNode capability where new cap should be inserted
/// - dest_slot: Slot in dest CNode for new capability
///
/// Returns: Physical address of new object on success, u64::MAX on error
///
/// This is the PROPER way for userspace to spawn processes:
/// 1. Caller must have UntypedMemory capability (delegated from root-task)
/// 2. Kernel carves object from untyped using watermark allocator
/// 3. New capability is inserted into caller's CSpace
/// 4. Original untyped tracks child for revocation
///
/// Security: Can ONLY create objects from Untyped caps caller already has.
/// Cannot forge capabilities or access other processes' memory.
fn sys_retype(untyped_cap_slot: u64, object_type: u64, size_bits: u64,
              dest_cnode_cap: u64, dest_slot: u64) -> u64 {
    use crate::objects::{Capability, CapType, UntypedMemory};
    use crate::objects::cnode_cdt::CNodeCdt;
    use crate::memory::PhysAddr;

    crate::kprintln!("[syscall] retype: untyped_slot={}, obj_type={}, size_bits={}, dest_cnode={}, dest_slot={}",
              untyped_cap_slot, object_type, size_bits, dest_cnode_cap, dest_slot);

    unsafe {
        // Get caller's CSpace
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            crate::kprintln!("[syscall] retype: no current thread");
            return u64::MAX;
        }

        let cspace_root = (*current_tcb).cspace_root();
        if cspace_root.is_null() {
            crate::kprintln!("[syscall] retype: no CSpace root");
            return u64::MAX;
        }

        let caller_cspace = &mut *(cspace_root as *mut CNodeCdt);

        // 1. Lookup UntypedMemory capability
        let untyped_cap = match caller_cspace.lookup(untyped_cap_slot as usize) {
            Some(cap) => cap,
            None => {
                crate::kprintln!("[syscall] retype: untyped cap not found at slot {}", untyped_cap_slot);
                return u64::MAX;
            }
        };

        // Verify it's an UntypedMemory capability
        if untyped_cap.cap_type() != CapType::UntypedMemory {
            crate::kprintln!("[syscall] retype: slot {} is not UntypedMemory (found {:?})",
                            untyped_cap_slot, untyped_cap.cap_type());
            return u64::MAX;
        }

        // Get mutable reference to UntypedMemory object
        let untyped_ptr = untyped_cap.object_ptr() as *mut UntypedMemory;
        if untyped_ptr.is_null() {
            crate::kprintln!("[syscall] retype: untyped object pointer is null");
            return u64::MAX;
        }

        let untyped = &mut *untyped_ptr;

        // 2. Convert object_type number to CapType enum
        let target_type = match object_type {
            1 => CapType::UntypedMemory,
            2 => CapType::Endpoint,
            3 => CapType::Notification,
            4 => CapType::Tcb,
            5 => CapType::CNode,
            6 => CapType::VSpace,
            7 => CapType::PageTable,
            8 => CapType::Page,
            _ => {
                crate::kprintln!("[syscall] retype: invalid object type {}", object_type);
                return u64::MAX;
            }
        };

        // 3. Retype: allocate from untyped memory
        let obj_paddr = match untyped.retype(target_type, size_bits as u8) {
            Ok(paddr) => paddr,
            Err(e) => {
                crate::kprintln!("[syscall] retype: failed to retype - {:?}", e);
                return u64::MAX;
            }
        };

        crate::kprintln!("[syscall] retype: allocated {:?} at phys={:#x}, size=2^{} bytes",
                        target_type, obj_paddr.as_u64(), size_bits);

        // 4. Lookup destination CNode capability
        let dest_cnode_cap_obj = if dest_cnode_cap == 0 {
            // 0 means "use caller's own CSpace"
            cspace_root as *const CNodeCdt
        } else {
            match caller_cspace.lookup(dest_cnode_cap as usize) {
                Some(cap) if cap.cap_type() == CapType::CNode => {
                    cap.object_ptr() as *const CNodeCdt
                }
                _ => {
                    crate::kprintln!("[syscall] retype: dest_cnode_cap {} not found or not CNode", dest_cnode_cap);
                    return u64::MAX;
                }
            }
        };

        let dest_cnode = &mut *(dest_cnode_cap_obj as *mut CNodeCdt);

        // 5. Create new capability for the allocated object
        let new_cap = Capability::new(target_type, obj_paddr.as_u64() as usize);

        // 6. Insert capability into destination CNode
        if let Err(e) = dest_cnode.insert(dest_slot as usize, new_cap) {
            crate::kprintln!("[syscall] retype: failed to insert cap into slot {}: {:?}", dest_slot, e);
            return u64::MAX;
        }

        crate::kprintln!("[syscall] retype: ✓ created {:?} cap at slot {}", target_type, dest_slot);

        // Return physical address of new object
        obj_paddr.as_u64()
    }
}

/// Insert capability into target process's CSpace (Phase 5)
///
/// Args:
/// - target_tcb_cap: Capability slot for target process's TCB
/// - target_slot: Slot in target's CSpace to insert capability
/// - cap_type: Type of capability to insert
/// - object_ptr: Physical address of the capability object
///
/// Returns: 0 on success, u64::MAX on error
///
/// This allows one process (e.g., root-task) to grant capabilities to another
/// process by inserting them into the target's CSpace. The caller must have a
/// TCB capability for the target process. This is used to pass notification
/// capabilities and other resources to spawned components.
fn sys_cap_insert_into(target_tcb_cap: u64, target_slot: u64, cap_type: u64, object_ptr: u64) -> u64 {
    use crate::objects::{Capability, CapType};

    crate::kprintln!("[syscall] cap_insert_into: target_tcb={}, slot={}, type={}, obj={:#x}",
              target_tcb_cap, target_slot, cap_type, object_ptr);

    unsafe {
        // Get current thread's CSpace
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            ksyscall_debug!("[syscall] cap_insert_into: no current thread");
            return u64::MAX;
        }

        // Check if caller has capability management capability
        if !(*current_tcb).has_capability(TCB::CAP_CAPS) {
            ksyscall_debug!("[syscall] cap_insert_into: caller lacks CAP_CAPS capability");
            return u64::MAX; // Permission denied
        }

        let cspace_root = (*current_tcb).cspace_root();
        if cspace_root.is_null() {
            ksyscall_debug!("[syscall] cap_insert_into: thread has no CSpace root");
            return u64::MAX;
        }

        // Look up target TCB capability from caller's CSpace
        let cnode = &*cspace_root;
        let tcb_cap = match cnode.lookup(target_tcb_cap as usize) {
            Some(c) => c,
            None => {
                ksyscall_debug!("[syscall] cap_insert_into: TCB cap_slot {} not found", target_tcb_cap);
                return u64::MAX;
            }
        };

        // Verify it's a TCB capability
        if tcb_cap.cap_type() != CapType::Tcb {
            ksyscall_debug!("[syscall] cap_insert_into: cap_slot {} is not a TCB (type={:?})",
                     target_tcb_cap, tcb_cap.cap_type());
            return u64::MAX;
        }

        // Get target TCB and its CSpace
        let target_tcb_ptr = tcb_cap.object_ptr() as *mut TCB;
        if target_tcb_ptr.is_null() {
            ksyscall_debug!("[syscall] cap_insert_into: null target TCB pointer");
            return u64::MAX;
        }

        let target_cspace = (*target_tcb_ptr).cspace_root();
        if target_cspace.is_null() {
            ksyscall_debug!("[syscall] cap_insert_into: target has no CSpace");
            return u64::MAX;
        }

        // Convert cap_type from u64 to CapType enum
        let cap_type_enum = match cap_type {
            0 => CapType::Null,
            1 => CapType::UntypedMemory,
            2 => CapType::Endpoint,
            3 => CapType::Notification,
            4 => CapType::Tcb,
            5 => CapType::CNode,
            6 => CapType::VSpace,
            7 => CapType::Page,
            8 => CapType::PageTable,
            9 => CapType::IrqHandler,
            10 => CapType::IrqControl,
            11 => CapType::Reply,
            _ => {
                ksyscall_debug!("[syscall] cap_insert_into: invalid cap_type {}", cap_type);
                return u64::MAX;
            }
        };

        // Create the capability
        let cap = Capability::new(cap_type_enum, object_ptr as usize);

        // Insert into target's CSpace
        let target_cnode = &mut *target_cspace;

        // Debug: Check if slot is already occupied
        if !target_cnode.is_empty(target_slot as usize) {
            crate::kprintln!("[syscall] cap_insert_into: slot {} already occupied", target_slot);
            if let Some(existing_cap) = target_cnode.lookup(target_slot as usize) {
                crate::kprintln!("[syscall] cap_insert_into: existing cap type: {:?}", existing_cap.cap_type());
            }
        }

        match target_cnode.insert(target_slot as usize, cap) {
            Ok(()) => {
                crate::kprintln!("[syscall] cap_insert_into: ✓ inserted {:?} cap at slot {}", cap_type_enum, target_slot);
                0
            }
            Err(e) => {
                crate::kprintln!("[syscall] cap_insert_into: ✗ failed to insert: {:?}", e);
                u64::MAX
            }
        }
    }
}

/// Revoke capability and all its descendants (seL4-style CDT revocation)
///
/// Recursively deletes the capability at the specified slot in the caller's CSpace
/// along with all capabilities derived from it. This implements seL4's capability
/// revocation using the CDT (Capability Derivation Tree).
///
/// # Arguments
/// - cnode_cap: CNode capability slot (currently unused, uses caller's CSpace root)
/// - slot: Slot number to revoke
///
/// # Returns
/// 0 on success, u64::MAX on error
///
/// # Security
/// - Requires WRITE rights on CNode capability
/// - Ensures no dangling capabilities remain after revocation
/// - Recursively removes entire derivation subtree
fn sys_cap_revoke(cnode_cap: u64, slot: u64) -> u64 {
    use crate::objects::cnode_cdt::CNodeCdt;
    use crate::objects::{CapType, CapRights};

    ksyscall_debug!("[syscall] cap_revoke: cnode_cap={}, slot={}", cnode_cap, slot);

    unsafe {
        // Get current thread's CSpace
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            ksyscall_debug!("[syscall] cap_revoke: no current thread");
            return u64::MAX;
        }

        // Check if caller has capability management capability
        if !(*current_tcb).has_capability(TCB::CAP_CAPS) {
            ksyscall_debug!("[syscall] cap_revoke: caller lacks CAP_CAPS capability");
            return u64::MAX; // Permission denied
        }

        let cspace_root = (*current_tcb).cspace_root();
        if cspace_root.is_null() {
            ksyscall_debug!("[syscall] cap_revoke: thread has no CSpace root");
            return u64::MAX;
        }

        let caller_cspace = &*(cspace_root as *const CNodeCdt);

        // Special case: cnode_cap == 0 means "use caller's CSpace root directly"
        // This allows userspace to operate on its own CSpace without needing a capability to it
        let target_cnode = if cnode_cap == 0 {
            ksyscall_debug!("[syscall] cap_revoke: using caller's own CSpace (cnode_cap=0)");
            &mut *(cspace_root as *mut CNodeCdt)
        } else {
            // ✅ seL4-style implementation: Lookup CNode capability with proper validation
            // 1. Lookup the CNode capability from caller's CSpace
            let cnode_capability = match caller_cspace.lookup(cnode_cap as usize) {
                Some(cap) => cap,
                None => {
                    ksyscall_debug!("[syscall] cap_revoke: CNode capability not found at slot {}", cnode_cap);
                    return u64::MAX;
                }
            };

            // 2. Verify it's a CNode capability
            if cnode_capability.cap_type() != CapType::CNode {
                ksyscall_debug!("[syscall] cap_revoke: slot {} is not a CNode (type={:?})",
                              cnode_cap, cnode_capability.cap_type());
                return u64::MAX;
            }

            // 3. Verify WRITE rights on the CNode
            if !cnode_capability.rights().contains(CapRights::WRITE) {
                ksyscall_debug!("[syscall] cap_revoke: insufficient rights on CNode (need WRITE)");
                return u64::MAX;
            }

            // 4. Get the target CNode object
            &mut *(cnode_capability.object_ptr() as *mut CNodeCdt)
        };

        // Revoke the capability at the specified slot in the TARGET CNode
        match target_cnode.revoke(slot as usize) {
            Ok(()) => {
                ksyscall_debug!("[syscall] cap_revoke: ✓ revoked slot {} and all descendants", slot);
                0
            }
            Err(e) => {
                ksyscall_debug!("[syscall] cap_revoke: ✗ failed to revoke slot {}: {:?}", slot, e);
                u64::MAX
            }
        }
    }
}

/// Derive a capability with reduced rights
///
/// Creates a child capability in the CDT with equal or reduced rights.
/// The child is tracked as a descendant and will be revoked when parent is revoked.
///
/// # Arguments
/// - cnode_cap: CNode capability slot
/// - src_slot: Source capability slot
/// - dest_slot: Destination slot (must be empty)
/// - new_rights: New rights for derived capability (must be <= source rights)
///
/// # Returns
/// 0 on success, u64::MAX on error
///
/// # Security
/// - Requires CAP_CAPS permission
/// - Requires WRITE rights on CNode capability
/// - new_rights must not exceed source capability rights (authority monotonicity)
fn sys_cap_derive(cnode_cap: u64, src_slot: u64, dest_slot: u64, new_rights: u64) -> u64 {
    use crate::objects::cnode_cdt::CNodeCdt;
    use crate::objects::{CapType, CapRights};

    ksyscall_debug!("[syscall] cap_derive: cnode={}, src={}, dest={}, rights={:#x}",
                  cnode_cap, src_slot, dest_slot, new_rights);

    unsafe {
        // Get current thread's CSpace
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            return u64::MAX;
        }

        // Check if caller has capability management capability
        if !(*current_tcb).has_capability(TCB::CAP_CAPS) {
            ksyscall_debug!("[syscall] cap_derive: caller lacks CAP_CAPS");
            return u64::MAX;
        }

        let cspace_root = (*current_tcb).cspace_root();
        if cspace_root.is_null() {
            return u64::MAX;
        }

        let caller_cspace = &*(cspace_root as *const CNodeCdt);

        // Special case: cnode_cap == 0 means "use caller's CSpace root directly"
        // This allows userspace to operate on its own CSpace without needing a capability to it
        let target_cnode = if cnode_cap == 0 {
            ksyscall_debug!("[syscall] cap_derive: using caller's own CSpace (cnode_cap=0)");
            &mut *(cspace_root as *mut CNodeCdt)
        } else {
            // Lookup CNode capability with validation
            let cnode_capability = match caller_cspace.lookup(cnode_cap as usize) {
                Some(cap) => cap,
                None => {
                    ksyscall_debug!("[syscall] cap_derive: CNode not found at slot {}", cnode_cap);
                    return u64::MAX;
                }
            };

            // Verify it's a CNode capability
            if cnode_capability.cap_type() != CapType::CNode {
                ksyscall_debug!("[syscall] cap_derive: slot {} is not a CNode", cnode_cap);
                return u64::MAX;
            }

            // Verify WRITE rights on the CNode
            if !cnode_capability.rights().contains(CapRights::WRITE) {
                ksyscall_debug!("[syscall] cap_derive: insufficient rights on CNode");
                return u64::MAX;
            }

            // Get target CNode
            &mut *(cnode_capability.object_ptr() as *mut CNodeCdt)
        };

        // Convert u64 to CapRights
        let rights = CapRights::from_bits(new_rights as u8);

        match target_cnode.derive(src_slot as usize, dest_slot as usize, rights) {
            Ok(()) => {
                ksyscall_debug!("[syscall] cap_derive: ✓ derived cap from slot {} to {}",
                              src_slot, dest_slot);
                0
            }
            Err(e) => {
                ksyscall_debug!("[syscall] cap_derive: ✗ failed: {:?}", e);
                u64::MAX
            }
        }
    }
}

/// Mint a badged endpoint capability
///
/// Creates a badged child capability for IPC endpoint identification.
/// The badge allows the receiver to identify which endpoint was used.
///
/// # Arguments
/// - cnode_cap: CNode capability slot
/// - src_slot: Source endpoint capability slot
/// - dest_slot: Destination slot (must be empty)
/// - badge: Badge value (non-zero)
///
/// # Returns
/// 0 on success, u64::MAX on error
///
/// # Security
/// - Requires CAP_CAPS permission
/// - Requires WRITE rights on CNode capability
/// - Source must be an endpoint capability
fn sys_cap_mint(cnode_cap: u64, src_slot: u64, dest_slot: u64, badge: u64) -> u64 {
    use crate::objects::cnode_cdt::CNodeCdt;
    use crate::objects::{CapType, CapRights};

    ksyscall_debug!("[syscall] cap_mint: cnode={}, src={}, dest={}, badge={:#x}",
                  cnode_cap, src_slot, dest_slot, badge);

    unsafe {
        // Get current thread's CSpace
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            return u64::MAX;
        }

        // Check if caller has capability management capability
        if !(*current_tcb).has_capability(TCB::CAP_CAPS) {
            ksyscall_debug!("[syscall] cap_mint: caller lacks CAP_CAPS");
            return u64::MAX;
        }

        let cspace_root = (*current_tcb).cspace_root();
        if cspace_root.is_null() {
            return u64::MAX;
        }

        let caller_cspace = &*(cspace_root as *const CNodeCdt);

        // Special case: cnode_cap == 0 means "use caller's CSpace root directly"
        let target_cnode = if cnode_cap == 0 {
            ksyscall_debug!("[syscall] cap_mint: using caller's own CSpace (cnode_cap=0)");
            &mut *(cspace_root as *mut CNodeCdt)
        } else {
            // Lookup CNode capability with validation
            let cnode_capability = match caller_cspace.lookup(cnode_cap as usize) {
                Some(cap) => cap,
                None => {
                    ksyscall_debug!("[syscall] cap_mint: CNode not found at slot {}", cnode_cap);
                    return u64::MAX;
                }
            };

            // Verify it's a CNode capability
            if cnode_capability.cap_type() != CapType::CNode {
                ksyscall_debug!("[syscall] cap_mint: slot {} is not a CNode", cnode_cap);
                return u64::MAX;
            }

            // Verify WRITE rights on the CNode
            if !cnode_capability.rights().contains(CapRights::WRITE) {
                ksyscall_debug!("[syscall] cap_mint: insufficient rights on CNode");
                return u64::MAX;
            }

            // Get target CNode
            &mut *(cnode_capability.object_ptr() as *mut CNodeCdt)
        };

        match target_cnode.mint(src_slot as usize, dest_slot as usize, badge) {
            Ok(()) => {
                ksyscall_debug!("[syscall] cap_mint: ✓ minted cap from slot {} to {} with badge {:#x}",
                              src_slot, dest_slot, badge);
                0
            }
            Err(e) => {
                ksyscall_debug!("[syscall] cap_mint: ✗ failed: {:?}", e);
                u64::MAX
            }
        }
    }
}

fn sys_cap_copy(src_cnode_cap: u64, src_slot: u64, dest_cnode_cap: u64, dest_slot: u64) -> u64 {
    use crate::objects::cnode_cdt::CNodeCdt;
    use crate::objects::{CapType, CapRights};

    ksyscall_debug!("[syscall] cap_copy: src_cnode={}, src_slot={}, dest_cnode={}, dest_slot={}",
                  src_cnode_cap, src_slot, dest_cnode_cap, dest_slot);

    unsafe {
        // Get current thread's CSpace
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            return u64::MAX;
        }

        // Check if caller has capability management capability
        if !(*current_tcb).has_capability(TCB::CAP_CAPS) {
            ksyscall_debug!("[syscall] cap_copy: caller lacks CAP_CAPS");
            return u64::MAX;
        }

        let cspace_root = (*current_tcb).cspace_root();
        if cspace_root.is_null() {
            return u64::MAX;
        }

        let caller_cspace = &*(cspace_root as *const CNodeCdt);

        // Get source CNode
        let src_cnode = if src_cnode_cap == 0 {
            ksyscall_debug!("[syscall] cap_copy: using caller's own CSpace as source");
            &mut *(cspace_root as *mut CNodeCdt)
        } else {
            let cnode_capability = match caller_cspace.lookup(src_cnode_cap as usize) {
                Some(cap) => cap,
                None => {
                    ksyscall_debug!("[syscall] cap_copy: source CNode not found");
                    return u64::MAX;
                }
            };

            if cnode_capability.cap_type() != CapType::CNode {
                ksyscall_debug!("[syscall] cap_copy: source is not a CNode");
                return u64::MAX;
            }

            if !cnode_capability.rights().contains(CapRights::READ) {
                ksyscall_debug!("[syscall] cap_copy: insufficient READ rights on source CNode");
                return u64::MAX;
            }

            &mut *(cnode_capability.object_ptr() as *mut CNodeCdt)
        };

        // Get destination CNode
        let dest_cnode = if dest_cnode_cap == 0 {
            ksyscall_debug!("[syscall] cap_copy: using caller's own CSpace as dest");
            &mut *(cspace_root as *mut CNodeCdt)
        } else if dest_cnode_cap == src_cnode_cap {
            // Same CNode for source and dest
            src_cnode
        } else {
            let cnode_capability = match caller_cspace.lookup(dest_cnode_cap as usize) {
                Some(cap) => cap,
                None => {
                    ksyscall_debug!("[syscall] cap_copy: dest CNode not found");
                    return u64::MAX;
                }
            };

            if cnode_capability.cap_type() != CapType::CNode {
                ksyscall_debug!("[syscall] cap_copy: dest is not a CNode");
                return u64::MAX;
            }

            if !cnode_capability.rights().contains(CapRights::WRITE) {
                ksyscall_debug!("[syscall] cap_copy: insufficient WRITE rights on dest CNode");
                return u64::MAX;
            }

            &mut *(cnode_capability.object_ptr() as *mut CNodeCdt)
        };

        match dest_cnode.copy(src_slot as usize, dest_slot as usize) {
            Ok(()) => {
                ksyscall_debug!("[syscall] cap_copy: ✓ copied cap from slot {} to {}",
                              src_slot, dest_slot);
                0
            }
            Err(e) => {
                ksyscall_debug!("[syscall] cap_copy: ✗ failed: {:?}", e);
                u64::MAX
            }
        }
    }
}

fn sys_cap_delete(cnode_cap: u64, slot: u64) -> u64 {
    use crate::objects::cnode_cdt::CNodeCdt;
    use crate::objects::{CapType, CapRights};

    ksyscall_debug!("[syscall] cap_delete: cnode={}, slot={}", cnode_cap, slot);

    unsafe {
        // Get current thread's CSpace
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            return u64::MAX;
        }

        // Check if caller has capability management capability
        if !(*current_tcb).has_capability(TCB::CAP_CAPS) {
            ksyscall_debug!("[syscall] cap_delete: caller lacks CAP_CAPS");
            return u64::MAX;
        }

        let cspace_root = (*current_tcb).cspace_root();
        if cspace_root.is_null() {
            return u64::MAX;
        }

        let caller_cspace = &*(cspace_root as *const CNodeCdt);

        // Get target CNode
        let target_cnode = if cnode_cap == 0 {
            ksyscall_debug!("[syscall] cap_delete: using caller's own CSpace");
            &mut *(cspace_root as *mut CNodeCdt)
        } else {
            let cnode_capability = match caller_cspace.lookup(cnode_cap as usize) {
                Some(cap) => cap,
                None => {
                    ksyscall_debug!("[syscall] cap_delete: CNode not found");
                    return u64::MAX;
                }
            };

            if cnode_capability.cap_type() != CapType::CNode {
                ksyscall_debug!("[syscall] cap_delete: not a CNode");
                return u64::MAX;
            }

            if !cnode_capability.rights().contains(CapRights::WRITE) {
                ksyscall_debug!("[syscall] cap_delete: insufficient rights");
                return u64::MAX;
            }

            &mut *(cnode_capability.object_ptr() as *mut CNodeCdt)
        };

        match target_cnode.delete(slot as usize) {
            Ok(()) => {
                ksyscall_debug!("[syscall] cap_delete: ✓ deleted cap at slot {}", slot);
                0
            }
            Err(e) => {
                ksyscall_debug!("[syscall] cap_delete: ✗ failed: {:?}", e);
                u64::MAX
            }
        }
    }
}

fn sys_cap_move(src_cnode_cap: u64, src_slot: u64, dest_cnode_cap: u64, dest_slot: u64) -> u64 {
    use crate::objects::cnode_cdt::CNodeCdt;
    use crate::objects::{CapType, CapRights};

    ksyscall_debug!("[syscall] cap_move: src_cnode={}, src_slot={}, dest_cnode={}, dest_slot={}",
                  src_cnode_cap, src_slot, dest_cnode_cap, dest_slot);

    unsafe {
        // Get current thread's CSpace
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            return u64::MAX;
        }

        // Check if caller has capability management capability
        if !(*current_tcb).has_capability(TCB::CAP_CAPS) {
            ksyscall_debug!("[syscall] cap_move: caller lacks CAP_CAPS");
            return u64::MAX;
        }

        let cspace_root = (*current_tcb).cspace_root();
        if cspace_root.is_null() {
            return u64::MAX;
        }

        let caller_cspace = &*(cspace_root as *const CNodeCdt);

        // For move, both source and dest must be the same CNode (or both caller's CSpace)
        // This simplifies the implementation and is consistent with seL4
        let target_cnode = if src_cnode_cap == 0 && dest_cnode_cap == 0 {
            ksyscall_debug!("[syscall] cap_move: using caller's own CSpace");
            &mut *(cspace_root as *mut CNodeCdt)
        } else if src_cnode_cap == dest_cnode_cap && src_cnode_cap != 0 {
            let cnode_capability = match caller_cspace.lookup(src_cnode_cap as usize) {
                Some(cap) => cap,
                None => {
                    ksyscall_debug!("[syscall] cap_move: CNode not found");
                    return u64::MAX;
                }
            };

            if cnode_capability.cap_type() != CapType::CNode {
                ksyscall_debug!("[syscall] cap_move: not a CNode");
                return u64::MAX;
            }

            if !cnode_capability.rights().contains(CapRights::WRITE) {
                ksyscall_debug!("[syscall] cap_move: insufficient rights");
                return u64::MAX;
            }

            &mut *(cnode_capability.object_ptr() as *mut CNodeCdt)
        } else {
            ksyscall_debug!("[syscall] cap_move: source and dest must be same CNode");
            return u64::MAX;
        };

        match target_cnode.move_cap(src_slot as usize, dest_slot as usize) {
            Ok(()) => {
                ksyscall_debug!("[syscall] cap_move: ✓ moved cap from slot {} to {}",
                              src_slot, dest_slot);
                0
            }
            Err(e) => {
                ksyscall_debug!("[syscall] cap_move: ✗ failed: {:?}", e);
                u64::MAX
            }
        }
    }
}

/// Insert capability into caller's own CSpace
///
/// Simpler variant of sys_cap_insert_into that operates on the caller's CSpace.
/// Used by root-task to register TCB capabilities of spawned children.
///
/// # Arguments
/// - cap_slot: Slot number in caller's CSpace
/// - cap_type: Type of capability (CapType as u64)
/// - object_ptr: Physical pointer to the capability object
///
/// # Returns
/// 0 on success, u64::MAX on error
fn sys_cap_insert_self(cap_slot: u64, cap_type: u64, object_ptr: u64) -> u64 {
    use crate::objects::{Capability, CapType};

    crate::kprintln!("[syscall] cap_insert_self: slot={}, type={}, obj={:#x}",
              cap_slot, cap_type, object_ptr);

    unsafe {
        // Get current thread's CSpace
        let current_tcb = crate::scheduler::current_thread();
        if current_tcb.is_null() {
            ksyscall_debug!("[syscall] cap_insert_self: no current thread");
            return u64::MAX;
        }

        // Check if caller has capability management capability
        if !(*current_tcb).has_capability(TCB::CAP_CAPS) {
            ksyscall_debug!("[syscall] cap_insert_self: caller lacks CAP_CAPS capability");
            return u64::MAX; // Permission denied
        }

        let cspace_root = (*current_tcb).cspace_root();
        if cspace_root.is_null() {
            ksyscall_debug!("[syscall] cap_insert_self: thread has no CSpace root");
            return u64::MAX;
        }

        // Convert cap_type from u64 to CapType enum
        let cap_type_enum = match cap_type {
            0 => CapType::Null,
            1 => CapType::UntypedMemory,
            2 => CapType::Endpoint,
            3 => CapType::Notification,
            4 => CapType::Tcb,
            5 => CapType::CNode,
            6 => CapType::VSpace,
            _ => {
                ksyscall_debug!("[syscall] cap_insert_self: invalid cap_type {}", cap_type);
                return u64::MAX;
            }
        };

        // Create the capability
        let cap = Capability::new(cap_type_enum, object_ptr as usize);

        // Insert into caller's own CSpace
        let cnode = &mut *cspace_root;

        // Debug: Check slot status and CNode size
        crate::kprintln!("[syscall] cap_insert_self: CNode has {} slots, inserting at slot {}",
                 cnode.num_slots(), cap_slot);
        crate::kprintln!("[syscall] cap_insert_self: CNode slots_paddr={:#x}, size_bits={}",
                 cnode.slots_paddr().as_usize(), cnode.size_bits());
        if cap_slot as usize >= cnode.num_slots() {
            crate::kprintln!("[syscall] cap_insert_self: slot {} is out of bounds!", cap_slot);
        } else if !cnode.is_empty(cap_slot as usize) {
            crate::kprintln!("[syscall] cap_insert_self: slot {} is already occupied", cap_slot);
        }

        match cnode.insert(cap_slot as usize, cap) {
            Ok(()) => {
                crate::kprintln!("[syscall] cap_insert_self: ✓ inserted {:?} cap at slot {}", cap_type_enum, cap_slot);
                0
            }
            Err(e) => {
                crate::kprintln!("[syscall] cap_insert_self: ✗ failed to insert: {:?}", e);
                u64::MAX
            }
        }
    }
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
    ksyscall_debug!("[syscall] IPC Send: endpoint={}, msg_ptr=0x{:x}, len={}",
        endpoint_cap_slot, message_ptr, message_len);

    // Validate message length (max 256 bytes)
    if message_len > 256 {
        ksyscall_debug!("[syscall] IPC Send -> error: message too large ({} bytes)", message_len);
        return u64::MAX;
    }

    // Validate endpoint capability slot
    if endpoint_cap_slot >= 4096 {
        ksyscall_debug!("[syscall] IPC Send -> error: invalid endpoint cap slot {}", endpoint_cap_slot);
        return u64::MAX;
    }

    unsafe {
        // Get current thread
        let current = crate::scheduler::current_thread();
        if current.is_null() {
            ksyscall_debug!("[syscall] IPC Send -> error: no current thread");
            return u64::MAX;
        }

        // Look up endpoint from capability slot
        let endpoint_ptr = lookup_endpoint_capability(endpoint_cap_slot as usize);
        if endpoint_ptr.is_null() {
            ksyscall_debug!("[syscall] IPC Send -> error: endpoint not found for cap_slot {}", endpoint_cap_slot);
            return u64::MAX;
        }

        let endpoint = &mut *endpoint_ptr;

        // Copy message from userspace to kernel buffer
        let mut kernel_msg_buffer = [0u8; 256];
        if !copy_from_user(message_ptr, &mut kernel_msg_buffer, message_len as usize, tf.saved_ttbr0) {
            ksyscall_debug!("[syscall] IPC Send -> error: failed to copy message from userspace");
            return u64::MAX;
        }

        ksyscall_debug!("[syscall] IPC Send: copied {} bytes from userspace", message_len);

        // Check if there's a receiver waiting
        if let Some(receiver_tcb) = endpoint.dequeue_receiver() {
            ksyscall_debug!("[syscall] IPC Send: found waiting receiver, transferring message");

            // Copy message to receiver's IPC buffer
            let receiver = &mut *receiver_tcb;
            let receiver_context = receiver.context();
            let receiver_ttbr0 = receiver_context.saved_ttbr0;
            let receiver_ipc_buffer = receiver.ipc_buffer().as_u64();

            if !copy_to_user(&kernel_msg_buffer[..message_len as usize], receiver_ipc_buffer, message_len as usize, receiver_ttbr0) {
                ksyscall_debug!("[syscall] IPC Send -> error: failed to copy message to receiver");
                return u64::MAX;
            }

            // Store message length in receiver's x0 (return value)
            let receiver_ctx_mut = receiver.context_mut();
            receiver_ctx_mut.x0 = message_len;

            // Wake up receiver
            receiver.set_state(crate::objects::ThreadState::Runnable);
            crate::scheduler::enqueue(receiver_tcb);

            ksyscall_debug!("[syscall] IPC Send -> success, message delivered to receiver");
            return 0;
        }

        // No receiver waiting - block sender on endpoint's send queue
        ksyscall_debug!("[syscall] IPC Send: no receiver waiting, blocking sender");

        // Store message in sender's IPC buffer for later transfer
        let sender = &mut *current;
        let sender_ipc_buffer = sender.ipc_buffer().as_u64();
        if !copy_to_user(&kernel_msg_buffer[..message_len as usize], sender_ipc_buffer, message_len as usize, tf.saved_ttbr0) {
            ksyscall_debug!("[syscall] IPC Send -> error: failed to store message in sender's IPC buffer");
            return u64::MAX;
        }

        // Store message length in sender's context for later retrieval
        let sender_ctx_mut = sender.context_mut();
        sender_ctx_mut.x2 = message_len;

        // Block sender on endpoint
        endpoint.queue_send(current);

        // Context switch to next runnable thread
        crate::scheduler::yield_current();

        // When we return here, message has been delivered
        ksyscall_debug!("[syscall] IPC Send -> success after blocking");
        0
    }
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
    ksyscall_debug!("[syscall] IPC Recv: endpoint={}, buf_ptr=0x{:x}, len={}",
        endpoint_cap_slot, buffer_ptr, buffer_len);

    // Validate buffer length
    if buffer_len > 256 {
        ksyscall_debug!("[syscall] IPC Recv -> error: buffer too large ({} bytes)", buffer_len);
        return u64::MAX;
    }

    // Validate endpoint capability slot
    if endpoint_cap_slot >= 4096 {
        ksyscall_debug!("[syscall] IPC Recv -> error: invalid endpoint cap slot {}", endpoint_cap_slot);
        return u64::MAX;
    }

    unsafe {
        // Get current thread
        let current = crate::scheduler::current_thread();
        if current.is_null() {
            ksyscall_debug!("[syscall] IPC Recv -> error: no current thread");
            return u64::MAX;
        }

        // Look up endpoint from capability slot
        let endpoint_ptr = lookup_endpoint_capability(endpoint_cap_slot as usize);
        if endpoint_ptr.is_null() {
            ksyscall_debug!("[syscall] IPC Recv -> error: endpoint not found for cap_slot {}", endpoint_cap_slot);
            return u64::MAX;
        }

        let endpoint = &mut *endpoint_ptr;

        // Check if there's a sender waiting
        if let Some(sender_tcb) = endpoint.dequeue_sender() {
            ksyscall_debug!("[syscall] IPC Recv: found waiting sender, transferring message");

            let sender = &mut *sender_tcb;

            // Retrieve message length from sender's context (stored during send)
            let sender_context = sender.context();
            let message_len = sender_context.x2 as usize;

            if message_len > buffer_len as usize {
                ksyscall_debug!("[syscall] IPC Recv -> error: sender message ({} bytes) larger than buffer ({} bytes)",
                         message_len, buffer_len);
                return u64::MAX;
            }

            // Copy message from sender's IPC buffer to kernel buffer
            let mut kernel_msg_buffer = [0u8; 256];
            let sender_ttbr0 = sender_context.saved_ttbr0;
            let sender_ipc_buffer = sender.ipc_buffer().as_u64();

            if !copy_from_user(sender_ipc_buffer, &mut kernel_msg_buffer, message_len, sender_ttbr0) {
                ksyscall_debug!("[syscall] IPC Recv -> error: failed to copy message from sender's IPC buffer");
                return u64::MAX;
            }

            // Copy message to receiver's buffer
            if !copy_to_user(&kernel_msg_buffer[..message_len], buffer_ptr, message_len, tf.saved_ttbr0) {
                ksyscall_debug!("[syscall] IPC Recv -> error: failed to copy message to receiver's buffer");
                return u64::MAX;
            }

            // Wake up sender
            sender.set_state(crate::objects::ThreadState::Runnable);
            crate::scheduler::enqueue(sender_tcb);

            ksyscall_debug!("[syscall] IPC Recv -> success, received {} bytes from sender", message_len);
            return message_len as u64;
        }

        // No sender waiting - block receiver on endpoint's recv queue
        ksyscall_debug!("[syscall] IPC Recv: no sender waiting, blocking receiver");

        let receiver = &mut *current;

        // Store buffer info in receiver's context for later use
        let receiver_ctx_mut = receiver.context_mut();
        receiver_ctx_mut.x1 = buffer_ptr;
        receiver_ctx_mut.x2 = buffer_len;

        // Block receiver on endpoint
        endpoint.queue_receive(current);

        // Context switch to next runnable thread
        crate::scheduler::yield_current();

        // When we return here, message has been received
        // The message length is stored in x0 by the sender
        let final_context = (*current).context();
        let bytes_received = final_context.x0;
        ksyscall_debug!("[syscall] IPC Recv -> success after blocking, received {} bytes", bytes_received);
        bytes_received
    }
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
    ksyscall_debug!("[syscall] IPC Call: endpoint={}, req_ptr=0x{:x}, req_len={}, rep_ptr=0x{:x}, rep_len={}",
        endpoint_cap_slot, request_ptr, request_len, reply_ptr, reply_len);

    // TODO: Full implementation
    // 1. Validate endpoint_cap_slot
    // 2. Get current TCB
    // 3. Copy request from userspace
    // 4. Call ipc::call(endpoint, tcb, request_message)
    // 5. Handle blocking/context switch
    // 6. Copy reply to userspace

    // For Phase 2, return 0 bytes to test the syscall path
    ksyscall_debug!("[syscall] IPC Call -> success (stub, 0 bytes)");
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
    ksyscall_debug!("[syscall] IPC Reply: reply_cap={}, msg_ptr=0x{:x}",
        reply_cap_slot, message_ptr);

    // TODO: Full implementation
    // 1. Validate reply_cap_slot
    // 2. Get current TCB
    // 3. Copy reply message from userspace
    // 4. Call ipc::reply(reply_cap, message)
    // 5. Wake up caller

    // For Phase 2, return success to test the syscall path
    ksyscall_debug!("[syscall] IPC Reply -> success (stub)");
    0
}

// ============================================================================
// Chapter 9 Phase 2: Notification Syscalls (Shared Memory IPC)
// ============================================================================

/// Create a notification object
///
/// Returns: notification capability slot, or u64::MAX on error
fn sys_notification_create() -> u64 {
    use crate::objects::Notification;
    use crate::memory::alloc_frame;
    use core::ptr;

    // Allocate a physical frame for the Notification object
    let notification_frame = match alloc_frame() {
        Some(pfn) => pfn,
        None => {
            ksyscall_debug!("[syscall] notification_create: out of memory");
            return u64::MAX;
        }
    };

    let notification_phys = notification_frame.phys_addr();

    // Create the Notification object
    let notification_ptr = notification_phys.as_u64() as *mut Notification;
    unsafe {
        ptr::write(notification_ptr, Notification::new());
    }

    // Allocate capability slot for the notification
    let slot = sys_cap_allocate();

    // Insert notification capability into current thread's CSpace
    unsafe {
        if !insert_notification_capability(slot as usize, notification_ptr) {
            ksyscall_debug!("[syscall] notification_create: failed to insert capability");
            return u64::MAX;
        }
    }

    ksyscall_debug!("[syscall] notification_create: SUCCESS -> cap_slot={}", slot);
    slot
}

/// Insert a notification capability into the current thread's CSpace
unsafe fn insert_notification_capability(cap_slot: usize, notification: *mut Notification) -> bool {
    use crate::objects::{Capability, CapType};

    // Get current thread's CSpace root
    let current_tcb = crate::scheduler::current_thread();
    if current_tcb.is_null() {
        ksyscall_debug!("[syscall] insert_notification: no current thread");
        return false;
    }

    let cspace_root = (*current_tcb).cspace_root();
    if cspace_root.is_null() {
        ksyscall_debug!("[syscall] insert_notification: thread has no CSpace root");
        return false;
    }

    // Create Notification capability
    let cap = Capability::new(CapType::Notification, notification as usize);

    // Insert into CSpace
    let cnode = &mut *cspace_root;
    match cnode.insert(cap_slot, cap) {
        Ok(()) => {
            ksyscall_debug!("[syscall] insert_notification: cap_slot {} -> notification {:p}", cap_slot, notification);
            true
        }
        Err(e) => {
            ksyscall_debug!("[syscall] insert_notification: failed to insert at cap_slot {}: {:?}", cap_slot, e);
            false
        }
    }
}

/// Look up a notification capability from the current thread's CSpace
unsafe fn lookup_notification_capability(cap_slot: usize) -> *mut Notification {
    use crate::objects::{CapType, Capability, Notification};

    // Get current thread's CSpace root
    let current_tcb = crate::scheduler::current_thread();
    if current_tcb.is_null() {
        ksyscall_debug!("[syscall] lookup_notification: no current thread");
        return ptr::null_mut();
    }

    let cspace_root = (*current_tcb).cspace_root();
    if cspace_root.is_null() {
        ksyscall_debug!("[syscall] lookup_notification: thread has no CSpace root");
        return ptr::null_mut();
    }

    // Look up capability in CSpace
    let cnode = &*cspace_root;
    let cap = match cnode.lookup(cap_slot) {
        Some(c) => c,
        None => {
            ksyscall_debug!("[syscall] lookup_notification: cap_slot {} not found in CSpace", cap_slot);
            return ptr::null_mut();
        }
    };

    // Verify it's a Notification capability
    if cap.cap_type() != CapType::Notification {
        ksyscall_debug!("[syscall] lookup_notification: cap_slot {} is not a Notification (type={:?})",
                 cap_slot, cap.cap_type());
        return ptr::null_mut();
    }

    // Return pointer to Notification object
    cap.object_ptr() as *mut Notification
}

/// Signal a notification (non-blocking)
///
/// Args:
/// - notification_cap_slot: Capability slot for notification
/// - badge: Signal bits to set (OR'd with existing signals)
///
/// Returns: 0 on success, u64::MAX on error
fn sys_signal(notification_cap_slot: u64, badge: u64) -> u64 {
    crate::kprintln!("[syscall] sys_signal called: slot={}, badge=0x{:x}", notification_cap_slot, badge);

    unsafe {
        // Look up notification from capability slot
        let notification_ptr = lookup_notification_capability(notification_cap_slot as usize);
        if notification_ptr.is_null() {
            crate::kprintln!("[syscall] sys_signal: ERROR - notification not found for slot {}", notification_cap_slot);
            return u64::MAX;
        }

        let notification = &mut *notification_ptr;

        // Signal the notification
        notification.signal(badge);

        crate::kprintln!("[syscall] sys_signal: SUCCESS - returning to userspace");
        0
    }
}

/// Wait for notification (blocking)
///
/// Args:
/// - notification_cap_slot: Capability slot for notification
///
/// Returns: signal bits (non-zero), or u64::MAX on error
fn sys_wait(tf: &mut TrapFrame, notification_cap_slot: u64) -> u64 {
    crate::kprintln!("[syscall] sys_wait called: notification_cap_slot={}", notification_cap_slot);
    ksyscall_debug!("[syscall] Wait: notification={}", notification_cap_slot);

    unsafe {
        // Get current thread
        let current = crate::scheduler::current_thread();
        if current.is_null() {
            crate::kprintln!("[syscall] sys_wait: ERROR - no current thread");
            ksyscall_debug!("[syscall] Wait -> error: no current thread");
            return u64::MAX;
        }

        // Save current thread's context BEFORE potentially blocking
        // This is critical - if we block, we need the context saved for when we resume
        *(*current).context_mut() = *tf;

        // Debug: verify saved context (commented out - too verbose)
        // crate::kprintln!("[syscall] sys_wait: saved context for TCB={:#x}, ELR={:#x}, SP={:#x}",
        //                 current as usize, tf.elr_el1, tf.sp_el0);

        // Look up notification from capability slot
        let notification_ptr = lookup_notification_capability(notification_cap_slot as usize);
        if notification_ptr.is_null() {
            ksyscall_debug!("[syscall] Wait -> error: notification not found for cap_slot {}", notification_cap_slot);
            return u64::MAX;
        }

        let notification = &mut *notification_ptr;

        // Wait for notification (blocks if no signals pending)
        match notification.wait(current) {
            Some(signals) => {
                // Signals were already pending, return immediately
                if signals != 0 {
                    crate::kprintln!("[syscall] sys_wait: signals pending 0x{:x}, returning immediately", signals);
                } else {
                    crate::kprintln!("[syscall] sys_wait: WARNING - notification.wait() returned Some(0)!");
                }
                ksyscall_debug!("[syscall] Wait -> received signals 0x{:x}", signals);
                signals
            }
            None => {
                // No signals pending - thread has been blocked
                // Now we need to schedule the next thread
                crate::kprintln!("[syscall] sys_wait: no signals, blocking thread TCB={:#x}", current as usize);
                let next = crate::scheduler::schedule();
                if next.is_null() || next == current {
                    // No other thread available - this shouldn't happen if we blocked
                    crate::kprintln!("[syscall] sys_wait: ERROR - blocked but no other thread available!");
                    ksyscall_debug!("[syscall] Wait -> blocked but no other thread!");
                    return u64::MAX;
                }

                // Switch to next thread
                let next_tcb = &mut *next;
                next_tcb.set_state(crate::objects::ThreadState::Running);
                crate::scheduler::test_set_current_thread(next);

                crate::kprintln!("[syscall] sys_wait: switching to TCB={:#x}, ELR={:#x}, TTBR0={:#x}",
                                next as usize, next_tcb.context().elr_el1, next_tcb.context().saved_ttbr0);

                // Replace our TrapFrame with the next thread's context
                // When we return from this syscall, the exception handler will restore
                // the next thread's context and eret to it
                *tf = *next_tcb.context();

                // Return 0 - but this won't be seen by current thread
                // When this thread is signaled and resumed, it will return with
                // the signal value stored in its context's x0
                0
            }
        }
    }
}

/// Poll notification (non-blocking)
///
/// Args:
/// - notification_cap_slot: Capability slot for notification
///
/// Returns: signal bits (0 if no signals), or u64::MAX on error
fn sys_poll(notification_cap_slot: u64) -> u64 {
    ksyscall_debug!("[syscall] Poll: notification={}", notification_cap_slot);

    unsafe {
        // Look up notification from capability slot
        let notification_ptr = lookup_notification_capability(notification_cap_slot as usize);
        if notification_ptr.is_null() {
            ksyscall_debug!("[syscall] Poll -> error: notification not found for cap_slot {}", notification_cap_slot);
            return u64::MAX;
        }

        let notification = &*notification_ptr;

        // Poll for signals (non-blocking)
        let signals = notification.poll();

        ksyscall_debug!("[syscall] Poll -> signals 0x{:x}", signals);
        signals
    }
}

/// Register shared memory with the kernel registry
/// Args: name_ptr, name_len, phys_addr, size
/// Returns: 0 on success, u64::MAX on error
fn sys_shmem_register(tf: &TrapFrame, name_ptr: u64, name_len: u64, phys_addr: u64, size: u64) -> u64 {
    use core::cmp::min;

    if name_len == 0 || name_len > 32 {
        kprintln!("[syscall] shmem_register: invalid name length {}", name_len);
        return u64::MAX;
    }

    // Copy channel name from userspace
    let mut name_buf = [0u8; 32];
    if !unsafe { copy_from_user(name_ptr, &mut name_buf[..name_len as usize], name_len as usize, tf.saved_ttbr0) } {
        kprintln!("[syscall] shmem_register: failed to copy name from userspace");
        return u64::MAX;
    }

    // Find free slot in registry
    unsafe {
        for entry in SHMEM_REGISTRY.iter_mut() {
            if !entry.valid {
                // Use this slot
                let len = min(name_len as usize, 32);
                entry.name[..len].copy_from_slice(&name_buf[..len]);
                entry.name_len = len;
                entry.phys_addr = phys_addr as usize;
                entry.size = size as usize;
                entry.valid = true;

                kprintln!("[syscall] shmem_register: registered '{}' at phys={:#x}, size={:#x}",
                         core::str::from_utf8(&name_buf[..len]).unwrap_or("<invalid>"),
                         phys_addr, size);
                return 0;
            }
        }
    }

    kprintln!("[syscall] shmem_register: registry full");
    u64::MAX
}

/// Query shared memory from the kernel registry
/// Args: name_ptr, name_len
/// Returns: (phys_addr << 32) | size on success, 0 if not found
fn sys_shmem_query(tf: &TrapFrame, name_ptr: u64, name_len: u64) -> u64 {
    if name_len == 0 || name_len > 32 {
        return 0;
    }

    // Copy channel name from userspace
    let mut name_buf = [0u8; 32];
    if !unsafe { copy_from_user(name_ptr, &mut name_buf[..name_len as usize], name_len as usize, tf.saved_ttbr0) } {
        kprintln!("[syscall] shmem_query: failed to copy name from userspace");
        return 0;
    }

    // Search for matching entry
    unsafe {
        for entry in SHMEM_REGISTRY.iter() {
            if entry.valid && entry.name_len == name_len as usize
                && entry.name[..entry.name_len] == name_buf[..name_len as usize] {
                    // Found it - return phys_addr in lower 32 bits, size in upper 32 bits
                    // Actually, let's just return phys_addr and use a separate syscall for size
                    kprintln!("[syscall] shmem_query: found '{}' at phys={:#x}, size={:#x}",
                             core::str::from_utf8(&name_buf[..name_len as usize]).unwrap_or("<invalid>"),
                             entry.phys_addr, entry.size);
                    return entry.phys_addr as u64;
                }
        }
    }

    kprintln!("[syscall] shmem_query: not found '{}'",
             core::str::from_utf8(&name_buf[..name_len as usize]).unwrap_or("<invalid>"));
    0
}

// ==============================================================================
// IRQ Handling Syscalls
// ==============================================================================

/// IRQControl_Get - Allocate an IRQ handler capability
///
/// This syscall allows a process with an IRQControl capability to allocate
/// an IRQHandler for a specific hardware interrupt and bind it to a notification.
///
/// # Arguments
/// * `irq_control_cap` - Capability slot containing IRQControl capability
/// * `irq_num` - Hardware IRQ number to allocate (e.g., 27 for timer, 33 for UART0)
/// * `notification_cap` - Capability slot containing notification to signal on IRQ
/// * `irq_handler_slot` - Empty capability slot to store the new IRQHandler
///
/// # Returns
/// - 0 on success
/// - u64::MAX on error (invalid capability, IRQ already allocated, etc.)
///
/// # Security
/// - Requires IRQControl capability (only root-task has this by default)
/// - Only one IRQHandler can exist per IRQ number
/// - IRQHandler is bound to the specific notification
///
/// # Usage
/// ```ignore
/// let notification_cap = sys_notification_create();
/// let irq_handler_slot = sys_cap_allocate();
/// sys_irq_handler_get(irq_control_cap, IRQ_UART0, notification_cap, irq_handler_slot);
/// ```
fn sys_irq_handler_get(tf: &TrapFrame, irq_control_cap: u64, irq_num: u64, notification_cap: u64, irq_handler_slot: u64) -> u64 {
    ksyscall_debug!("[syscall] sys_irq_handler_get: irq_control={}, irq={}, notif={}, slot={}",
                   irq_control_cap, irq_num, notification_cap, irq_handler_slot);

    unsafe {
        let current = crate::scheduler::current_thread();
        if current.is_null() {
            kprintln!("[syscall] sys_irq_handler_get: no current thread");
            return u64::MAX;
        }

        let tcb = &*current;
        let cspace_root = tcb.cspace_root();
        if cspace_root.is_null() {
            kprintln!("[syscall] sys_irq_handler_get: no CSpace");
            return u64::MAX;
        }

        // Look up IRQControl capability
        let cap = match (*cspace_root).lookup(irq_control_cap as usize) {
            Some(c) => c,
            None => {
                kprintln!("[syscall] sys_irq_handler_get: IRQControl cap not found");
                return u64::MAX;
            }
        };

        if cap.cap_type() != crate::objects::CapType::IrqControl {
            kprintln!("[syscall] sys_irq_handler_get: not an IRQControl capability");
            return u64::MAX;
        }

        // Look up notification capability
        let notif_cap = match (*cspace_root).lookup(notification_cap as usize) {
            Some(c) => c,
            None => {
                kprintln!("[syscall] sys_irq_handler_get: notification cap not found");
                return u64::MAX;
            }
        };

        if notif_cap.cap_type() != crate::objects::CapType::Notification {
            kprintln!("[syscall] sys_irq_handler_get: not a Notification capability");
            return u64::MAX;
        }

        let notification_ptr = notif_cap.object_ptr() as *mut crate::objects::Notification;
        if notification_ptr.is_null() {
            kprintln!("[syscall] sys_irq_handler_get: invalid notification pointer");
            return u64::MAX;
        }

        // Validate IRQ number
        if irq_num >= crate::arch::aarch64::gic::MAX_IRQS as u64 {
            kprintln!("[syscall] sys_irq_handler_get: invalid IRQ number {}", irq_num);
            return u64::MAX;
        }

        // Check if IRQ is already allocated
        if crate::objects::irq_handler::get_irq_handler(irq_num as u32).is_some() {
            kprintln!("[syscall] sys_irq_handler_get: IRQ {} already allocated", irq_num);
            return u64::MAX;
        }

        // Allocate physical frame for IRQHandler object
        let handler_frame = crate::memory::alloc_frame();
        if handler_frame.is_none() {
            kprintln!("[syscall] sys_irq_handler_get: failed to allocate IRQHandler frame");
            return u64::MAX;
        }

        let handler_phys = handler_frame.unwrap().phys_addr();
        let handler_ptr = handler_phys.as_usize() as *mut crate::objects::IRQHandler;

        // Initialize IRQHandler
        let handler = crate::objects::IRQHandler::new(irq_num as u32, notification_ptr);
        core::ptr::write(handler_ptr, handler);

        // Register handler in global table
        if crate::objects::irq_handler::register_irq_handler(irq_num as u32, handler_ptr).is_err() {
            kprintln!("[syscall] sys_irq_handler_get: failed to register IRQ handler");
            // TODO: Deallocate frame
            return u64::MAX;
        }

        // Enable the IRQ in the GIC
        crate::arch::aarch64::gic::enable_irq(irq_num as u32);

        // Create capability for IRQHandler
        let irq_handler_cap = crate::objects::Capability::new(
            crate::objects::CapType::IrqHandler,
            handler_ptr as usize,
        );

        // Insert capability into caller's CSpace
        if (*cspace_root).insert(irq_handler_slot as usize, irq_handler_cap).is_err() {
            kprintln!("[syscall] sys_irq_handler_get: failed to insert capability");
            crate::objects::irq_handler::unregister_irq_handler(irq_num as u32);
            // TODO: Deallocate frame
            return u64::MAX;
        }

        kprintln!("[syscall] sys_irq_handler_get: ✓ allocated IRQ {} handler at {:#x}",
                 irq_num, handler_ptr as usize);

        0 // Success
    }
}

/// IRQHandler_Ack - Acknowledge IRQ and re-enable it
///
/// This syscall must be called by a device driver after it has serviced an interrupt.
/// It re-enables the IRQ at the GIC, allowing future interrupts to be delivered.
///
/// # Arguments
/// * `irq_handler_cap` - Capability slot containing IRQHandler capability
///
/// # Returns
/// - 0 on success
/// - u64::MAX on error (invalid capability, etc.)
///
/// # Security
/// - Requires IRQHandler capability for the specific IRQ
/// - Only the holder of the IRQHandler can acknowledge the IRQ
///
/// # IRQ Flow
/// 1. IRQ fires → kernel ACKs at GIC + signals notification + masks IRQ
/// 2. Driver wakes up from sys_wait(notification)
/// 3. Driver services the device
/// 4. Driver calls sys_irq_handler_ack() → kernel unmasks IRQ
/// 5. IRQ can fire again
///
/// # Usage
/// ```ignore
/// loop {
///     sys_wait(notification_cap); // Wait for IRQ
///     // Service device here
///     uart_handle_interrupt();
///     sys_irq_handler_ack(irq_handler_cap); // Re-enable IRQ
/// }
/// ```
fn sys_irq_handler_ack(tf: &TrapFrame, irq_handler_cap: u64) -> u64 {
    ksyscall_debug!("[syscall] sys_irq_handler_ack: cap={}", irq_handler_cap);

    unsafe {
        let current = crate::scheduler::current_thread();
        if current.is_null() {
            return u64::MAX;
        }

        let tcb = &*current;
        let cspace_root = tcb.cspace_root();
        if cspace_root.is_null() {
            return u64::MAX;
        }

        // Look up IRQHandler capability
        let cap = match (*cspace_root).lookup(irq_handler_cap as usize) {
            Some(c) => c,
            None => {
                kprintln!("[syscall] sys_irq_handler_ack: capability not found");
                return u64::MAX;
            }
        };

        if cap.cap_type() != crate::objects::CapType::IrqHandler {
            kprintln!("[syscall] sys_irq_handler_ack: not an IRQHandler capability");
            return u64::MAX;
        }

        let handler_ptr = cap.object_ptr() as *mut crate::objects::IRQHandler;
        if handler_ptr.is_null() {
            kprintln!("[syscall] sys_irq_handler_ack: invalid handler pointer");
            return u64::MAX;
        }

        // Acknowledge IRQ (re-enables it at GIC)
        let handler = &mut *handler_ptr;
        handler.ack();

        ksyscall_debug!("[syscall] sys_irq_handler_ack: ✓ IRQ {} re-enabled", handler.irq_num());

        0 // Success
    }
}

// ============================================================================
// System Control Syscalls
// ============================================================================

/// Shutdown the system
///
/// This syscall gracefully shuts down the system by issuing a PSCI SYSTEM_OFF call.
/// On QEMU, this cleanly exits the emulator. On real hardware, this powers off the system.
///
/// Returns: Does not return
fn sys_shutdown() -> ! {
    crate::kprintln!("\n[kernel] System shutdown requested");
    crate::kprintln!("[kernel] Powering off...\n");

    // ARM PSCI (Power State Coordination Interface) SYSTEM_OFF
    // Function ID: 0x84000008
    unsafe {
        core::arch::asm!(
            "movz x0, #0x0008",      // Lower 16 bits
            "movk x0, #0x8400, lsl #16",  // Upper 16 bits
            "hvc #0",                 // Hypervisor call
            options(noreturn)
        );
    }
}
