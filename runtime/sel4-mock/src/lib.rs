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

/// Mock GetBootInfo - returns null for now
/// TODO PHASE 2: This must return real bootinfo from seL4 kernel
pub unsafe fn seL4_GetBootInfo() -> *const seL4_BootInfo {
    core::ptr::null()
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
