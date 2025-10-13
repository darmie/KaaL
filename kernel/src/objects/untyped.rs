//! Untyped Memory Implementation
//!
//! Untyped memory objects represent raw physical memory that can be "retyped"
//! into other kernel objects. This is the foundation of KaaL's memory management
//! and capability-based resource allocation.
//!
//! ## Concept
//!
//! In seL4 and KaaL, all kernel objects (TCBs, Endpoints, CNodes, etc.) are
//! created by "retyping" untyped memory. This provides:
//! - **Explicit resource allocation**: No hidden allocations
//! - **Revocation**: All objects derived from untyped can be reclaimed
//! - **Accounting**: Memory usage is tracked via capability tree
//!
//! ## Retyping
//!
//! ```
//! Untyped Memory (1MB)
//!   ├─ TCB (4KB)
//!   ├─ Endpoint (64B)
//!   └─ CNode (2^10 slots = 32KB)
//! ```
//!
//! Once memory is retyped, it can be revoked (destroying all derived objects)
//! and then retyped again into different objects.
//!
//! ## Watermark Allocation
//!
//! Untyped memory uses a simple watermark allocator:
//! - Objects are allocated sequentially from the base address
//! - Watermark tracks the next free byte
//! - Revocation resets the watermark (after destroying children)
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Create untyped memory object
//! let mut untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20)?; // 1MB
//!
//! // Retype into a TCB (requires 4KB = 12 bits)
//! let tcb_paddr = untyped.retype(ObjectType::TCB, 12)?;
//!
//! // Retype into an endpoint (requires 64B = 6 bits)
//! let ep_paddr = untyped.retype(ObjectType::Endpoint, 6)?;
//!
//! // Revoke all children (destroys TCB and Endpoint)
//! untyped.revoke()?;
//! ```

use crate::memory::PhysAddr;
use super::{CapError, CapType};
use alloc::vec::Vec;

/// Untyped Memory - raw memory that can be retyped into kernel objects
///
/// Untyped memory is the root of KaaL's memory management. All kernel objects
/// are created by retyping untyped memory. This provides explicit control over
/// memory allocation and enables revocation.
#[derive(Debug)]
pub struct UntypedMemory {
    /// Physical address of the memory region
    paddr: PhysAddr,

    /// Size in bits (2^size_bits bytes)
    ///
    /// For example:
    /// - size_bits = 12 → 4KB (page)
    /// - size_bits = 20 → 1MB
    /// - size_bits = 30 → 1GB
    size_bits: u8,

    /// Current allocation watermark (bytes from base)
    ///
    /// This tracks the next free byte in the untyped region.
    /// Objects are allocated sequentially starting at paddr.
    watermark: usize,

    /// List of child objects derived from this untyped
    ///
    /// Stores physical addresses of all objects created from this untyped.
    /// Used during revocation to destroy all children.
    children: Vec<PhysAddr>,

    /// Whether this untyped is currently available for retyping
    ///
    /// Set to false during revocation or when fully allocated.
    is_available: bool,
}

impl UntypedMemory {
    /// Create a new untyped memory object
    ///
    /// # Arguments
    ///
    /// * `paddr` - Physical address of the memory region
    /// * `size_bits` - Size as log2 (e.g., 12 = 4KB, 20 = 1MB)
    ///
    /// # Returns
    ///
    /// * `Ok(UntypedMemory)` - New untyped object
    /// * `Err(CapError)` - Invalid size or address
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Create 1MB untyped region
    /// let untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20)?;
    /// ```
    pub fn new(paddr: PhysAddr, size_bits: u8) -> Result<Self, CapError> {
        // Validate size
        if size_bits > 32 {
            return Err(CapError::InvalidArgument);
        }

        // Check alignment: paddr must be aligned to 2^size_bits
        let size = 1usize << size_bits;
        if paddr.as_u64() as usize % size != 0 {
            return Err(CapError::InvalidArgument);
        }

        Ok(Self {
            paddr,
            size_bits,
            watermark: 0,
            children: Vec::new(),
            is_available: true,
        })
    }

    /// Get the physical address
    #[inline]
    pub fn paddr(&self) -> PhysAddr {
        self.paddr
    }

    /// Get the size in bits
    #[inline]
    pub fn size_bits(&self) -> u8 {
        self.size_bits
    }

    /// Get the size in bytes
    #[inline]
    pub fn size(&self) -> usize {
        1 << self.size_bits
    }

    /// Get the number of free bytes remaining
    #[inline]
    pub fn free_bytes(&self) -> usize {
        self.size().saturating_sub(self.watermark)
    }

    /// Check if this untyped is available for allocation
    #[inline]
    pub fn is_available(&self) -> bool {
        self.is_available
    }

    /// Get the number of children
    #[inline]
    pub fn num_children(&self) -> usize {
        self.children.len()
    }

    /// Retype untyped memory into a specific object type
    ///
    /// This allocates a new object from the untyped memory region.
    /// The object is allocated at the current watermark position.
    ///
    /// # Arguments
    ///
    /// * `obj_type` - Type of object to create
    /// * `size_bits` - Size of the object (log2 bytes)
    ///
    /// # Returns
    ///
    /// * `Ok(PhysAddr)` - Physical address of the new object
    /// * `Err(CapError)` - Insufficient space or invalid arguments
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Retype into 4KB TCB
    /// let tcb_addr = untyped.retype(ObjectType::TCB, 12)?;
    /// ```
    pub fn retype(&mut self, obj_type: CapType, size_bits: u8) -> Result<PhysAddr, CapError> {
        if !self.is_available {
            return Err(CapError::InvalidOperation);
        }

        // Calculate object size
        let obj_size = 1usize << size_bits;

        // Check if we have enough space
        if self.watermark + obj_size > self.size() {
            return Err(CapError::InsufficientMemory);
        }

        // Validate object type and size
        self.validate_retype(obj_type, size_bits)?;

        // Calculate physical address for new object
        let obj_paddr = PhysAddr::new((self.paddr.as_u64() + self.watermark as u64) as usize);

        // Align to object size
        let alignment = obj_size;
        let aligned_watermark = (self.watermark + alignment - 1) & !(alignment - 1);

        // Check aligned allocation still fits
        if aligned_watermark + obj_size > self.size() {
            return Err(CapError::InsufficientMemory);
        }

        let aligned_paddr = PhysAddr::new((self.paddr.as_u64() + aligned_watermark as u64) as usize);

        // Update watermark
        self.watermark = aligned_watermark + obj_size;

        // Record child
        self.children.push(aligned_paddr);

        Ok(aligned_paddr)
    }

    /// Validate retype parameters
    fn validate_retype(&self, obj_type: CapType, size_bits: u8) -> Result<(), CapError> {
        // Get minimum size for object type
        let min_size_bits = match obj_type {
            CapType::Null => return Err(CapError::InvalidArgument),
            CapType::UntypedMemory => size_bits, // Any size for nested untyped
            CapType::Tcb => 12,                    // 4KB minimum
            CapType::Endpoint => 6,                // 64B minimum
            CapType::Notification => 6,            // 64B minimum
            CapType::CNode => 6,                   // 64B minimum (1 slot)
            CapType::VSpace => 12,                 // 4KB (page table)
            CapType::Page => 12,                   // 4KB minimum
            CapType::PageTable => 12,              // 4KB
            CapType::IrqHandler => 0,              // Zero-size (just metadata)
            CapType::IrqControl => 0,              // Zero-size
        };

        if size_bits < min_size_bits {
            return Err(CapError::InvalidArgument);
        }

        // Check maximum size (1GB limit for sanity)
        if size_bits > 30 {
            return Err(CapError::InvalidArgument);
        }

        Ok(())
    }

    /// Revoke all children (reclaim memory)
    ///
    /// This destroys all objects derived from this untyped and resets
    /// the watermark to 0, making the full memory region available again.
    ///
    /// # Safety
    ///
    /// This is a destructive operation! All capabilities to child objects
    /// become invalid. The caller must ensure no references to child objects
    /// exist before calling this.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All children revoked successfully
    /// * `Err(CapError)` - Revocation failed
    pub unsafe fn revoke(&mut self) -> Result<(), CapError> {
        if !self.is_available {
            return Err(CapError::InvalidOperation);
        }

        // Mark as unavailable during revocation
        self.is_available = false;

        // TODO: Implement actual object destruction
        // For each child:
        // 1. Identify object type at that address
        // 2. Call object-specific destructor
        // 3. Clear memory (for security)

        // For now, just clear the children list and reset watermark
        self.children.clear();
        self.watermark = 0;

        // Make available again
        self.is_available = true;

        Ok(())
    }

    /// Split this untyped into smaller untyped objects
    ///
    /// This creates multiple smaller untyped regions from this larger one.
    /// Useful for delegating memory management.
    ///
    /// # Arguments
    ///
    /// * `split_size_bits` - Size of each split piece
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<UntypedMemory>)` - Vector of new untyped objects
    /// * `Err(CapError)` - Split failed
    pub fn split(&mut self, split_size_bits: u8) -> Result<Vec<UntypedMemory>, CapError> {
        if split_size_bits >= self.size_bits {
            return Err(CapError::InvalidArgument);
        }

        if !self.is_available || self.watermark > 0 {
            return Err(CapError::InvalidOperation);
        }

        let split_size = 1usize << split_size_bits;
        let num_splits = self.size() / split_size;

        let mut splits = Vec::new();
        for i in 0..num_splits {
            let split_paddr = PhysAddr::new((self.paddr.as_u64() + (i * split_size) as u64) as usize);
            splits.push(UntypedMemory::new(split_paddr, split_size_bits)?);
        }

        // Mark this untyped as fully allocated
        self.watermark = self.size();
        self.is_available = false;

        Ok(splits)
    }

    /// Check if a physical address is within this untyped region
    #[inline]
    pub fn contains(&self, paddr: PhysAddr) -> bool {
        let start = self.paddr.as_u64();
        let end = start + self.size() as u64;
        let addr = paddr.as_u64();
        addr >= start && addr < end
    }

    /// Get iterator over children
    pub fn children(&self) -> impl Iterator<Item = PhysAddr> + '_ {
        self.children.iter().copied()
    }
}

/// Object type for retyping (simplified version of CapType)
///
/// This is used during retyping to specify what kind of object to create.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    UntypedMemory,
    TCB,
    Endpoint,
    Notification,
    CNode,
    VSpace,
    Page,
    PageTable,
}

impl ObjectType {
    /// Get the minimum size in bits for this object type
    pub const fn min_size_bits(self) -> u8 {
        match self {
            ObjectType::UntypedMemory => 0, // Can be any size
            ObjectType::TCB => 12,           // 4KB
            ObjectType::Endpoint => 6,       // 64B
            ObjectType::Notification => 6,   // 64B
            ObjectType::CNode => 6,          // 64B (1 slot)
            ObjectType::VSpace => 12,        // 4KB
            ObjectType::Page => 12,          // 4KB
            ObjectType::PageTable => 12,     // 4KB
        }
    }

    /// Convert to CapType
    pub const fn to_cap_type(self) -> CapType {
        match self {
            ObjectType::UntypedMemory => CapType::UntypedMemory,
            ObjectType::TCB => CapType::Tcb,
            ObjectType::Endpoint => CapType::Endpoint,
            ObjectType::Notification => CapType::Notification,
            ObjectType::CNode => CapType::CNode,
            ObjectType::VSpace => CapType::VSpace,
            ObjectType::Page => CapType::Page,
            ObjectType::PageTable => CapType::PageTable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_untyped_creation() {
        let untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20).unwrap();
        assert_eq!(untyped.size_bits(), 20);
        assert_eq!(untyped.size(), 1024 * 1024); // 1MB
        assert_eq!(untyped.free_bytes(), 1024 * 1024);
        assert!(untyped.is_available());
    }

    #[test]
    fn test_untyped_alignment() {
        // Aligned address should work
        assert!(UntypedMemory::new(PhysAddr::new(0x100000), 20).is_ok());

        // Unaligned address should fail
        assert!(UntypedMemory::new(PhysAddr::new(0x100001), 20).is_err());
    }

    #[test]
    fn test_retype_tcb() {
        let mut untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20).unwrap();

        // Retype into TCB (4KB)
        let tcb_addr = untyped.retype(CapType::Tcb, 12).unwrap();
        assert_eq!(tcb_addr, PhysAddr::new(0x50000000));
        assert_eq!(untyped.num_children(), 1);
        assert_eq!(untyped.free_bytes(), 1024 * 1024 - 4096);
    }

    #[test]
    fn test_retype_multiple() {
        let mut untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20).unwrap();

        // Allocate multiple objects
        let tcb1 = untyped.retype(CapType::Tcb, 12).unwrap();
        let ep1 = untyped.retype(CapType::Endpoint, 6).unwrap();
        let tcb2 = untyped.retype(CapType::Tcb, 12).unwrap();

        assert_eq!(untyped.num_children(), 3);
        assert!(tcb1 < ep1);
        assert!(ep1 < tcb2);
    }

    #[test]
    fn test_retype_exhaustion() {
        let mut untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 12).unwrap(); // 4KB

        // Fill with 64B endpoints
        for _ in 0..64 {
            untyped.retype(CapType::Endpoint, 6).unwrap();
        }

        // Next allocation should fail
        assert!(untyped.retype(CapType::Endpoint, 6).is_err());
    }

    #[test]
    fn test_revoke() {
        let mut untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20).unwrap();

        // Allocate some objects
        untyped.retype(CapType::Tcb, 12).unwrap();
        untyped.retype(CapType::Endpoint, 6).unwrap();
        assert_eq!(untyped.num_children(), 2);

        // Revoke
        unsafe {
            untyped.revoke().unwrap();
        }

        assert_eq!(untyped.num_children(), 0);
        assert_eq!(untyped.free_bytes(), 1024 * 1024);
        assert!(untyped.is_available());
    }

    #[test]
    fn test_contains() {
        let untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20).unwrap();

        assert!(untyped.contains(PhysAddr::new(0x50000000)));
        assert!(untyped.contains(PhysAddr::new(0x50000001)));
        assert!(untyped.contains(PhysAddr::new(0x500FFFFF)));
        assert!(!untyped.contains(PhysAddr::new(0x50100000)));
        assert!(!untyped.contains(PhysAddr::new(0x4FFFFFFF)));
    }
}
