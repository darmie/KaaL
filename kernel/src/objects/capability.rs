//! Capability System
//!
//! This module implements the capability-based security model for KaaL.
//! Capabilities are unforgeable tokens that grant specific rights to kernel objects.
//!
//! ## Design
//!
//! Following seL4's capability model:
//! - Capabilities are stored in CNodes (capability nodes)
//! - Each capability has a type (TCB, Endpoint, Page, etc.)
//! - Each capability has rights (Read, Write, Grant)
//! - Capabilities can be derived with reduced rights
//! - User space cannot forge capabilities
//!
//! ## Capability Structure
//!
//! A capability is 32 bytes (cache-line friendly):
//! ```
//! struct Capability {
//!     cap_type: CapType      (1 byte)
//!     _padding: [u8; 7]      (7 bytes)
//!     object_ptr: usize      (8 bytes)
//!     rights: CapRights      (1 byte)
//!     _padding2: [u8; 7]     (7 bytes)
//!     guard: u64             (8 bytes)
//! }
//! Total: 32 bytes
//! ```

use core::fmt;

/// Capability - unforgeable token granting rights to a kernel object
///
/// Capabilities are the sole means of accessing kernel objects in KaaL.
/// They cannot be forged by user space and can only be obtained through
/// derivation from existing capabilities or initial kernel setup.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Capability {
    /// Type of the object this capability refers to
    cap_type: CapType,

    /// Padding for alignment
    _padding: [u8; 7],

    /// Pointer to the kernel object
    ///
    /// This is either a physical address or a kernel virtual address
    /// depending on the object type.
    object_ptr: usize,

    /// Access rights for this capability
    rights: CapRights,

    /// Padding for alignment
    _padding2: [u8; 7],

    /// Guard word for CNode addressing
    ///
    /// The guard is prepended to the remaining address bits during
    /// capability lookup in a CNode tree. This allows for more compact
    /// capability address spaces.
    guard: u64,
}

impl Capability {
    /// Create a null capability (empty slot)
    pub const fn null() -> Self {
        Self {
            cap_type: CapType::Null,
            _padding: [0; 7],
            object_ptr: 0,
            rights: CapRights::empty(),
            _padding2: [0; 7],
            guard: 0,
        }
    }

    /// Create a new capability with full rights
    pub const fn new(cap_type: CapType, object_ptr: usize) -> Self {
        Self {
            cap_type,
            _padding: [0; 7],
            object_ptr,
            rights: CapRights::ALL,
            _padding2: [0; 7],
            guard: 0,
        }
    }

    /// Create a capability with specific rights
    pub const fn with_rights(cap_type: CapType, object_ptr: usize, rights: CapRights) -> Self {
        Self {
            cap_type,
            _padding: [0; 7],
            object_ptr,
            rights,
            _padding2: [0; 7],
            guard: 0,
        }
    }

    /// Create a capability with a guard
    pub const fn with_guard(cap_type: CapType, object_ptr: usize, guard: u64) -> Self {
        Self {
            cap_type,
            _padding: [0; 7],
            object_ptr,
            rights: CapRights::ALL,
            _padding2: [0; 7],
            guard,
        }
    }

    /// Get the capability type
    #[inline]
    pub fn cap_type(&self) -> CapType {
        self.cap_type
    }

    /// Get the object pointer
    #[inline]
    pub fn object_ptr(&self) -> usize {
        self.object_ptr
    }

    /// Get the capability rights
    #[inline]
    pub fn rights(&self) -> CapRights {
        self.rights
    }

    /// Get the guard word
    #[inline]
    pub fn guard(&self) -> u64 {
        self.guard
    }

    /// Check if this is a null capability
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self.cap_type, CapType::Null)
    }

    /// Check if capability has a specific right
    #[inline]
    pub fn has_right(&self, right: CapRights) -> bool {
        self.rights.contains(right)
    }

    /// Derive a new capability with reduced rights
    pub fn derive(&self, new_rights: CapRights) -> Result<Self, CapError> {
        // Can only reduce rights, not add new ones
        if !self.rights.contains(new_rights) {
            return Err(CapError::InsufficientRights);
        }

        Ok(Self {
            cap_type: self.cap_type,
            _padding: [0; 7],
            object_ptr: self.object_ptr,
            rights: new_rights,
            _padding2: [0; 7],
            guard: self.guard,
        })
    }

    /// Mint a new capability with a badge (for endpoints)
    pub fn mint(&self, badge: u64) -> Result<Self, CapError> {
        // Only endpoints can be badged
        if !matches!(self.cap_type, CapType::Endpoint) {
            return Err(CapError::InvalidOperation);
        }

        Ok(Self {
            cap_type: self.cap_type,
            _padding: [0; 7],
            object_ptr: self.object_ptr,
            rights: self.rights,
            _padding2: [0; 7],
            guard: badge, // Reuse guard field for badge
        })
    }

    /// Get the badge value (for endpoint/reply capabilities)
    ///
    /// The guard field is reused for badges in endpoint and reply capabilities.
    #[inline]
    pub fn badge(&self) -> u64 {
        self.guard
    }

    /// Set the badge value (for endpoint/reply capabilities)
    ///
    /// The guard field is reused for badges in endpoint and reply capabilities.
    #[inline]
    pub fn set_badge(&mut self, badge: u64) {
        self.guard = badge;
    }

    /// Set the rights for this capability
    #[inline]
    pub fn set_rights(&mut self, rights: CapRights) {
        self.rights = rights;
    }
}

/// Types of kernel objects
///
/// Each object type has specific operations that can be performed through
/// capability invocations.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapType {
    /// Null capability (empty slot)
    Null = 0,

    /// Untyped memory - raw memory that can be retyped into other objects
    UntypedMemory = 1,

    /// Endpoint - synchronous IPC rendezvous point
    Endpoint = 2,

    /// Notification - asynchronous signaling object
    Notification = 3,

    /// TCB (Thread Control Block) - represents a thread of execution
    Tcb = 4,

    /// CNode - capability node (container for capabilities)
    CNode = 5,

    /// VSpace - virtual address space root (page table root)
    VSpace = 6,

    /// Page Table - intermediate page table
    PageTable = 7,

    /// Page - physical memory page
    Page = 8,

    /// IRQ Handler - interrupt handler capability
    IrqHandler = 9,

    /// IRQ Control - IRQ management capability
    IrqControl = 10,

    /// Reply - one-time reply capability for IPC call/reply
    Reply = 11,
}

/// Capability rights (bitflags)
///
/// Rights control what operations can be performed on an object through
/// a capability. Rights can only be reduced (not added) during derivation.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CapRights(u8);

impl CapRights {
    /// Read permission (can read from object)
    pub const READ: Self = Self(0b0001);

    /// Write permission (can write to object)
    pub const WRITE: Self = Self(0b0010);

    /// Grant permission (can transfer capability with full rights)
    pub const GRANT: Self = Self(0b0100);

    /// All rights (read + write + grant)
    pub const ALL: Self = Self(0b0111);

    /// No rights (empty)
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Check if this contains another set of rights
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Get the raw bits
    #[inline]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Create from raw bits
    #[inline]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 0b0111) // Mask to valid bits
    }

    /// Union of two rights
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersection of two rights
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl fmt::Debug for CapRights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = alloc::vec::Vec::new();
        if self.contains(Self::READ) {
            parts.push("READ");
        }
        if self.contains(Self::WRITE) {
            parts.push("WRITE");
        }
        if self.contains(Self::GRANT) {
            parts.push("GRANT");
        }
        if parts.is_empty() {
            write!(f, "NONE")
        } else {
            write!(f, "{}", parts.join(" | "))
        }
    }
}

/// Capability errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapError {
    /// Insufficient rights to perform operation
    InsufficientRights,

    /// Invalid capability (null or wrong type)
    InvalidCapability,

    /// Operation not valid for this capability type
    InvalidOperation,

    /// Capability not found
    NotFound,

    /// Capability slot already occupied
    SlotOccupied,

    /// Invalid argument
    InvalidArgument,

    /// Insufficient memory
    InsufficientMemory,
}

impl fmt::Debug for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Capability")
            .field("type", &self.cap_type)
            .field("object", &format_args!("{:#x}", self.object_ptr))
            .field("rights", &self.rights)
            .field("guard", &format_args!("{:#x}", self.guard))
            .finish()
    }
}

// Compile-time assertions
const _: () = {
    assert!(core::mem::size_of::<Capability>() == 32, "Capability must be 32 bytes");
    assert!(core::mem::align_of::<Capability>() == 8, "Capability must be 8-byte aligned");
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_size() {
        assert_eq!(core::mem::size_of::<Capability>(), 32);
        assert_eq!(core::mem::align_of::<Capability>(), 8);
    }

    #[test]
    fn null_capability() {
        let cap = Capability::null();
        assert!(cap.is_null());
        assert_eq!(cap.object_ptr(), 0);
    }

    #[test]
    fn capability_rights() {
        let cap = Capability::with_rights(CapType::Endpoint, 0x1000, CapRights::READ);
        assert!(cap.has_right(CapRights::READ));
        assert!(!cap.has_right(CapRights::WRITE));
    }

    #[test]
    fn derive_capability() {
        let cap = Capability::new(CapType::Endpoint, 0x1000);
        let derived = cap.derive(CapRights::READ).unwrap();
        assert!(derived.has_right(CapRights::READ));
        assert!(!derived.has_right(CapRights::WRITE));

        // Cannot derive with more rights
        assert!(derived.derive(CapRights::WRITE).is_err());
    }
}
