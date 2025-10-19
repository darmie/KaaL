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

    // Privileged syscalls for root-task
    pub const SYS_MEMORY_MAP_INTO: usize = 0x1B;
    pub const SYS_CAP_INSERT_INTO: usize = 0x1C;
    pub const SYS_CAP_INSERT_SELF: usize = 0x1D;

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
        "mov x0, {arg0}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) syscall_num,
        arg0 = in(reg) arg0,
        result = out(reg) result,
        out("x8") _,
        out("x0") _,
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
        "mov x0, {arg0}",
        "mov x1, {arg1}",
        "mov x2, {arg2}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) syscall_num,
        arg0 = in(reg) arg0,
        arg1 = in(reg) arg1,
        arg2 = in(reg) arg2,
        result = out(reg) result,
        out("x8") _,
        out("x0") _,
        out("x1") _,
        out("x2") _,
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
                "mov x0, {arg0}",
                "mov x1, {arg1}",
                "svc #0",
                "mov {result}, x0",
                syscall_num = in(reg) $num,
                arg0 = in(reg) $arg0 as usize,
                arg1 = in(reg) $arg1 as usize,
                result = out(reg) result,
                out("x8") _,
                out("x0") _,
                out("x1") _,
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
                "mov x0, {arg0}",
                "mov x1, {arg1}",
                "mov x2, {arg2}",
                "mov x3, {arg3}",
                "mov x8, {num}",
                "svc #0",
                "mov {result}, x0",
                arg0 = in(reg) $arg0 as usize,
                arg1 = in(reg) $arg1 as usize,
                arg2 = in(reg) $arg2 as usize,
                arg3 = in(reg) $arg3 as usize,
                num = in(reg) $num,
                result = out(reg) result,
                out("x0") _,
                out("x1") _,
                out("x2") _,
                out("x3") _,
                out("x8") _,
            );
            result
        }
    }};

    // 5 arguments
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {{
        let result: usize;
        unsafe {
            core::arch::asm!(
                "mov x0, {arg0}",
                "mov x1, {arg1}",
                "mov x2, {arg2}",
                "mov x3, {arg3}",
                "mov x4, {arg4}",
                "mov x8, {num}",
                "svc #0",
                "mov {result}, x0",
                arg0 = in(reg) $arg0 as usize,
                arg1 = in(reg) $arg1 as usize,
                arg2 = in(reg) $arg2 as usize,
                arg3 = in(reg) $arg3 as usize,
                arg4 = in(reg) $arg4 as usize,
                num = in(reg) $num,
                result = out(reg) result,
                out("x0") _,
                out("x1") _,
                out("x2") _,
                out("x3") _,
                out("x4") _,
                out("x8") _,
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
                "mov x0, {arg0}",
                "mov x1, {arg1}",
                "mov x2, {arg2}",
                "mov x3, {arg3}",
                "mov x4, {arg4}",
                "mov x5, {arg5}",
                "mov x6, {arg6}",
                "mov x7, {arg7}",
                "mov x8, {num}",
                "mov x9, {priority}",
                "mov x10, {capabilities}",
                "svc #0",
                "mov {result}, x0",
                arg0 = in(reg) $arg0 as usize,
                arg1 = in(reg) $arg1 as usize,
                arg2 = in(reg) $arg2 as usize,
                arg3 = in(reg) $arg3 as usize,
                arg4 = in(reg) $arg4 as usize,
                arg5 = in(reg) $arg5 as usize,
                arg6 = in(reg) $arg6 as usize,
                arg7 = in(reg) $arg7 as usize,
                num = in(reg) $num,
                priority = in(reg) $priority as usize,
                capabilities = in(reg) $capabilities as usize,
                result = out(reg) result,
                out("x0") _,
                out("x1") _,
                out("x2") _,
                out("x3") _,
                out("x4") _,
                out("x5") _,
                out("x6") _,
                out("x7") _,
                out("x8") _,
                out("x9") _,
                out("x10") _,
            );
            result
        }
    }};
}

/// Register shared memory with the kernel registry
///
/// Allows producer to publish physical address for consumers to discover
pub unsafe fn shmem_register(channel_name: &str, phys_addr: usize, size: usize) -> crate::Result<()> {
    let result = crate::syscall!(
        numbers::SYS_SHMEM_REGISTER,
        channel_name.as_ptr(),
        channel_name.len(),
        phys_addr,
        size
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
