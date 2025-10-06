//! Unified seL4 syscall interface
//!
//! This module provides platform-agnostic syscalls that work across:
//! - Mock mode (for testing)
//! - Runtime mode (real seL4)
//!
//! rust-sel4 generates syscalls as methods on seL4_IPCBuffer.
//! This module provides C-style standalone function wrappers for compatibility.

// ========== Mock Mode ==========
#[cfg(feature = "mock")]
pub use sel4_mock_sys::*;

// ========== Runtime Mode - Standalone Function Wrappers ==========

#[cfg(feature = "runtime")]
use crate::adapter::*;

/// Access the thread-local IPC buffer (runtime mode)
#[cfg(feature = "runtime")]
extern "C" {
    static mut __sel4_ipc_buffer_ptr: *mut seL4_IPCBuffer;
}

#[cfg(feature = "runtime")]
#[inline]
unsafe fn ipc() -> &'static mut seL4_IPCBuffer {
    &mut *__sel4_ipc_buffer_ptr
}

// ========== Untyped Object Invocations ==========

#[cfg(feature = "runtime")]
pub unsafe fn seL4_Untyped_Retype(
    service: seL4_Untyped,
    type_: seL4_Word,
    size_bits: seL4_Word,
    root: seL4_CNode,
    node_index: seL4_Word,
    node_depth: seL4_Word,
    node_offset: seL4_Word,
    num_objects: seL4_Word,
) -> seL4_Error::Type {
    ipc().seL4_Untyped_Retype(service, type_, size_bits, root, node_index, node_depth, node_offset, num_objects)
}

// ========== TCB Invocations ==========

#[cfg(feature = "runtime")]
pub unsafe fn seL4_TCB_SetSpace(
    tcb: seL4_TCB,
    fault_ep: seL4_CPtr,
    cspace_root: seL4_CNode,
    cspace_root_data: seL4_Word,
    vspace_root: seL4_CPtr,
    vspace_root_data: seL4_Word,
) -> seL4_Error::Type {
    ipc().seL4_TCB_SetSpace(tcb, fault_ep, cspace_root, cspace_root_data, vspace_root, vspace_root_data)
}

#[cfg(feature = "runtime")]
pub unsafe fn seL4_TCB_SetIPCBuffer(
    tcb: seL4_TCB,
    buffer: seL4_Word,
    bufferFrame: seL4_CPtr,
) -> seL4_Error::Type {
    ipc().seL4_TCB_SetIPCBuffer(tcb, buffer, bufferFrame)
}

#[cfg(feature = "runtime")]
pub unsafe fn seL4_TCB_SetPriority(
    tcb: seL4_TCB,
    authority: seL4_TCB,
    priority: seL4_Word,
) -> seL4_Error::Type {
    ipc().seL4_TCB_SetPriority(tcb, authority, priority)
}

#[cfg(feature = "runtime")]
pub unsafe fn seL4_TCB_WriteRegisters(
    tcb: seL4_TCB,
    resume: seL4_Bool,
    arch_flags: seL4_Uint8,
    count: seL4_Word,
    regs: *mut seL4_UserContext,
) -> seL4_Error::Type {
    // rust-sel4 expects a reference, not a raw pointer
    ipc().seL4_TCB_WriteRegisters(tcb, resume, arch_flags, count, &*regs)
}

#[cfg(feature = "runtime")]
pub unsafe fn seL4_TCB_Resume(tcb: seL4_TCB) -> seL4_Error::Type {
    ipc().seL4_TCB_Resume(tcb)
}

#[cfg(feature = "runtime")]
pub unsafe fn seL4_TCB_Suspend(tcb: seL4_TCB) -> seL4_Error::Type {
    ipc().seL4_TCB_Suspend(tcb)
}

// ========== ARM Page Invocations ==========

#[cfg(feature = "runtime")]
pub unsafe fn seL4_ARM_Page_Map(
    page: seL4_ARM_Page,
    vspace: seL4_CPtr,
    vaddr: seL4_Word,
    rights: seL4_Word, // Accept u64 rights bitfield
    attr: seL4_Word, // VM attributes are passed as seL4_Word
) -> seL4_Error::Type {
    // Convert rights bitfield to seL4_CapRights struct via transmute
    let rights_struct: seL4_CapRights = core::mem::transmute(rights);
    ipc().seL4_ARM_Page_Map(page, vspace, vaddr, rights_struct, attr)
}

#[cfg(feature = "runtime")]
pub unsafe fn seL4_ARM_Page_Unmap(page: seL4_ARM_Page) -> seL4_Error::Type {
    ipc().seL4_ARM_Page_Unmap(page)
}

// Architecture-specific aliases
#[cfg(feature = "runtime")]
pub use seL4_ARM_Page_Map as seL4_ARCH_Page_Map;
#[cfg(feature = "runtime")]
pub use seL4_ARM_Page_Unmap as seL4_ARCH_Page_Unmap;

// ========== IRQ Invocations ==========

#[cfg(feature = "runtime")]
pub unsafe fn seL4_IRQControl_Get(
    control: seL4_IRQControl,
    irq: seL4_Word,
    root: seL4_CNode,
    index: seL4_Word,
    depth: seL4_Word,
) -> seL4_Error::Type {
    // rust-sel4 expects depth as u8
    ipc().seL4_IRQControl_Get(control, irq, root, index, depth as u8)
}

#[cfg(feature = "runtime")]
pub unsafe fn seL4_IRQHandler_SetNotification(
    handler: seL4_IRQHandler,
    notification: seL4_CPtr,
) -> seL4_Error::Type {
    ipc().seL4_IRQHandler_SetNotification(handler, notification)
}

#[cfg(feature = "runtime")]
pub unsafe fn seL4_IRQHandler_Ack(handler: seL4_IRQHandler) -> seL4_Error::Type {
    ipc().seL4_IRQHandler_Ack(handler)
}

// ========== IPC Syscalls ==========

#[cfg(feature = "runtime")]
pub unsafe fn seL4_Wait(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    // rust-sel4's seL4_Wait returns ((), badge) tuple, and takes only src parameter
    let (_msg_info, badge) = ipc().seL4_Wait(src);
    // Write badge to the sender pointer if provided
    if !sender.is_null() {
        *sender = badge;
    }
    // Return empty message info - transmute from u64 (seL4_MessageInfo is a newtype wrapper)
    core::mem::transmute::<u64, seL4_MessageInfo>(0u64)
}
