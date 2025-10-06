//! seL4 Platform Adapter Layer
//!
//! Provides a unified API by re-exporting the appropriate backend.
//! Based on ACTUAL rust-sel4 generated bindings structure.

// ========== Mock Mode ==========
#[cfg(feature = "mock")]
pub use sel4_mock_sys::*;

// ========== Runtime Mode ==========
#[cfg(feature = "runtime")]
pub use sel4::sys::*;

// Re-export error constants from the seL4_Error module
#[cfg(feature = "runtime")]
pub use sel4::sys::seL4_Error::{
    seL4_NoError,
    seL4_InvalidArgument,
    seL4_InvalidCapability,
    seL4_IllegalOperation,
    seL4_RangeError,
    seL4_AlignmentError,
    seL4_FailedLookup,
    seL4_TruncatedMessage,
    seL4_DeleteFirst,
    seL4_RevokeFirst,
    seL4_NotEnoughMemory,
};

// Re-export object types from api_object module (NOT _mode_object!)
#[cfg(feature = "runtime")]
pub use sel4::sys::api_object::{
    seL4_UntypedObject,
    seL4_TCBObject,
    seL4_EndpointObject,
    seL4_NotificationObject,
    seL4_CapTableObject,
};

// Re-export ARM page objects from _object module
#[cfg(feature = "runtime")]
pub use sel4::sys::_object::{
    seL4_ARM_SmallPageObject as seL4_ARCH_4KPage,
    seL4_ARM_LargePageObject as seL4_ARCH_LargePage,
    seL4_ARM_PageTableObject,
};

// Re-export ARM HugePage from _mode_object
#[cfg(feature = "runtime")]
pub use sel4::sys::_mode_object::{
    seL4_ARM_HugePageObject as seL4_ARCH_HugePage,
};

// ARM VM attributes - using the actual constant names from bindings
#[cfg(feature = "runtime")]
pub use sel4::sys::seL4_ARM_VMAttributes::{
    seL4_ARM_Default_VMAttributes as seL4_ARCH_WriteBack,
    seL4_ARM_PageCacheable as seL4_ARCH_PageCacheable,
};

// Constants for missing VM attributes (these don't exist in ARM)
#[cfg(feature = "runtime")]
#[allow(non_upper_case_globals)]
pub const seL4_ARCH_Uncached: sel4::sys::seL4_Word = 0;

// Capability rights - rust-sel4 uses seL4_CapRights struct, not bit constants
// Provide bit constants for compatibility
#[cfg(feature = "runtime")]
pub const seL4_CanRead: sel4::sys::seL4_Word = 1 << 0;
#[cfg(feature = "runtime")]
pub const seL4_CanWrite: sel4::sys::seL4_Word = 1 << 1;
#[cfg(feature = "runtime")]
pub const seL4_CanGrant: sel4::sys::seL4_Word = 1 << 2;

// ========== Type Aliases ==========

#[cfg(feature = "mock")]
pub type Error = sel4_mock_sys::seL4_Error;

#[cfg(feature = "runtime")]
pub type Error = sel4::sys::seL4_Error::Type;

// ========== Helper Functions ==========

#[inline]
pub fn is_ok(err: Error) -> bool {
    #[cfg(feature = "mock")]
    { err == sel4_mock_sys::seL4_NoError }

    #[cfg(feature = "runtime")]
    { err == seL4_NoError }
}

#[inline]
pub fn is_err(err: Error) -> bool {
    !is_ok(err)
}

// ========== Re-export Syscalls ==========
// Make syscall functions available from adapter module for compatibility
pub use crate::syscalls::*;
