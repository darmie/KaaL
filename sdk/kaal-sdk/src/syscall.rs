//! System call wrappers
//!
//! Provides safe, ergonomic wrappers around raw KaaL syscalls.

use crate::{Result, Error};

/// Syscall numbers
mod numbers {
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
    pub const SYS_DEBUG_PRINT: usize = 0x1001;
}

/// Print a message to the debug console
///
/// # Example
/// ```no_run
/// kaal_sdk::syscall::print("Hello, world!\n");
/// ```
pub fn print(msg: &str) {
    unsafe {
        let msg_ptr = msg.as_ptr() as usize;
        let msg_len = msg.len();

        core::arch::asm!(
            "mov x8, {syscall_num}",
            "mov x0, {msg_ptr}",
            "mov x1, {msg_len}",
            "svc #0",
            syscall_num = in(reg) numbers::SYS_DEBUG_PRINT,
            msg_ptr = in(reg) msg_ptr,
            msg_len = in(reg) msg_len,
            out("x0") _,
            out("x1") _,
            out("x8") _,
        );
    }
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
            "mov x0, {size}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) numbers::SYS_MEMORY_ALLOCATE,
            size = in(reg) size,
            result = out(reg) result,
            out("x8") _,
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
            "mov x0, {phys}",
            "mov x1, {size}",
            "mov x2, {perms}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) numbers::SYS_MEMORY_MAP,
            phys = in(reg) phys_addr,
            size = in(reg) size,
            perms = in(reg) permissions,
            result = out(reg) result,
            out("x8") _,
            out("x0") _,
            out("x1") _,
            out("x2") _,
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
            "mov x0, {virt}",
            "mov x1, {size}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) numbers::SYS_MEMORY_UNMAP,
            virt = in(reg) virt_addr,
            size = in(reg) size,
            result = out(reg) result,
            out("x8") _,
            out("x0") _,
            out("x1") _,
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
            "mov x0, {device_id}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) numbers::SYS_DEVICE_REQUEST,
            device_id = in(reg) device_id,
            result = out(reg) result,
            out("x8") _,
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
            "mov x0, {cap}",
            "mov x1, {badge}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) numbers::SYS_SIGNAL,
            cap = in(reg) notification,
            badge = in(reg) badge,
            result = out(reg) result,
            out("x8") _,
            out("x0") _,
            out("x1") _,
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
            "mov x0, {cap}",
            "svc #0",
            "mov {result}, x0",
            syscall_num = in(reg) numbers::SYS_WAIT,
            cap = in(reg) notification,
            result = out(reg) result,
            out("x8") _,
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
