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
        numbers::SYS_MEMORY_MAP => sys_memory_map(args[0], args[1], args[2]),
        numbers::SYS_MEMORY_UNMAP => sys_memory_unmap(args[0], args[1]),

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
/// For multi-page allocations, we allocate contiguous frames if possible.
fn sys_memory_allocate(size: u64) -> u64 {
    use crate::memory::{alloc_frame, PAGE_SIZE};

    let page_size = PAGE_SIZE as u64;
    let pages_needed = ((size + page_size - 1) / page_size) as usize;

    kprintln!("[syscall] memory_allocate: {} bytes ({} pages)", size, pages_needed);

    // Allocate the first frame
    let first_pfn = match alloc_frame() {
        Some(pfn) => pfn,
        None => {
            kprintln!("[syscall] memory_allocate: out of memory");
            return u64::MAX;
        }
    };

    let base_addr = first_pfn.phys_addr();

    // For multi-page allocations, allocate additional contiguous frames
    // Note: This is a simplified approach. A production system would use
    // a buddy allocator or similar for contiguous allocation guarantees.
    if pages_needed > 1 {
        for i in 1..pages_needed {
            match alloc_frame() {
                Some(_pfn) => {
                    // In a real system, verify contiguous allocation
                    // For now, we trust the frame allocator
                }
                None => {
                    kprintln!("[syscall] memory_allocate: partial allocation failed at page {}/{}", i, pages_needed);
                    // TODO: Free previously allocated frames
                    return u64::MAX;
                }
            }
        }
    }

    kprintln!("[syscall] memory_allocate -> 0x{:x} ({} pages)", base_addr.as_u64(), pages_needed);
    base_addr.as_u64()
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
static mut NEXT_VIRT_ADDR: u64 = 0x80000000; // Start at 2GB

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
fn sys_memory_map(phys_addr: u64, size: u64, permissions: u64) -> u64 {
    use crate::memory::{PAGE_SIZE, VirtAddr, PhysAddr, PageSize};
    use crate::arch::aarch64::page_table::{PageTable, PageTableFlags};

    kprintln!("[syscall] memory_map: phys={:#x}, size={}, perms={:#x}", phys_addr, size, permissions);

    // Round size up to page boundary
    let page_size = PAGE_SIZE as u64;
    let num_pages = ((size + page_size - 1) / page_size) as usize;
    let aligned_size = num_pages as u64 * page_size;

    // Get current page table from TTBR0_EL1
    let ttbr0: u64;
    unsafe {
        core::arch::asm!("mrs {}, ttbr0_el1", out(reg) ttbr0);
    }
    let page_table_phys = ttbr0 as usize;
    kprintln!("[syscall] memory_map: caller's TTBR0={:#x}", page_table_phys);

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

    // Set up page table flags for userspace read-write data
    // Build flags explicitly to debug permission issue
    let flags = PageTableFlags::VALID
              | PageTableFlags::TABLE_OR_PAGE
              | PageTableFlags::AP_RW_ALL
              | PageTableFlags::ACCESSED
              | PageTableFlags::INNER_SHARE
              | PageTableFlags::NORMAL
              | PageTableFlags::UXN;

    kprintln!("[syscall] memory_map: using flags = {:#x}", flags.bits());
    kprintln!("[syscall] memory_map: USER_DATA = {:#x}", PageTableFlags::USER_DATA.bits());

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
            "isb",        // Instruction synchronization barrier
        );
    }

    // Flush TLB for the entire ASID (simpler than per-page)
    unsafe {
        core::arch::asm!(
            "tlbi vmalle1is",  // Invalidate all EL1&0 stage 1 TLB entries
            "dsb ish",         // Ensure TLB invalidation completes
            "isb",             // Instruction synchronization barrier
        );
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
