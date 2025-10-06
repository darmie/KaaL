//! seL4 Platform Adapter Layer
//!
//! This module provides a unified API that matches what KaaL expects,
//! but delegates to the appropriate backend (mock vs real seL4).

// ========== Backend Selection ==========

#[cfg(feature = "mock")]
use sel4_mock_sys as sys_backend;

#[cfg(feature = "runtime")]
use sel4::sys as sys_backend;

// ========== Type Aliases ==========

pub type Word = sys_backend::seL4_Word;
pub type CPtr = sys_backend::seL4_CPtr;

#[allow(non_camel_case_types)]
pub type seL4_Word = Word;
#[allow(non_camel_case_types)]
pub type seL4_CPtr = CPtr;

// Error type - use the sys backend's error type directly
#[cfg(feature = "mock")]
pub type Error = sys_backend::seL4_Error;

#[cfg(feature = "runtime")]
pub type Error = sys_backend::seL4_Error::Type;

#[allow(non_camel_case_types)]
pub type seL4_Error = Error;

// ========== Common Type Re-exports ==========

pub use sys_backend::{
    seL4_BootInfo as BootInfo,
    seL4_IPCBuffer as IPCBuffer,
    seL4_SlotRegion as SlotRegion,
    seL4_UntypedDesc as UntypedDesc,
    seL4_UserContext as UserContext,
};

#[cfg(feature = "mock")]
pub use sys_backend::{
    seL4_MessageInfo as MessageInfo,
};

#[cfg(feature = "runtime")]
pub use sys_backend::{
    seL4_MessageInfo_t as MessageInfo,
};

// ========== Error Constants ==========

#[cfg(feature = "mock")]
pub use sys_backend::{
    seL4_NoError as NO_ERROR,
    seL4_InvalidArgument as INVALID_ARGUMENT,
    seL4_InvalidCapability as INVALID_CAPABILITY,
    seL4_IllegalOperation as ILLEGAL_OPERATION,
    seL4_RangeError as RANGE_ERROR,
    seL4_AlignmentError as ALIGNMENT_ERROR,
    seL4_FailedLookup as FAILED_LOOKUP,
    seL4_TruncatedMessage as TRUNCATED_MESSAGE,
    seL4_DeleteFirst as DELETE_FIRST,
    seL4_RevokeFirst as REVOKE_FIRST,
    seL4_NotEnoughMemory as NOT_ENOUGH_MEMORY,
};

// Real seL4 error constants from the seL4_Error module
#[cfg(feature = "runtime")]
pub use sys_backend::seL4_Error::{
    seL4_NoError as NO_ERROR,
    seL4_InvalidArgument as INVALID_ARGUMENT,
    seL4_InvalidCapability as INVALID_CAPABILITY,
    seL4_IllegalOperation as ILLEGAL_OPERATION,
    seL4_RangeError as RANGE_ERROR,
    seL4_AlignmentError as ALIGNMENT_ERROR,
    seL4_FailedLookup as FAILED_LOOKUP,
    seL4_TruncatedMessage as TRUNCATED_MESSAGE,
    seL4_DeleteFirst as DELETE_FIRST,
    seL4_RevokeFirst as REVOKE_FIRST,
    seL4_NotEnoughMemory as NOT_ENOUGH_MEMORY,
};

// ========== Capability Rights ==========

#[cfg(feature = "mock")]
pub use sys_backend::{
    seL4_CanRead as CAN_READ,
    seL4_CanWrite as CAN_WRITE,
    seL4_CanGrant as CAN_GRANT,
};

#[cfg(feature = "runtime")]
pub use sys_backend::seL4_CapRights::{
    seL4_CanRead as CAN_READ,
    seL4_CanWrite as CAN_WRITE,
    seL4_CanGrant as CAN_GRANT,
};

// ========== Object Types ==========

#[cfg(feature = "mock")]
pub use sys_backend::{
    seL4_UntypedObject as UNTYPED_OBJECT,
    seL4_TCBObject as TCB_OBJECT,
    seL4_EndpointObject as ENDPOINT_OBJECT,
    seL4_NotificationObject as NOTIFICATION_OBJECT,
    seL4_CapTableObject as CAP_TABLE_OBJECT,
};

#[cfg(feature = "runtime")]
pub use sys_backend::_mode_object::{
    seL4_UntypedObject as UNTYPED_OBJECT,
    seL4_TCBObject as TCB_OBJECT,
    seL4_EndpointObject as ENDPOINT_OBJECT,
    seL4_NotificationObject as NOTIFICATION_OBJECT,
    seL4_CapTableObject as CAP_TABLE_OBJECT,
};

// ========== Page Sizes (Architecture-Specific) ==========

#[cfg(feature = "mock")]
pub use sys_backend::{
    seL4_ARCH_4KPage as ARCH_4K_PAGE,
    seL4_ARCH_LargePage as ARCH_LARGE_PAGE,
    seL4_ARCH_HugePage as ARCH_HUGE_PAGE,
};

// ARM page objects from _object module
#[cfg(feature = "runtime")]
pub use sys_backend::_object::{
    seL4_ARM_SmallPageObject as ARCH_4K_PAGE,
    seL4_ARM_LargePageObject as ARCH_LARGE_PAGE,
    seL4_ARM_HugePageObject as ARCH_HUGE_PAGE,
};

// ========== Memory Attributes ==========

#[cfg(feature = "mock")]
pub use sys_backend::{
    seL4_ARCH_Uncached as ARCH_UNCACHED,
    seL4_ARCH_WriteThrough as ARCH_WRITE_THROUGH,
    seL4_ARCH_WriteCombining as ARCH_WRITE_COMBINING,
    seL4_ARCH_WriteBack as ARCH_WRITE_BACK,
};

// ARM VM attributes
#[cfg(feature = "runtime")]
pub use sys_backend::seL4_ARM_VMAttributes::{
    seL4_ARM_Default as ARCH_WRITE_BACK,
    seL4_ARM_PageCacheable as ARCH_WRITE_THROUGH,
};

// These don't have direct equivalents in ARM
#[cfg(feature = "runtime")]
pub const ARCH_UNCACHED: Word = 0;
#[cfg(feature = "runtime")]
pub const ARCH_WRITE_COMBINING: Word = 0;

// ========== Syscall Wrappers ==========

/// Get boot information from seL4
#[inline]
pub unsafe fn get_boot_info() -> *const BootInfo {
    sys_backend::seL4_GetBootInfo()
}

/// Retype untyped memory into typed objects
#[inline]
pub unsafe fn untyped_retype(
    untyped: CPtr,
    object_type: usize,
    size_bits: usize,
    root: CPtr,
    node_index: usize,
    node_depth: usize,
    node_offset: CPtr,
    num_objects: usize,
) -> Error {
    sys_backend::seL4_Untyped_Retype(
        untyped,
        object_type as _,
        size_bits as _,
        root,
        node_index as _,
        node_depth as _,
        node_offset,
        num_objects as _,
    )
}

/// Map a page into a VSpace
#[inline]
pub unsafe fn page_map(
    page: CPtr,
    vspace: CPtr,
    vaddr: usize,
    rights: Word,
    attr: Word,
) -> Error {
    sys_backend::seL4_ARM_Page_Map(
        page,
        vspace,
        vaddr as _,
        rights as _,
        attr as _,
    )
}

/// Unmap a page from a VSpace
#[inline]
pub unsafe fn page_unmap(page: CPtr) -> Error {
    sys_backend::seL4_ARM_Page_Unmap(page)
}

// ========== IRQ Management ==========

/// Get an IRQ handler capability
#[inline]
pub unsafe fn irq_control_get(
    irq_control: CPtr,
    irq: usize,
    root: CPtr,
    index: usize,
    depth: usize,
) -> Error {
    sys_backend::seL4_IRQControl_Get(
        irq_control,
        irq as _,
        root,
        index as _,
        depth as _,
    )
}

/// Set the notification for an IRQ handler
#[inline]
pub unsafe fn irq_handler_set_notification(
    irq_handler: CPtr,
    notification: CPtr,
) -> Error {
    sys_backend::seL4_IRQHandler_SetNotification(irq_handler, notification)
}

/// Acknowledge an IRQ
#[inline]
pub unsafe fn irq_handler_ack(irq_handler: CPtr) -> Error {
    sys_backend::seL4_IRQHandler_Ack(irq_handler)
}

/// Clear an IRQ handler
#[inline]
pub unsafe fn irq_handler_clear(irq_handler: CPtr) -> Error {
    sys_backend::seL4_IRQHandler_Clear(irq_handler)
}

// ========== IPC Operations ==========

/// Send and wait for reply (seL4_Call)
#[inline]
pub unsafe fn call(dest: CPtr, msg_info: MessageInfo) -> MessageInfo {
    sys_backend::seL4_Call(dest, msg_info)
}

/// Send a message (seL4_Send)
#[inline]
pub unsafe fn send(dest: CPtr, msg_info: MessageInfo) {
    sys_backend::seL4_Send(dest, msg_info)
}

/// Non-blocking send (seL4_NBSend)
#[inline]
pub unsafe fn nb_send(dest: CPtr, msg_info: MessageInfo) {
    sys_backend::seL4_NBSend(dest, msg_info)
}

/// Receive a message (seL4_Recv)
#[inline]
pub unsafe fn recv(src: CPtr, sender: *mut CPtr) -> MessageInfo {
    sys_backend::seL4_Recv(src, sender)
}

/// Wait for a message (seL4_Wait)
#[inline]
pub unsafe fn wait(src: CPtr, sender: *mut CPtr) -> MessageInfo {
    sys_backend::seL4_Wait(src, sender)
}

/// Signal a notification (seL4_Signal)
#[inline]
pub unsafe fn signal(notification: CPtr) {
    sys_backend::seL4_Signal(notification)
}

// ========== TCB Operations ==========

/// Configure a TCB
#[inline]
pub unsafe fn tcb_configure(
    tcb: CPtr,
    fault_ep: CPtr,
    cspace_root: CPtr,
    cspace_root_data: Word,
    vspace_root: CPtr,
    vspace_root_data: Word,
    buffer: usize,
    buffer_frame: CPtr,
) -> Error {
    sys_backend::seL4_TCB_Configure(
        tcb,
        fault_ep,
        cspace_root,
        cspace_root_data,
        vspace_root,
        vspace_root_data,
        buffer as _,
        buffer_frame,
    )
}

/// Set TCB priority
#[inline]
pub unsafe fn tcb_set_priority(tcb: CPtr, authority: CPtr, priority: usize) -> Error {
    sys_backend::seL4_TCB_SetPriority(tcb, authority, priority as _)
}

/// Set TCB scheduling parameters
#[inline]
pub unsafe fn tcb_set_sched_params(
    tcb: CPtr,
    authority: CPtr,
    mcp: usize,
    priority: usize,
) -> Error {
    sys_backend::seL4_TCB_SetSchedParams(tcb, authority, mcp as _, priority as _)
}

/// Write TCB registers
#[inline]
pub unsafe fn tcb_write_registers(
    tcb: CPtr,
    resume: bool,
    arch_flags: u8,
    count: usize,
    regs: *const UserContext,
) -> Error {
    sys_backend::seL4_TCB_WriteRegisters(
        tcb,
        resume as _,
        arch_flags,
        count as _,
        regs as *mut _,
    )
}

/// Resume a TCB
#[inline]
pub unsafe fn tcb_resume(tcb: CPtr) -> Error {
    sys_backend::seL4_TCB_Resume(tcb)
}

/// Suspend a TCB
#[inline]
pub unsafe fn tcb_suspend(tcb: CPtr) -> Error {
    sys_backend::seL4_TCB_Suspend(tcb)
}

/// Bind a notification to a TCB
#[inline]
pub unsafe fn tcb_bind_notification(tcb: CPtr, notification: CPtr) -> Error {
    sys_backend::seL4_TCB_BindNotification(tcb, notification)
}

/// Unbind notification from a TCB
#[inline]
pub unsafe fn tcb_unbind_notification(tcb: CPtr) -> Error {
    sys_backend::seL4_TCB_UnbindNotification(tcb)
}
