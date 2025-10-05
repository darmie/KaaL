//! ⚠️  MOCK seL4 Implementation for Phase 1 Development
//!
//! # WARNING: This is NOT the real seL4!
//!
//! This is a minimal mock implementation to allow KaaL development
//! without requiring the full seL4 kernel build.
//!
//! ## Phase 2 TODO: Replace with Real seL4
//!
//! In Phase 2, this mock must be replaced with:
//! 1. Real seL4-sys crate from https://github.com/seL4/rust-sel4
//! 2. Actual seL4 kernel binaries
//! 3. Proper capability derivation
//! 4. Real IPC primitives
//!
//! ## Current Limitations
//!
//! - All operations are stubs
//! - No actual kernel functionality
//! - Types are placeholders
//! - Used ONLY for compilation and unit tests
//!
//! ## Usage
//!
//! This mock is automatically used when building without the full seL4 kernel.
//! To use real seL4 in Phase 2, remove this crate and use the official seL4-sys.

#![no_std]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

//! TODO PHASE 2: Remove this entire file and use real seL4-sys

// Basic seL4 types (mocked)
pub type seL4_Word = usize;
pub type seL4_CPtr = seL4_Word;
pub type seL4_CNode = seL4_Word;
pub type seL4_IRQHandler = seL4_Word;
pub type seL4_Error = i32;

// Constants
pub const seL4_NoError: seL4_Error = 0;
pub const seL4_InvalidArgument: seL4_Error = 1;
pub const seL4_InvalidCapability: seL4_Error = 2;
pub const seL4_IllegalOperation: seL4_Error = 3;
pub const seL4_RangeError: seL4_Error = 4;
pub const seL4_AlignmentError: seL4_Error = 5;
pub const seL4_FailedLookup: seL4_Error = 6;
pub const seL4_TruncatedMessage: seL4_Error = 7;
pub const seL4_DeleteFirst: seL4_Error = 8;
pub const seL4_RevokeFirst: seL4_Error = 9;
pub const seL4_NotEnoughMemory: seL4_Error = 10;

// Capability Rights
pub const seL4_CanRead: seL4_Word = 0x01;
pub const seL4_CanWrite: seL4_Word = 0x02;
pub const seL4_CanGrant: seL4_Word = 0x04;

// Mock boot info
#[repr(C)]
pub struct seL4_BootInfo {
    pub extraLen: seL4_Word,
    pub nodeID: seL4_Word,
    pub numNodes: seL4_Word,
    pub numIOPTLevels: seL4_Word,
    pub ipcBuffer: *mut seL4_IPCBuffer,
    pub empty: seL4_SlotRegion,
    pub sharedFrames: seL4_SlotRegion,
    pub userImageFrames: seL4_SlotRegion,
    pub userImagePaging: seL4_SlotRegion,
    pub ioSpaceCaps: seL4_SlotRegion,
    pub extraBIPages: seL4_SlotRegion,
    pub initThreadCNodeSizeBits: seL4_Word,
    pub initThreadDomain: seL4_Word,
    pub untyped: seL4_UntypedDesc,
}

#[repr(C)]
pub struct seL4_SlotRegion {
    pub start: seL4_CPtr,
    pub end: seL4_CPtr,
}

#[repr(C)]
pub struct seL4_UntypedDesc {
    pub paddr: seL4_Word,
    pub sizeBits: u8,
    pub isDevice: u8,
    pub padding: [u8; 6],
}

#[repr(C)]
pub struct seL4_IPCBuffer {
    pub tag: seL4_MessageInfo,
    pub msg: [seL4_Word; 120],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct seL4_MessageInfo {
    pub words: [seL4_Word; 1],
}

impl seL4_MessageInfo {
    pub fn new(label: seL4_Word, caps: seL4_Word, extra: seL4_Word, length: seL4_Word) -> Self {
        Self {
            words: [label | (caps << 12) | (extra << 16) | (length << 20)],
        }
    }
}

// Mock syscalls (all return success for Phase 1)
/// TODO PHASE 2: Replace with real seL4_Call
pub unsafe fn seL4_Call(_dest: seL4_CPtr, _msgInfo: seL4_MessageInfo) -> seL4_MessageInfo {
    seL4_MessageInfo::new(0, 0, 0, 0)
}

/// TODO PHASE 2: Replace with real seL4_Send
pub unsafe fn seL4_Send(_dest: seL4_CPtr, _msgInfo: seL4_MessageInfo) {
    // Mock: do nothing
}

/// TODO PHASE 2: Replace with real seL4_NBSend
pub unsafe fn seL4_NBSend(_dest: seL4_CPtr, _msgInfo: seL4_MessageInfo) {
    // Mock: do nothing
}

/// TODO PHASE 2: Replace with real seL4_Recv
pub unsafe fn seL4_Recv(_src: seL4_CPtr, _sender: *mut seL4_Word) -> seL4_MessageInfo {
    seL4_MessageInfo::new(0, 0, 0, 0)
}

/// TODO PHASE 2: Replace with real seL4_Wait
pub unsafe fn seL4_Wait(_src: seL4_CPtr, _sender: *mut seL4_Word) -> seL4_MessageInfo {
    seL4_MessageInfo::new(0, 0, 0, 0)
}

/// TODO PHASE 2: Replace with real seL4_Signal
pub unsafe fn seL4_Signal(_dest: seL4_CPtr) {
    // Mock: do nothing
}

/// TODO PHASE 2: Replace with real seL4_Yield
pub unsafe fn seL4_Yield() {
    // Mock: do nothing
}

// Mock object type constants
pub const seL4_UntypedObject: usize = 0;
pub const seL4_TCBObject: usize = 1;
pub const seL4_EndpointObject: usize = 2;
pub const seL4_NotificationObject: usize = 3;
pub const seL4_CapTableObject: usize = 4;

// Architecture-specific page sizes
pub const seL4_ARCH_4KPage: usize = 5;
pub const seL4_ARCH_LargePage: usize = 6;
pub const seL4_ARCH_HugePage: usize = 7;

// Memory attributes
pub const seL4_ARCH_Uncached: usize = 0;
pub const seL4_ARCH_WriteCombining: usize = 1;
pub const seL4_ARCH_WriteThrough: usize = 2;
pub const seL4_ARCH_WriteBack: usize = 3;

/// Mock GetBootInfo - returns null for now
/// TODO PHASE 2: This must return real bootinfo from seL4 kernel
pub unsafe fn seL4_GetBootInfo() -> *const seL4_BootInfo {
    core::ptr::null()
}

/// Mock Untyped_Retype - converts untyped memory to typed objects
/// TODO PHASE 2: Replace with real seL4_Untyped_Retype
pub unsafe fn seL4_Untyped_Retype(
    _untyped: seL4_CPtr,
    _type: usize,
    _size_bits: usize,
    _root: seL4_CPtr,
    _node_index: usize,
    _node_depth: usize,
    _node_offset: seL4_CPtr,
    _num_objects: usize,
) -> seL4_Error {
    seL4_NoError
}

/// Mock ARCH_Page_Map - maps a page into a VSpace
/// TODO PHASE 2: Replace with real seL4_ARCH_Page_Map
pub unsafe fn seL4_ARCH_Page_Map(
    _page: seL4_CPtr,
    _vspace: seL4_CPtr,
    _vaddr: usize,
    _rights: seL4_Word,
    _attr: usize,
) -> seL4_Error {
    seL4_NoError
}

/// Mock ARCH_Page_Unmap - unmaps a page from a VSpace
/// TODO PHASE 2: Replace with real seL4_ARCH_Page_Unmap
pub unsafe fn seL4_ARCH_Page_Unmap(_page: seL4_CPtr) -> seL4_Error {
    seL4_NoError
}

/// Mock IRQControl_Get - gets an IRQ handler capability
/// TODO PHASE 2: Replace with real seL4_IRQControl_Get
pub unsafe fn seL4_IRQControl_Get(
    _irq_control: seL4_CPtr,
    _irq: usize,
    _root: seL4_CPtr,
    _index: seL4_CPtr,
    _depth: usize,
) -> seL4_Error {
    seL4_NoError
}

/// Mock IRQHandler_SetNotification - binds IRQ to notification
/// TODO PHASE 2: Replace with real seL4_IRQHandler_SetNotification
pub unsafe fn seL4_IRQHandler_SetNotification(
    _irq_handler: seL4_CPtr,
    _notification: seL4_CPtr,
) -> seL4_Error {
    seL4_NoError
}

/// Mock IRQHandler_Ack - acknowledges an IRQ
/// TODO PHASE 2: Replace with real seL4_IRQHandler_Ack
pub unsafe fn seL4_IRQHandler_Ack(_irq_handler: seL4_CPtr) -> seL4_Error {
    seL4_NoError
}

/// Mock IRQHandler_Clear - clears IRQ handler binding
/// TODO PHASE 2: Replace with real seL4_IRQHandler_Clear
pub unsafe fn seL4_IRQHandler_Clear(_irq_handler: seL4_CPtr) -> seL4_Error {
    seL4_NoError
}

// ========== TCB Management ==========

/// User context structure for register state (x86_64)
#[cfg(target_arch = "x86_64")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct seL4_UserContext {
    pub rip: u64,
    pub rsp: u64,
    pub rflags: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub fs_base: u64,
    pub gs_base: u64,
}

/// User context structure for register state (aarch64 / ARM64)
#[cfg(target_arch = "aarch64")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct seL4_UserContext {
    pub pc: u64,  // Program counter
    pub sp: u64,  // Stack pointer
    pub spsr: u64, // Saved program status register
    pub x0: u64,
    pub x1: u64,
    pub x2: u64,
    pub x3: u64,
    pub x4: u64,
    pub x5: u64,
    pub x6: u64,
    pub x7: u64,
    pub x8: u64,
    pub x9: u64,
    pub x10: u64,
    pub x11: u64,
    pub x12: u64,
    pub x13: u64,
    pub x14: u64,
    pub x15: u64,
    pub x16: u64,
    pub x17: u64,
    pub x18: u64,
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64,  // Frame pointer
    pub x30: u64,  // Link register
    pub tpidr_el0: u64, // Thread ID register
    pub tpidrro_el0: u64, // Read-only thread ID
}

/// Mock TCB_SetSpace - configure TCB's CSpace and VSpace
/// TODO PHASE 2: Replace with real seL4_TCB_SetSpace
pub unsafe fn seL4_TCB_SetSpace(
    _tcb: seL4_CPtr,
    _fault_ep: seL4_CPtr,
    _cspace_root: seL4_CPtr,
    _cspace_root_data: seL4_Word,
    _vspace_root: seL4_CPtr,
    _vspace_root_data: seL4_Word,
) -> seL4_Error {
    seL4_NoError
}

/// Mock TCB_SetIPCBuffer - set IPC buffer for TCB
/// TODO PHASE 2: Replace with real seL4_TCB_SetIPCBuffer
pub unsafe fn seL4_TCB_SetIPCBuffer(
    _tcb: seL4_CPtr,
    _buffer_addr: seL4_Word,
    _buffer_frame: seL4_CPtr,
) -> seL4_Error {
    seL4_NoError
}

/// Mock TCB_SetPriority - set TCB priority
/// TODO PHASE 2: Replace with real seL4_TCB_SetPriority
pub unsafe fn seL4_TCB_SetPriority(
    _tcb: seL4_CPtr,
    _authority: seL4_CPtr,
    _priority: u8,
) -> seL4_Error {
    seL4_NoError
}

/// Mock TCB_WriteRegisters - write CPU registers for TCB
/// TODO PHASE 2: Replace with real seL4_TCB_WriteRegisters
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub unsafe fn seL4_TCB_WriteRegisters(
    _tcb: seL4_CPtr,
    _resume: seL4_Word,
    _arch_flags: u8,
    _count: usize,
    _regs: *mut seL4_UserContext,
) -> seL4_Error {
    seL4_NoError
}

/// Mock TCB_Resume - start/resume TCB execution
/// TODO PHASE 2: Replace with real seL4_TCB_Resume
pub unsafe fn seL4_TCB_Resume(_tcb: seL4_CPtr) -> seL4_Error {
    seL4_NoError
}

/// Mock TCB_Suspend - suspend TCB execution
/// TODO PHASE 2: Replace with real seL4_TCB_Suspend
pub unsafe fn seL4_TCB_Suspend(_tcb: seL4_CPtr) -> seL4_Error {
    seL4_NoError
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_types() {
        let msg = seL4_MessageInfo::new(0, 0, 0, 0);
        assert_eq!(msg.words[0], 0);
    }

    #[test]
    fn test_mock_errors() {
        assert_eq!(seL4_NoError, 0);
        assert_ne!(seL4_InvalidArgument, seL4_NoError);
    }
}
