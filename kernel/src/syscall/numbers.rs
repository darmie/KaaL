//! System call numbers
//!
//! Syscall numbering follows seL4 conventions where possible.
//! Debug syscalls are in the 0x1000+ range.

/// Debug: Print a single character to console
pub const SYS_DEBUG_PUTCHAR: u64 = 0x1000;

/// Debug: Print a string to console (ptr, len)
pub const SYS_DEBUG_PRINT: u64 = 0x1001;

/// Yield the CPU to the scheduler
pub const SYS_YIELD: u64 = 0x01;

/// Send a message on an IPC endpoint (not yet implemented)
pub const SYS_SEND: u64 = 0x02;

/// Receive a message on an IPC endpoint (not yet implemented)
pub const SYS_RECV: u64 = 0x03;

/// Call: Combined send + receive (not yet implemented)
pub const SYS_CALL: u64 = 0x04;

/// Reply: Reply to a call (not yet implemented)
pub const SYS_REPLY: u64 = 0x05;

// Capability Management Syscalls (Chapter 9)
// These syscalls provide the foundation for the capability broker

/// Allocate a capability slot
/// Returns: capability slot number, or -1 on error
pub const SYS_CAP_ALLOCATE: u64 = 0x10;

/// Allocate physical memory
/// Args: size (bytes)
/// Returns: physical address, or -1 on error
pub const SYS_MEMORY_ALLOCATE: u64 = 0x11;

/// Request device resources
/// Args: device_id
/// Returns: MMIO base address, or -1 on error
pub const SYS_DEVICE_REQUEST: u64 = 0x12;

/// Create IPC endpoint
/// Returns: endpoint capability slot, or -1 on error
pub const SYS_ENDPOINT_CREATE: u64 = 0x13;

/// Create a new process with full isolation
/// Args: entry_point, stack_pointer, page_table_root, cspace_root
/// Returns: process ID, or -1 on error
pub const SYS_PROCESS_CREATE: u64 = 0x14;

/// Map physical memory into caller's virtual address space
/// Args: physical_addr, size, permissions (read=1, write=2, exec=4)
/// Returns: virtual address, or -1 on error
///
/// This allows userspace to access allocated physical memory by mapping
/// it into a free region of its virtual address space.
pub const SYS_MEMORY_MAP: u64 = 0x15;

/// Unmap virtual memory from caller's address space
/// Args: virtual_addr, size
/// Returns: 0 on success, -1 on error
pub const SYS_MEMORY_UNMAP: u64 = 0x16;

// Notification Syscalls (Chapter 9 Phase 2)
// Lightweight signaling for shared memory IPC

/// Create a notification object
/// Returns: notification capability slot, or -1 on error
pub const SYS_NOTIFICATION_CREATE: u64 = 0x17;

/// Signal a notification (non-blocking)
/// Args: notification_cap_slot, badge (signal bits)
/// Returns: 0 on success, -1 on error
pub const SYS_SIGNAL: u64 = 0x18;

/// Wait for notification (blocking)
/// Args: notification_cap_slot
/// Returns: signal bits (non-zero), or -1 on error
pub const SYS_WAIT: u64 = 0x19;

/// Poll notification (non-blocking)
/// Args: notification_cap_slot
/// Returns: signal bits (0 if no signals), or -1 on error
pub const SYS_POLL: u64 = 0x1A;

/// Map physical memory into target process's virtual address space (Phase 5)
/// Args: target_tcb_cap, phys_addr, size, virt_addr, permissions (read=1, write=2, exec=4)
/// Returns: 0 on success, -1 on error
///
/// Maps physical memory at a specific virtual address in target process.
/// The caller specifies the target virt_addr to avoid hardcoded addresses.
/// Requires TCB capability for the target process.
pub const SYS_MEMORY_MAP_INTO: u64 = 0x1B;

/// Insert capability into target process's CSpace (Phase 5)
/// Args: target_tcb_cap, cap_slot, cap_type, object_ptr
/// Returns: 0 on success, -1 on error
///
/// This allows one process (e.g., root-task) to grant capabilities to another
/// process by inserting them into the target's CSpace. Required for orchestrating
/// IPC by passing notification and TCB capabilities to spawned components.
pub const SYS_CAP_INSERT_INTO: u64 = 0x1C;

/// Insert capability into caller's own CSpace (Phase 6)
/// Args: cap_slot, cap_type, object_ptr
/// Returns: 0 on success, -1 on error
///
/// Simpler variant of SYS_CAP_INSERT_INTO that inserts into the caller's CSpace.
/// Used by root-task to register TCB capabilities of spawned children.
pub const SYS_CAP_INSERT_SELF: u64 = 0x1D;

// Channel management syscalls

/// Channel_Establish - Establish IPC channel between components
/// Args: target_pid, buffer_size, role (0=producer, 1=consumer)
/// Returns: packed channel config (vaddr|notify_slots), or 0 on error
pub const SYS_CHANNEL_ESTABLISH: u64 = 0x30;

/// Channel_Query - Query channel information
/// Args: channel_id
/// Returns: channel state/config, or 0 on error
pub const SYS_CHANNEL_QUERY: u64 = 0x31;

/// Channel_Close - Close an IPC channel
/// Args: channel_id
/// Returns: 1 on success, 0 on error
pub const SYS_CHANNEL_CLOSE: u64 = 0x32;

/// Register shared memory with broker (Producer)
/// Args: channel_name_ptr, channel_name_len, phys_addr, size
/// Returns: 0 on success, -1 on error
/// Allows producer to register allocated physical memory with the broker
pub const SYS_SHMEM_REGISTER: u64 = 0x33;

/// Query shared memory from broker (Consumer)
/// Args: channel_name_ptr, channel_name_len
/// Returns: physical address on success, 0 if not found, -1 on error
/// Allows consumer to discover the physical address allocated by producer
pub const SYS_SHMEM_QUERY: u64 = 0x34;

/// Register current process as root-task for yield (temporary)
/// Args: vspace_root (TTBR0 physical address)
/// Returns: 0 on success
/// TODO: Remove when proper scheduler integration complete
pub const SYS_REGISTER_ROOT: u64 = 0x1FFF;
