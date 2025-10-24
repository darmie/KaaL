//! System call wrappers
//!
//! Provides safe, ergonomic wrappers around raw KaaL syscalls.

use crate::{Result, Error};

/// Syscall numbers (re-exported for use in other modules)
pub mod numbers {
    pub const SYS_YIELD: usize = 0x01;
    pub const SYS_CAP_ALLOCATE: usize = 0x10;
    pub const SYS_MEMORY_ALLOCATE: usize = 0x11;
    pub const SYS_DEVICE_REQUEST: usize = 0x12;
    pub const SYS_ENDPOINT_CREATE: usize = 0x13;
    pub const SYS_PROCESS_CREATE: usize = 0x14;
    pub const SYS_MEMORY_MAP: usize = 0x15;
    pub const SYS_MEMORY_UNMAP: usize = 0x16;
    pub const SYS_NOTIFICATION_CREATE: usize = 0x17;
    pub const SYS_SIGNAL: usize = 0x18;
    pub const SYS_WAIT: usize = 0x19;
    pub const SYS_POLL: usize = 0x1A;

    // Channel management syscalls
    pub const SYS_CHANNEL_ESTABLISH: usize = 0x30;
    pub const SYS_CHANNEL_QUERY: usize = 0x31;
    pub const SYS_CHANNEL_CLOSE: usize = 0x32;

    pub const SYS_SHMEM_REGISTER: usize = 0x33;
    pub const SYS_SHMEM_QUERY: usize = 0x34;
    pub const SYS_SHMEM_GET_NOTIFICATION: usize = 0x35;

    // Privileged syscalls for root-task
    pub const SYS_MEMORY_MAP_INTO: usize = 0x1B;
    pub const SYS_CAP_INSERT_INTO: usize = 0x1C;
    pub const SYS_CAP_INSERT_SELF: usize = 0x1D;
    pub const SYS_CAP_REVOKE: usize = 0x1E;
    pub const SYS_CAP_DERIVE: usize = 0x1F;
    pub const SYS_CAP_MINT: usize = 0x20;
    pub const SYS_CAP_COPY: usize = 0x21;
    pub const SYS_CAP_DELETE: usize = 0x22;
    pub const SYS_CAP_MOVE: usize = 0x23;
    pub const SYS_MEMORY_REMAP: usize = 0x24;
    pub const SYS_MEMORY_SHARE: usize = 0x25;
    pub const SYS_RETYPE: usize = 0x26;

    // IRQ handling syscalls
    pub const SYS_IRQ_HANDLER_GET: usize = 0x40;
    pub const SYS_IRQ_HANDLER_ACK: usize = 0x41;

    // System control syscalls
    pub const SYS_SHUTDOWN: usize = 0x50;

    pub const SYS_DEBUG_PRINT: usize = 0x1001;
}

/// Print a message to the debug console
///
/// # Example
/// ```no_run
/// kaal_sdk::syscall::print("Hello, world!\n");
/// ```
pub fn print(msg: &str) {
    let msg_ptr = msg.as_ptr() as usize;
    let msg_len = msg.len();

    unsafe {
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_DEBUG_PRINT,
            inlateout("x0") msg_ptr => _,
            inlateout("x1") msg_len => _,
            lateout("x8") _,
        );
    }
}

/// Print formatted text to the debug console
///
/// # Example
/// ```no_run
/// use kaal_sdk::printf;
/// printf!("Hello {}\n", "world");
/// printf!("Value: {}\n", 42);
/// ```
#[macro_export]
macro_rules! printf {
    ($fmt:literal) => {
        $crate::syscall::print($fmt)
    };
    ($fmt:literal, $($arg:expr),* $(,)?) => {{
        use core::fmt::Write;

        static mut BUF: [u8; 512] = [0; 512];
        static mut LEN: usize = 0;

        struct Writer;
        impl Write for Writer {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                unsafe {
                    let bytes = s.as_bytes();
                    if LEN + bytes.len() > BUF.len() {
                        return Err(core::fmt::Error);
                    }
                    BUF[LEN..LEN + bytes.len()].copy_from_slice(bytes);
                    LEN += bytes.len();
                    Ok(())
                }
            }
        }

        unsafe {
            LEN = 0;
            let _ = core::write!(&mut Writer, $fmt, $($arg),*);
            $crate::syscall::print(core::str::from_utf8_unchecked(&BUF[..LEN]));
        }
    }};
}

/// Yield the current thread to the scheduler
///
/// # Example
/// ```no_run
/// kaal_sdk::syscall::yield_now();
/// ```
pub fn yield_now() {
    unsafe {
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_YIELD,
            out("x8") _,
            out("x0") _,
        );
    }
}

/// Allocate a capability slot
///
/// Returns the allocated slot number on success.
pub fn cap_allocate() -> Result<usize> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) numbers::SYS_CAP_ALLOCATE,
            result = out(reg) result,
            out("x8") _,
        );
        Error::from_syscall(result)
    }
}

/// Revoke capability and all its descendants (seL4-style CDT revocation)
///
/// Recursively deletes the capability at the specified slot along with all
/// capabilities derived from it. Implements seL4's capability revocation using
/// the CDT (Capability Derivation Tree).
///
/// # Arguments
/// * `cnode_cap` - Capability slot containing the CNode capability
/// * `slot` - Slot number within the CNode to revoke
///
/// # Returns
/// `Ok(())` on success
///
/// # Errors
/// * Permission denied if caller lacks CAP_CAPS capability
/// * Invalid capability if cnode_cap is not a valid CNode
/// * Insufficient rights if CNode doesn't have WRITE rights
/// * Not found if slot is invalid or empty
///
/// # Example
/// ```no_run
/// // Revoke capability at slot 5 in current CSpace (cnode_cap=0 means current)
/// kaal_sdk::syscall::cap_revoke(0, 5)?;
/// ```
///
/// # Security
/// Requires CAP_CAPS permission and WRITE rights on the target CNode.
pub fn cap_revoke(cnode_cap: usize, slot: usize) -> Result<()> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_CAP_REVOKE,
            inlateout("x0") cnode_cap => result,
            inlateout("x1") slot => _,
            lateout("x8") _,
        );
        Error::from_syscall(result).map(|_| ())
    }
}

/// Derive a capability with reduced rights
///
/// Creates a child capability in the CDT with equal or reduced rights.
/// The child is tracked as a descendant and will be revoked when parent is revoked.
///
/// # Arguments
/// * `cnode_cap` - Capability slot containing the CNode capability
/// * `src_slot` - Source capability slot
/// * `dest_slot` - Destination slot (must be empty)
/// * `new_rights` - New rights for derived capability (must be <= source rights)
///
/// # Returns
/// `Ok(())` on success
///
/// # Errors
/// * Permission denied if caller lacks CAP_CAPS capability
/// * Invalid capability if cnode_cap is not a valid CNode
/// * Insufficient rights if CNode doesn't have WRITE rights or new_rights > source rights
/// * Slot occupied if destination slot is not empty
///
/// # Example
/// ```no_run
/// use kaal_sdk::syscall::{cap_derive, CapRights};
///
/// // Derive a read-only capability from slot 3 to slot 5
/// cap_derive(0, 3, 5, CapRights::READ.bits())?;
/// ```
///
/// # Security
/// Requires CAP_CAPS permission, WRITE rights on CNode, and enforces authority monotonicity.
pub fn cap_derive(cnode_cap: usize, src_slot: usize, dest_slot: usize, new_rights: usize) -> Result<()> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_CAP_DERIVE,
            inlateout("x0") cnode_cap => result,
            inlateout("x1") src_slot => _,
            inlateout("x2") dest_slot => _,
            inlateout("x3") new_rights => _,
            lateout("x8") _,
        );
        Error::from_syscall(result).map(|_| ())
    }
}

/// Mint a badged endpoint capability
///
/// Creates a badged child capability for IPC endpoint identification.
/// The badge allows the receiver to identify which endpoint was used in IPC.
///
/// # Arguments
/// * `cnode_cap` - Capability slot containing the CNode capability
/// * `src_slot` - Source endpoint capability slot
/// * `dest_slot` - Destination slot (must be empty)
/// * `badge` - Badge value (non-zero, identifies the sender)
///
/// # Returns
/// `Ok(())` on success
///
/// # Errors
/// * Permission denied if caller lacks CAP_CAPS capability
/// * Invalid capability if cnode_cap is not a valid CNode or src is not an endpoint
/// * Insufficient rights if CNode doesn't have WRITE rights
/// * Slot occupied if destination slot is not empty
///
/// # Example
/// ```no_run
/// use kaal_sdk::syscall::cap_mint;
///
/// // Mint a badged endpoint from slot 2 to slot 6 with badge 0x1234
/// cap_mint(0, 2, 6, 0x1234)?;
/// ```
///
/// # Security
/// Requires CAP_CAPS permission and WRITE rights on CNode. Source must be an endpoint.
pub fn cap_mint(cnode_cap: usize, src_slot: usize, dest_slot: usize, badge: usize) -> Result<()> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_CAP_MINT,
            inlateout("x0") cnode_cap => result,
            inlateout("x1") src_slot => _,
            inlateout("x2") dest_slot => _,
            inlateout("x3") badge => _,
            lateout("x8") _,
        );
        Error::from_syscall(result).map(|_| ())
    }
}

/// Copy a capability to another slot
///
/// Creates an exact copy of a capability, preserving all rights and badges.
/// The copy shares the same parent in the CDT.
///
/// # Arguments
/// * `src_cnode_cap` - Source CNode capability slot (0 = caller's CSpace)
/// * `src_slot` - Source capability slot
/// * `dest_cnode_cap` - Destination CNode capability slot (0 = caller's CSpace)
/// * `dest_slot` - Destination slot (must be empty)
///
/// # Returns
/// `Ok(())` on success
///
/// # Errors
/// * Permission denied if caller lacks CAP_CAPS capability
/// * Invalid capability if CNode caps are invalid
/// * Insufficient rights if missing READ on source or WRITE on dest CNode
/// * Slot occupied if destination slot is not empty
///
/// # Security
/// Requires CAP_CAPS permission and appropriate rights on both CNodes.
pub fn cap_copy(src_cnode_cap: usize, src_slot: usize, dest_cnode_cap: usize, dest_slot: usize) -> Result<()> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_CAP_COPY,
            inlateout("x0") src_cnode_cap => result,
            inlateout("x1") src_slot => _,
            inlateout("x2") dest_cnode_cap => _,
            inlateout("x3") dest_slot => _,
            lateout("x8") _,
        );
        Error::from_syscall(result).map(|_| ())
    }
}

/// Delete a capability from a slot
///
/// Removes a capability from the specified slot without affecting descendants.
/// Unlike revoke, this only deletes the specific capability.
///
/// # Arguments
/// * `cnode_cap` - CNode capability slot (0 = caller's CSpace)
/// * `slot` - Capability slot to delete
///
/// # Returns
/// `Ok(())` on success
///
/// # Errors
/// * Permission denied if caller lacks CAP_CAPS capability
/// * Invalid capability if CNode cap is invalid
/// * Insufficient rights if missing WRITE on CNode
/// * Not found if slot is empty
///
/// # Security
/// Requires CAP_CAPS permission and WRITE rights on CNode.
pub fn cap_delete(cnode_cap: usize, slot: usize) -> Result<()> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_CAP_DELETE,
            inlateout("x0") cnode_cap => result,
            inlateout("x1") slot => _,
            lateout("x8") _,
        );
        Error::from_syscall(result).map(|_| ())
    }
}

/// Move a capability to another slot
///
/// Atomically moves a capability from source to destination slot within the same CNode.
/// The source slot becomes empty. This preserves the CDT relationship.
///
/// # Arguments
/// * `src_cnode_cap` - Source CNode capability slot (0 = caller's CSpace)
/// * `src_slot` - Source capability slot
/// * `dest_cnode_cap` - Destination CNode capability slot (must match source)
/// * `dest_slot` - Destination slot (must be empty)
///
/// # Returns
/// `Ok(())` on success
///
/// # Errors
/// * Permission denied if caller lacks CAP_CAPS capability
/// * Invalid capability if CNode cap is invalid
/// * Insufficient rights if missing WRITE on CNode
/// * Slot occupied if destination slot is not empty
/// * Invalid operation if source and dest CNodes don't match
///
/// # Security
/// Requires CAP_CAPS permission and WRITE rights on CNode.
/// Source and destination must be in the same CNode.
pub fn cap_move(src_cnode_cap: usize, src_slot: usize, dest_cnode_cap: usize, dest_slot: usize) -> Result<()> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_CAP_MOVE,
            inlateout("x0") src_cnode_cap => result,
            inlateout("x1") src_slot => _,
            inlateout("x2") dest_cnode_cap => _,
            inlateout("x3") dest_slot => _,
            lateout("x8") _,
        );
        Error::from_syscall(result).map(|_| ())
    }
}

/// Allocate physical memory
///
/// # Arguments
/// * `size` - Size in bytes to allocate
///
/// # Returns
/// Physical address of allocated memory on success.
pub fn memory_allocate(size: usize) -> Result<usize> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_MEMORY_ALLOCATE,
            inlateout("x0") size => result,
            lateout("x8") _,
        );
        Error::from_syscall(result)
    }
}

/// Retype untyped memory into a kernel object (capability-based allocation)
///
/// This is the PROPER way for userspace to create kernel objects using
/// delegated UntypedMemory capabilities. Unlike memory_allocate (which uses
/// kernel's frame allocator), this uses caller's own Untyped caps.
///
/// # Arguments
/// * `untyped_slot` - Capability slot containing UntypedMemory capability
/// * `object_type` - Type of object to create:
///   - 1 = UntypedMemory
///   - 2 = Endpoint
///   - 3 = Notification
///   - 4 = TCB
///   - 5 = CNode
///   - 6 = VSpace (page table root)
///   - 7 = PageTable
///   - 8 = Page
/// * `size_bits` - Object size as log2 (12 = 4KB, 20 = 1MB, etc.)
/// * `dest_cnode` - CNode to insert new capability (0 = caller's own CSpace)
/// * `dest_slot` - Slot number for the new capability
///
/// # Returns
/// Physical address of the newly created object on success.
///
/// # Example
/// ```no_run
/// // Retype Untyped at slot 5 into a 4KB TCB, place cap at slot 10
/// let tcb_paddr = sys_retype(5, 4, 12, 0, 10)?;
/// ```
pub fn sys_retype(
    untyped_slot: usize,
    object_type: usize,
    size_bits: usize,
    dest_cnode: usize,
    dest_slot: usize,
) -> Result<usize> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_RETYPE,
            inlateout("x0") untyped_slot => result,
            inlateout("x1") object_type => _,
            inlateout("x2") size_bits => _,
            inlateout("x3") dest_cnode => _,
            inlateout("x4") dest_slot => _,
            lateout("x8") _,
        );
        Error::from_syscall(result)
    }
}

/// Map physical memory into virtual address space
///
/// # Arguments
/// * `phys_addr` - Physical address to map
/// * `size` - Size in bytes
/// * `permissions` - Memory permissions (read=0x1, write=0x2, exec=0x4)
///
/// # Returns
/// Virtual address of mapped memory on success.
pub fn memory_map(phys_addr: usize, size: usize, permissions: usize) -> Result<usize> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_MEMORY_MAP,
            inlateout("x0") phys_addr => result,
            inlateout("x1") size => _,
            inlateout("x2") permissions => _,
            lateout("x8") _,
        );
        Error::from_syscall(result)
    }
}

/// Unmap virtual memory
///
/// # Arguments
/// * `virt_addr` - Virtual address to unmap
/// * `size` - Size in bytes
pub fn memory_unmap(virt_addr: usize, size: usize) -> Result<()> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_MEMORY_UNMAP,
            inlateout("x0") virt_addr => result,
            inlateout("x1") size => _,
            lateout("x8") _,
        );
        Error::from_syscall(result)?;
        Ok(())
    }
}

/// Change memory protection flags for existing mapping
///
/// Updates the protection flags of an already-mapped memory region.
/// Useful for implementing guard pages, write-protecting code sections,
/// or implementing copy-on-write semantics.
///
/// # Arguments
/// * `virt_addr` - Virtual address of mapped region (must be page-aligned)
/// * `size` - Size in bytes (must be page-aligned)
/// * `new_permissions` - New permission flags (read=0x1, write=0x2, exec=0x4)
///
/// # Returns
/// `Ok(())` on success
///
/// # Errors
/// * Permission denied if caller lacks CAP_MEMORY capability
/// * Invalid address if the region is not currently mapped
/// * Invalid argument if addresses are not page-aligned
///
/// # Example
/// ```no_run
/// use kaal_sdk::syscall::memory_remap;
///
/// // Make a code region read-only (no write, no exec)
/// memory_remap(code_addr, code_size, 0x1)?;
///
/// // Make a data region read-write (no exec)
/// memory_remap(data_addr, data_size, 0x3)?;
///
/// // Make a guard page inaccessible (no permissions)
/// memory_remap(guard_addr, 4096, 0x0)?;
/// ```
///
/// # Security
/// Requires CAP_MEMORY permission. The region must be already mapped in the caller's
/// address space. This performs translate→unmap→map with new flags, then flushes TLB.
pub fn memory_remap(virt_addr: usize, size: usize, new_permissions: usize) -> Result<()> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_MEMORY_REMAP,
            inlateout("x0") virt_addr => result,
            inlateout("x1") size => _,
            inlateout("x2") new_permissions => _,
            lateout("x8") _,
        );
        Error::from_syscall(result)?;
        Ok(())
    }
}

/// Share memory between processes (zero-copy IPC)
///
/// Maps physical pages from the caller's address space into another process's
/// address space at a specified virtual address. This enables zero-copy shared
/// memory IPC between processes.
///
/// # Arguments
/// * `target_tcb_cap` - TCB capability slot for the target process
/// * `source_virt_addr` - Source virtual address in caller's address space
/// * `size` - Size in bytes (must be page-aligned)
/// * `dest_virt_addr` - Destination virtual address in target's address space
/// * `permissions` - Permission flags for target mapping (read=0x1, write=0x2, exec=0x4)
///
/// # Returns
/// `Ok(())` on success
///
/// # Errors
/// * Permission denied if caller lacks CAP_MEMORY capability
/// * Invalid capability if target_tcb_cap is not a valid TCB capability
/// * Invalid address if source region is not mapped or dest region conflicts
/// * Invalid argument if addresses/size are not page-aligned
///
/// # Example
/// ```no_run
/// use kaal_sdk::syscall::memory_share;
///
/// // Share a read-write buffer with another process
/// let shared_buffer_vaddr = 0x100000;
/// let target_buffer_vaddr = 0x200000;
/// let buffer_size = 4096;
/// memory_share(
///     target_tcb_cap,
///     shared_buffer_vaddr,
///     buffer_size,
///     target_buffer_vaddr,
///     0x3 // read-write
/// )?;
/// ```
///
/// # Security
/// Requires CAP_MEMORY permission and a valid TCB capability for the target process.
/// This allows direct memory sharing without copying data. The target process can
/// access the same physical pages at the specified virtual address.
///
/// # Implementation
/// For each page in the region:
/// 1. Translates source virtual address to physical address
/// 2. Maps the physical address into target process at destination virtual address
/// 3. Applies the specified permissions to the target mapping
/// 4. Flushes TLB to ensure visibility
pub fn memory_share(
    target_tcb_cap: usize,
    source_virt_addr: usize,
    size: usize,
    dest_virt_addr: usize,
    permissions: usize,
) -> Result<()> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_MEMORY_SHARE,
            inlateout("x0") target_tcb_cap => result,
            inlateout("x1") source_virt_addr => _,
            inlateout("x2") size => _,
            inlateout("x3") dest_virt_addr => _,
            inlateout("x4") permissions => _,
            lateout("x8") _,
        );
        Error::from_syscall(result)?;
        Ok(())
    }
}

/// Request access to a device
///
/// # Arguments
/// * `device_id` - Device identifier
///
/// # Returns
/// Device capability slot on success.
pub fn device_request(device_id: usize) -> Result<usize> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_DEVICE_REQUEST,
            inlateout("x0") device_id => result,
            lateout("x8") _,
        );
        Error::from_syscall(result)
    }
}

/// Create a notification object
///
/// **Notifications are synchronization primitives**, not message-passing channels.
/// They provide lightweight signaling with badge bits but carry no data payload.
///
/// For typed message passing, use `Channel<T>` from the `message` module instead,
/// which combines notifications with shared memory ring buffers.
///
/// # Returns
/// Notification capability slot on success.
///
/// # Use Cases
/// - Event signaling (e.g., "data ready", "work complete")
/// - Synchronization between threads/processes
/// - Interrupt notification from drivers
///
/// # Example
/// ```no_run
/// let notification = kaal_sdk::syscall::notification_create()?;
/// // Later: signal it
/// kaal_sdk::syscall::signal(notification, 0x1)?;
/// ```
pub fn notification_create() -> Result<usize> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) numbers::SYS_NOTIFICATION_CREATE,
            result = out(reg) result,
            out("x8") _,
        );
        Error::from_syscall(result)
    }
}

/// Signal a notification (non-blocking)
///
/// # Arguments
/// * `notification` - Notification capability slot
/// * `badge` - Signal badge to OR into notification
///
/// # Example
/// ```no_run
/// kaal_sdk::syscall::signal(notification, 0x1)?;
/// ```
pub fn signal(notification: usize, badge: u64) -> Result<()> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_SIGNAL,
            inlateout("x0") notification => result,
            inlateout("x1") badge => _,
            lateout("x8") _,
        );
        Error::from_syscall(result)?;
        Ok(())
    }
}

/// Wait for notification (blocking)
///
/// Blocks until the notification is signaled, then returns the signal bits.
///
/// # Arguments
/// * `notification` - Notification capability slot
///
/// # Returns
/// Signal bits on success.
///
/// # Example
/// ```no_run
/// let signals = kaal_sdk::syscall::wait(notification)?;
/// if signals & 0x1 != 0 {
///     // Handle signal 1
/// }
/// ```
pub fn wait(notification: usize) -> Result<u64> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_WAIT,
            inlateout("x0") notification => result,
            lateout("x8") _,
        );
        Error::from_syscall(result).map(|v| v as u64)
    }
}

/// Poll notification (non-blocking)
///
/// Returns immediately with signal bits, or 0 if no signals pending.
///
/// # Arguments
/// * `notification` - Notification capability slot
///
/// # Returns
/// Signal bits (0 if none pending).
///
/// # Example
/// ```no_run
/// let signals = kaal_sdk::syscall::poll(notification)?;
/// if signals != 0 {
///     // Process signals
/// }
/// ```
pub fn poll(notification: usize) -> Result<u64> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "mov x0, {cap}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) numbers::SYS_POLL,
            cap = in(reg) notification,
            result = out(reg) result,
            out("x8") _,
        );
        // Poll doesn't fail, returns 0 if no signals
        Ok(result as u64)
    }
}

/// Create an IPC endpoint
///
/// # Returns
/// Endpoint capability slot on success.
pub fn endpoint_create() -> Result<usize> {
    unsafe {
        let result: usize;
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) numbers::SYS_ENDPOINT_CREATE,
            result = out(reg) result,
            out("x8") _,
        );
        Error::from_syscall(result)
    }
}

// ============================================================================
// Raw syscall helpers - for internal use by SDK modules
// ============================================================================

/// Perform a raw system call with 1 argument
///
/// # Safety
/// Caller must ensure the syscall number and argument are valid for the kernel.
///
/// # Parameters
/// - `syscall_num`: The syscall number from `numbers` module
/// - `arg0`: First argument to pass in x0
///
/// # Returns
/// The raw return value from the kernel in x0
#[doc(hidden)]
pub unsafe fn raw_syscall_1arg(syscall_num: usize, arg0: usize) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "svc #0",
        syscall_num = in(reg) syscall_num,
        inlateout("x0") arg0 => result,
        lateout("x8") _,
    );
    result
}

/// Perform a raw system call with 3 arguments
///
/// # Safety
/// Caller must ensure the syscall number and arguments are valid for the kernel.
///
/// # Parameters
/// - `syscall_num`: The syscall number from `numbers` module
/// - `arg0`: First argument to pass in x0
/// - `arg1`: Second argument to pass in x1
/// - `arg2`: Third argument to pass in x2
///
/// # Returns
/// The raw return value from the kernel in x0
#[doc(hidden)]
pub unsafe fn raw_syscall_3args(syscall_num: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "svc #0",
        syscall_num = in(reg) syscall_num,
        inlateout("x0") arg0 => result,
        inlateout("x1") arg1 => _,
        inlateout("x2") arg2 => _,
        lateout("x8") _,
    );
    result
}

// ============================================================================
// Syscall invocation macro for cleaner code
// ============================================================================

/// Invoke a system call with variable number of arguments
///
/// # Examples
/// ```
/// syscall!(SYS_YIELD);                               // 0 args
/// syscall!(SYS_CHANNEL_QUERY, channel_id);           // 1 arg
/// syscall!(SYS_MEMORY_MAP, addr, size, perms);       // 3 args
/// ```
#[macro_export]
macro_rules! syscall {
    // 0 arguments
    ($num:expr) => {{
        let result: usize;
        unsafe {
            core::arch::asm!(
                "mov x8, {syscall_num}",
                "svc #0",
                "mov {result}, x0",
                syscall_num = in(reg) $num,
                result = out(reg) result,
                out("x8") _,
            );
            result
        }
    }};

    // 1 argument
    ($num:expr, $arg0:expr) => {{
        unsafe { $crate::syscall::raw_syscall_1arg($num, $arg0 as usize) }
    }};

    // 2 arguments
    ($num:expr, $arg0:expr, $arg1:expr) => {{
        let result: usize;
        unsafe {
            core::arch::asm!(
                "mov x8, {syscall_num}",
                "svc #0",
                syscall_num = in(reg) $num,
                inlateout("x0") $arg0 as usize => result,
                inlateout("x1") $arg1 as usize => _,
                lateout("x8") _,
            );
            result
        }
    }};

    // 3 arguments
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr) => {{
        unsafe { $crate::syscall::raw_syscall_3args($num, $arg0 as usize, $arg1 as usize, $arg2 as usize) }
    }};

    // 4 arguments
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {{
        let result: usize;
        unsafe {
            core::arch::asm!(
                "mov x8, {num}",
                "svc #0",
                num = in(reg) $num,
                inlateout("x0") $arg0 as usize => result,
                inlateout("x1") $arg1 as usize => _,
                inlateout("x2") $arg2 as usize => _,
                inlateout("x3") $arg3 as usize => _,
                lateout("x8") _,
            );
            result
        }
    }};

    // 5 arguments
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {{
        let result: usize;
        unsafe {
            core::arch::asm!(
                "mov x8, {num}",
                "svc #0",
                num = in(reg) $num,
                inlateout("x0") $arg0 as usize => result,
                inlateout("x1") $arg1 as usize => _,
                inlateout("x2") $arg2 as usize => _,
                inlateout("x3") $arg3 as usize => _,
                inlateout("x4") $arg4 as usize => _,
                lateout("x8") _,
            );
            result
        }
    }};

    // 10 arguments (8 in x0-x7, priority in x9, capabilities in x10)
    // Special case for SYS_PROCESS_CREATE
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr, $arg7:expr, $priority:expr, $capabilities:expr) => {{
        let result: usize;
        unsafe {
            core::arch::asm!(
                "mov x8, {num}",
                "svc #0",
                num = in(reg) $num,
                inlateout("x0") $arg0 as usize => result,
                inlateout("x1") $arg1 as usize => _,
                inlateout("x2") $arg2 as usize => _,
                inlateout("x3") $arg3 as usize => _,
                inlateout("x4") $arg4 as usize => _,
                inlateout("x5") $arg5 as usize => _,
                inlateout("x6") $arg6 as usize => _,
                inlateout("x7") $arg7 as usize => _,
                inlateout("x9") $priority as usize => _,
                inlateout("x10") $capabilities as usize => _,
                lateout("x8") _,
            );
            result
        }
    }};
}

/// Register shared memory with the kernel registry
///
/// Allows producer to publish physical address for consumers to discover
pub unsafe fn shmem_register(channel_name: &str, phys_addr: usize, size: usize, notification_cap: usize) -> crate::Result<()> {
    let result = crate::syscall!(
        numbers::SYS_SHMEM_REGISTER,
        channel_name.as_ptr(),
        channel_name.len(),
        phys_addr,
        size,
        notification_cap
    );

    if result == usize::MAX {
        Err(crate::Error::SyscallFailed)
    } else {
        Ok(())
    }
}

/// Query shared memory from the kernel registry
///
/// Allows consumer to discover physical address published by producer
pub unsafe fn shmem_query(channel_name: &str) -> crate::Result<usize> {
    let phys_addr = crate::syscall!(
        numbers::SYS_SHMEM_QUERY,
        channel_name.as_ptr(),
        channel_name.len()
    );

    if phys_addr == 0 {
        Err(crate::Error::SyscallFailed)
    } else {
        Ok(phys_addr)
    }
}

/// Get notification capability for a shared memory channel
///
/// Allows consumer to get a capability to the producer's notification for signaling
pub unsafe fn shmem_get_notification(channel_name: &str, dest_cap_slot: usize) -> crate::Result<()> {
    let result = crate::syscall!(
        numbers::SYS_SHMEM_GET_NOTIFICATION,
        channel_name.as_ptr(),
        channel_name.len(),
        dest_cap_slot
    );

    if result == usize::MAX {
        Err(crate::Error::SyscallFailed)
    } else {
        Ok(())
    }
}

/// Map physical memory into another component's address space (privileged)
///
/// This is a privileged syscall only available to the root-task for
/// centralized IPC channel establishment.
///
/// # Arguments
///
/// * `target_tcb_cap` - TCB capability of target component
/// * `phys_addr` - Physical address to map
/// * `size` - Size in bytes (must be page-aligned)
/// * `virt_addr` - Virtual address in target's address space
/// * `permissions` - Permission flags (0x3 = read-write)
///
/// # Safety
///
/// Unsafe because it modifies another component's address space
pub unsafe fn memory_map_into(
    target_tcb_cap: usize,
    phys_addr: usize,
    size: usize,
    virt_addr: usize,
    permissions: usize,
) -> crate::Result<()> {
    let result = crate::syscall!(
        numbers::SYS_MEMORY_MAP_INTO,
        target_tcb_cap,
        phys_addr,
        size,
        virt_addr,
        permissions
    );

    if result == 0 {
        Ok(())
    } else {
        Err(crate::Error::SyscallFailed)
    }
}

/// Insert a capability into another component's CSpace (privileged)
///
/// This is a privileged syscall only available to the root-task for
/// transferring capabilities to components during channel establishment.
///
/// # Arguments
///
/// * `target_tcb_cap` - TCB capability of target component
/// * `target_slot` - Slot in target's CSpace
/// * `cap_type` - Capability type (3 = Notification)
/// * `object_ptr` - Object reference
///
/// # Safety
///
/// Unsafe because it modifies another component's CSpace
pub unsafe fn cap_insert_into(
    target_tcb_cap: usize,
    target_slot: usize,
    cap_type: usize,
    object_ptr: usize,
) -> crate::Result<()> {
    let result = crate::syscall!(
        numbers::SYS_CAP_INSERT_INTO,
        target_tcb_cap,
        target_slot,
        cap_type,
        object_ptr
    );

    if result == 0 {
        Ok(())
    } else {
        Err(crate::Error::SyscallFailed)
    }
}

/// Create a new process with full isolation
///
/// # Arguments
///
/// * `entry_point` - Initial program counter
/// * `stack_pointer` - Initial stack pointer (virtual address)
/// * `page_table_root` - Physical address of page table (TTBR0)
/// * `cspace_root` - Physical address of CNode (capability space root)
/// * `code_phys` - Physical address where code is loaded
/// * `code_vaddr` - Virtual address where code should be mapped
/// * `code_size` - Size of code region in bytes
/// * `stack_phys` - Physical address where stack is located
/// * `priority` - Scheduling priority (0-255)
/// * `capabilities` - Capability bitmask for the new process
///
/// # Returns
///
/// Process ID (TCB physical address), or error on failure
///
/// # Safety
///
/// Unsafe because it creates a new isolated process with its own address space
pub unsafe fn process_create(
    entry_point: usize,
    stack_pointer: usize,
    page_table_root: usize,
    cspace_root: usize,
    code_phys: usize,
    code_vaddr: usize,
    code_size: usize,
    stack_phys: usize,
    priority: u8,
    capabilities: u64,
) -> crate::Result<usize> {
    let result = crate::syscall!(
        numbers::SYS_PROCESS_CREATE,
        entry_point,
        stack_pointer,
        page_table_root,
        cspace_root,
        code_phys,
        code_vaddr,
        code_size,
        stack_phys,
        priority,
        capabilities
    );

    if result == usize::MAX {
        Err(crate::Error::SyscallFailed)
    } else {
        Ok(result)
    }
}

/// Insert capability into caller's own CSpace
///
/// # Arguments
///
/// * `slot` - Slot in caller's CSpace
/// * `cap_type` - Capability type (4 = TCB)
/// * `object_ptr` - Object reference (PID for TCB)
///
/// # Returns
///
/// Ok(()) on success, error on failure
///
/// # Safety
///
/// Unsafe because it modifies the caller's CSpace
pub unsafe fn cap_insert_self(
    slot: usize,
    cap_type: usize,
    object_ptr: usize,
) -> crate::Result<()> {
    let result = crate::syscall!(
        numbers::SYS_CAP_INSERT_SELF,
        slot,
        cap_type,
        object_ptr
    );

    if result == 0 {
        Ok(())
    } else {
        Err(crate::Error::SyscallFailed)
    }
}

// =============================================================================
// IRQ Handling Syscalls
// =============================================================================

/// Allocate an IRQ handler capability (requires IRQControl capability)
///
/// This syscall allows a process with an IRQControl capability to allocate
/// an IRQHandler for a specific hardware interrupt and bind it to a notification.
///
/// # Arguments
///
/// * `irq_control_cap` - Capability slot containing IRQControl capability
/// * `irq_num` - Hardware IRQ number to allocate (e.g., 27 for timer, 33 for UART0)
/// * `notification_cap` - Capability slot containing notification to signal on IRQ
/// * `irq_handler_slot` - Empty capability slot to store the new IRQHandler
///
/// # Returns
///
/// Ok(()) on success, error on failure
///
/// # Security
///
/// - Requires IRQControl capability (only root-task has this by default)
/// - Only one IRQHandler can exist per IRQ number
/// - IRQHandler is bound to the specific notification
///
/// # Example
///
/// ```no_run
/// use kaal_sdk::syscall;
///
/// // Only root-task has IRQControl in slot 0
/// let irq_control = 0;
/// let uart_irq = 33;
///
/// // Create notification for IRQ signaling
/// let notification = syscall::notification_create()?;
///
/// // Allocate slot for IRQHandler
/// let irq_handler_slot = syscall::cap_allocate()?;
///
/// // Allocate IRQ handler
/// syscall::irq_handler_get(irq_control, uart_irq, notification, irq_handler_slot)?;
/// ```
pub fn irq_handler_get(
    irq_control_cap: usize,
    irq_num: usize,
    notification_cap: usize,
    irq_handler_slot: usize,
) -> crate::Result<()> {
    let result = crate::syscall!(
        numbers::SYS_IRQ_HANDLER_GET,
        irq_control_cap,
        irq_num,
        notification_cap,
        irq_handler_slot
    );

    if result == 0 {
        Ok(())
    } else {
        Err(crate::Error::SyscallFailed)
    }
}

/// Acknowledge IRQ and re-enable it (requires IRQHandler capability)
///
/// This syscall must be called by a device driver after it has serviced an interrupt.
/// It re-enables the IRQ at the GIC, allowing future interrupts to be delivered.
///
/// # Arguments
///
/// * `irq_handler_cap` - Capability slot containing IRQHandler capability
///
/// # Returns
///
/// Ok(()) on success, error on failure
///
/// # Security
///
/// - Requires IRQHandler capability for the specific IRQ
/// - Only the holder of the IRQHandler can acknowledge the IRQ
///
/// # Example
///
/// ```no_run
/// use kaal_sdk::syscall;
///
/// // Device driver IRQ handling loop
/// loop {
///     // Wait for IRQ
///     syscall::wait(notification)?;
///
///     // Service the device
///     handle_uart_interrupt();
///
///     // Re-enable IRQ
///     syscall::irq_handler_ack(irq_handler)?;
/// }
/// ```
pub fn irq_handler_ack(irq_handler_cap: usize) -> crate::Result<()> {
    let result = crate::syscall!(
        numbers::SYS_IRQ_HANDLER_ACK,
        irq_handler_cap
    );

    if result == 0 {
        Ok(())
    } else {
        Err(crate::Error::SyscallFailed)
    }
}

// ============================================================================
// System Control Functions
// ============================================================================

/// Shutdown the system
///
/// Requests the kernel to power off the system. On QEMU, this cleanly exits
/// the emulator. On real hardware, this powers off the system via PSCI.
///
/// This function does not return.
///
/// # Example
/// ```no_run
/// // Clean shutdown when application exits
/// kaal_sdk::syscall::shutdown();
/// ```
pub fn shutdown() -> ! {
    unsafe {
        core::arch::asm!(
            "mov x8, {syscall_num}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_SHUTDOWN,
            options(noreturn)
        );
    }
}
