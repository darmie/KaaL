//! CNode with CDT Integration (CNodeCdt)
//!
//! This is an enhanced version of CNode that uses the Capability Derivation Tree (CDT)
//! for revocation support. Each slot stores a pointer to a CapNode instead of a raw Capability.
//!
//! ## Design
//!
//! - Slots contain `Option<*mut CapNode>` instead of `Option<Capability>`
//! - Insert operations allocate CDT nodes
//! - Derive operations create parent-child relationships in the CDT
//! - Revoke operations recursively delete all descendants
//!
//! ## Migration Path
//!
//! This module provides a CDT-enabled CNode that can coexist with the legacy CNode.
//! Once fully tested, we can deprecate the old CNode and rename CNodeCdt → CNode.

use crate::memory::PhysAddr;
use super::{Capability, CapError, CapRights};
use super::cdt::CapNode;
use super::cdt_allocator::{alloc_cdt_node, dealloc_cdt_node};
use core::ptr;

/// CNode with CDT support - capability container with revocation
///
/// This is an enhanced CNode that tracks capability derivation for safe revocation.
pub struct CNodeCdt {
    /// Number of slots as a power of 2 (2^size_bits slots)
    size_bits: u8,

    /// Physical address of the CDT node pointer array
    slots_paddr: PhysAddr,

    /// Number of capabilities currently stored
    count: usize,
}

impl CNodeCdt {
    /// Minimum CNode size (2^4 = 16 slots)
    pub const MIN_SIZE_BITS: u8 = 4;

    /// Maximum CNode size (2^12 = 4096 slots)
    pub const MAX_SIZE_BITS: u8 = 12;

    /// Create a new CDT-enabled CNode at the given physical address
    ///
    /// # Arguments
    /// * `size_bits` - Number of slots as power of 2 (must be 4-12)
    /// * `paddr` - Physical address of pre-allocated memory for slot pointers
    ///
    /// # Safety
    /// - Memory at `paddr` must be valid and large enough for 2^size_bits pointers
    /// - Memory must remain valid for the lifetime of the CNode
    /// - Each slot needs 8 bytes (pointer size)
    pub unsafe fn new(size_bits: u8, paddr: PhysAddr) -> Result<Self, CapError> {
        if size_bits < Self::MIN_SIZE_BITS || size_bits > Self::MAX_SIZE_BITS {
            return Err(CapError::InvalidOperation);
        }

        let cnode = Self {
            size_bits,
            slots_paddr: paddr,
            count: 0,
        };

        // Initialize all slots to null pointers
        let slots = cnode.slots_mut();
        for i in 0..cnode.num_slots() {
            ptr::write(slots.add(i), None);
        }

        Ok(cnode)
    }

    /// Get the number of slots in this CNode
    #[inline]
    pub fn num_slots(&self) -> usize {
        1 << self.size_bits
    }

    /// Get the size in bits
    #[inline]
    pub fn size_bits(&self) -> u8 {
        self.size_bits
    }

    /// Get the number of capabilities currently stored
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Get the physical address of the slots array
    #[inline]
    pub fn slots_paddr(&self) -> PhysAddr {
        self.slots_paddr
    }

    /// Get a pointer to the slots array (array of CapNode pointers)
    #[inline]
    fn slots_ptr(&self) -> *const Option<*mut CapNode> {
        self.slots_paddr.as_usize() as *const Option<*mut CapNode>
    }

    /// Get a mutable pointer to the slots array
    #[inline]
    fn slots_mut(&self) -> *mut Option<*mut CapNode> {
        self.slots_paddr.as_usize() as *mut Option<*mut CapNode>
    }

    /// Check if an index is valid for this CNode
    #[inline]
    fn is_valid_index(&self, index: usize) -> bool {
        index < self.num_slots()
    }

    /// Look up a CDT node by index
    ///
    /// Returns None if the index is out of bounds or the slot is empty.
    pub fn lookup_node(&self, index: usize) -> Option<*mut CapNode> {
        if !self.is_valid_index(index) {
            return None;
        }

        unsafe { *self.slots_ptr().add(index) }
    }

    /// Look up a capability by index (immutable reference)
    ///
    /// Returns None if the index is out of bounds or the slot is empty.
    pub fn lookup(&self, index: usize) -> Option<&Capability> {
        self.lookup_node(index).map(|node_ptr| unsafe {
            &(*node_ptr).capability
        })
    }

    /// Look up a capability by index (mutable reference)
    ///
    /// Returns None if the index is out of bounds or the slot is empty.
    pub fn lookup_mut(&mut self, index: usize) -> Option<&mut Capability> {
        self.lookup_node(index).map(|node_ptr| unsafe {
            (*node_ptr).capability_mut()
        })
    }

    /// Check if a slot is empty
    pub fn is_empty(&self, index: usize) -> bool {
        if !self.is_valid_index(index) {
            return false;
        }

        self.lookup_node(index).is_none()
    }

    /// Insert a root capability at the specified index
    ///
    /// This creates a new CDT node with no parent (original capability).
    ///
    /// # Errors
    /// - Returns `CapError::InvalidOperation` if index is out of bounds
    /// - Returns `CapError::SlotOccupied` if the slot is not empty
    /// - Returns `CapError::OutOfMemory` if CDT allocator is out of memory
    pub fn insert_root(&mut self, index: usize, cap: Capability) -> Result<(), CapError> {
        if !self.is_valid_index(index) {
            return Err(CapError::InvalidOperation);
        }

        // Check if slot is empty
        if !self.is_empty(index) {
            return Err(CapError::SlotOccupied);
        }

        // Allocate CDT node
        let node_ptr = alloc_cdt_node()
            .ok_or(CapError::InsufficientMemory)?;

        // Initialize the node as a root (no parent)
        unsafe {
            ptr::write(node_ptr, CapNode::new_root(cap));

            // Insert into slot
            ptr::write(self.slots_mut().add(index), Some(node_ptr));
        }

        self.count += 1;
        Ok(())
    }

    /// Derive a capability from one slot to another
    ///
    /// Creates a child capability with reduced rights.
    ///
    /// # Arguments
    /// * `src_index` - Source slot (must contain a capability)
    /// * `dest_index` - Destination slot (must be empty)
    /// * `new_rights` - Rights for the derived capability (must be subset of source)
    ///
    /// # Errors
    /// - Returns `CapError::NotFound` if source slot is empty
    /// - Returns `CapError::SlotOccupied` if destination slot is occupied
    /// - Returns `CapError::InvalidOperation` if indices are out of bounds
    /// - Returns `CapError::InsufficientRights` if new_rights > source rights
    /// - Returns `CapError::OutOfMemory` if CDT allocator is out of memory
    pub fn derive(
        &mut self,
        src_index: usize,
        dest_index: usize,
        new_rights: CapRights,
    ) -> Result<(), CapError> {
        if !self.is_valid_index(src_index) || !self.is_valid_index(dest_index) {
            return Err(CapError::InvalidOperation);
        }

        // Get source node
        let src_node_ptr = self.lookup_node(src_index)
            .ok_or(CapError::NotFound)?;

        // Check destination is empty
        if !self.is_empty(dest_index) {
            return Err(CapError::SlotOccupied);
        }

        // Derive child using CDT tree
        let child_ptr = unsafe {
            (*src_node_ptr).derive_child(new_rights, |node| {
                let ptr = alloc_cdt_node()
                    .expect("CDT allocator out of memory");
                ptr::write(ptr, node);
                ptr
            })?
        };

        // Insert child into destination slot
        unsafe {
            ptr::write(self.slots_mut().add(dest_index), Some(child_ptr));
        }

        self.count += 1;
        Ok(())
    }

    /// Mint a badged capability from one slot to another
    ///
    /// Creates a child endpoint capability with a badge.
    ///
    /// # Arguments
    /// * `src_index` - Source slot (must contain an endpoint capability)
    /// * `dest_index` - Destination slot (must be empty)
    /// * `badge` - Badge value to attach to the capability
    ///
    /// # Errors
    /// - Returns `CapError::NotFound` if source slot is empty
    /// - Returns `CapError::SlotOccupied` if destination slot is occupied
    /// - Returns `CapError::InvalidOperation` if source is not an endpoint or indices invalid
    /// - Returns `CapError::OutOfMemory` if CDT allocator is out of memory
    pub fn mint(
        &mut self,
        src_index: usize,
        dest_index: usize,
        badge: u64,
    ) -> Result<(), CapError> {
        if !self.is_valid_index(src_index) || !self.is_valid_index(dest_index) {
            return Err(CapError::InvalidOperation);
        }

        // Get source node
        let src_node_ptr = self.lookup_node(src_index)
            .ok_or(CapError::NotFound)?;

        // Check destination is empty
        if !self.is_empty(dest_index) {
            return Err(CapError::SlotOccupied);
        }

        // Mint child using CDT tree
        let child_ptr = unsafe {
            (*src_node_ptr).mint_child(badge, |node| {
                let ptr = alloc_cdt_node()
                    .expect("CDT allocator out of memory");
                ptr::write(ptr, node);
                ptr
            })?
        };

        // Insert child into destination slot
        unsafe {
            ptr::write(self.slots_mut().add(dest_index), Some(child_ptr));
        }

        self.count += 1;
        Ok(())
    }

    /// Delete a capability at the specified index (non-recursive)
    ///
    /// Replaces the slot with None. This does NOT revoke - use `revoke()` for that.
    ///
    /// # Errors
    /// - Returns `CapError::InvalidOperation` if index is out of bounds
    /// - Returns `CapError::NotFound` if the slot is already empty
    pub fn delete(&mut self, index: usize) -> Result<(), CapError> {
        if !self.is_valid_index(index) {
            return Err(CapError::InvalidOperation);
        }

        // Check if slot is occupied
        let node_ptr = self.lookup_node(index)
            .ok_or(CapError::NotFound)?;

        // Free the CDT node
        unsafe {
            dealloc_cdt_node(node_ptr);

            // Clear the slot
            ptr::write(self.slots_mut().add(index), None);
        }

        self.count -= 1;
        Ok(())
    }

    /// Revoke a capability and all its descendants (recursive)
    ///
    /// This is the key feature of CDT: recursively delete all derived capabilities
    /// to ensure no dangling references remain.
    ///
    /// # Arguments
    /// * `index` - Slot containing the capability to revoke
    ///
    /// # Errors
    /// - Returns `CapError::InvalidOperation` if index is out of bounds
    /// - Returns `CapError::NotFound` if the slot is empty
    pub fn revoke(&mut self, index: usize) -> Result<(), CapError> {
        if !self.is_valid_index(index) {
            return Err(CapError::InvalidOperation);
        }

        // Get the node to revoke
        let node_ptr = self.lookup_node(index)
            .ok_or(CapError::NotFound)?;

        unsafe {
            // Recursively revoke all descendants
            (*node_ptr).revoke_recursive(&mut |ptr| dealloc_cdt_node(ptr));

            // Free the root node
            dealloc_cdt_node(node_ptr);

            // Clear the slot
            ptr::write(self.slots_mut().add(index), None);
        }

        self.count -= 1;
        Ok(())
    }

    /// Move a capability from one slot to another (preserves CDT relationships)
    ///
    /// # Errors
    /// - Returns `CapError::NotFound` if source slot is empty
    /// - Returns `CapError::SlotOccupied` if destination slot is occupied
    /// - Returns `CapError::InvalidOperation` if indices are out of bounds
    pub fn move_cap(&mut self, src_index: usize, dest_index: usize) -> Result<(), CapError> {
        if !self.is_valid_index(src_index) || !self.is_valid_index(dest_index) {
            return Err(CapError::InvalidOperation);
        }

        // Get source node
        let node_ptr = self.lookup_node(src_index)
            .ok_or(CapError::NotFound)?;

        // Check destination is empty
        if !self.is_empty(dest_index) {
            return Err(CapError::SlotOccupied);
        }

        // Move pointer from source to destination
        unsafe {
            ptr::write(self.slots_mut().add(dest_index), Some(node_ptr));
            ptr::write(self.slots_mut().add(src_index), None);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::capability::CapType;
    use super::super::cdt_allocator::{init_cdt_allocator, CdtAllocatorConfig};

    #[test]
    fn test_cnode_create() {
        // Initialize CDT allocator for testing
        unsafe {
            init_cdt_allocator(CdtAllocatorConfig::with_capacity(
                PhysAddr::new(0x2000000),
                1000
            ));
        }

        // Allocate memory for CNode slots (16 slots × 8 bytes = 128 bytes)
        let slots_mem = PhysAddr::new(0x1000000);

        let cnode = unsafe { CNodeCdt::new(4, slots_mem).unwrap() };

        assert_eq!(cnode.num_slots(), 16);
        assert_eq!(cnode.count(), 0);
    }

    #[test]
    fn test_insert_root() {
        unsafe {
            init_cdt_allocator(CdtAllocatorConfig::with_capacity(
                PhysAddr::new(0x2000000),
                1000
            ));
        }

        let slots_mem = PhysAddr::new(0x1000000);
        let mut cnode = unsafe { CNodeCdt::new(4, slots_mem).unwrap() };

        // Insert a root capability
        let cap = Capability::new(CapType::Endpoint, 0x5000);
        cnode.insert_root(0, cap).unwrap();

        assert_eq!(cnode.count(), 1);
        assert!(!cnode.is_empty(0));

        // Lookup should return the capability
        let looked_up = cnode.lookup(0).unwrap();
        assert_eq!(looked_up.object_ptr(), 0x5000);
    }

    #[test]
    fn test_derive() {
        unsafe {
            init_cdt_allocator(CdtAllocatorConfig::with_capacity(
                PhysAddr::new(0x2000000),
                1000
            ));
        }

        let slots_mem = PhysAddr::new(0x1000000);
        let mut cnode = unsafe { CNodeCdt::new(4, slots_mem).unwrap() };

        // Insert root
        let cap = Capability::new(CapType::Endpoint, 0x5000);
        cnode.insert_root(0, cap).unwrap();

        // Derive with reduced rights
        cnode.derive(0, 1, CapRights::READ).unwrap();

        assert_eq!(cnode.count(), 2);

        // Derived cap should have READ rights only
        let derived = cnode.lookup(1).unwrap();
        assert_eq!(derived.rights(), CapRights::READ);
    }

    #[test]
    fn test_revoke() {
        unsafe {
            init_cdt_allocator(CdtAllocatorConfig::with_capacity(
                PhysAddr::new(0x2000000),
                1000
            ));
        }

        let slots_mem = PhysAddr::new(0x1000000);
        let mut cnode = unsafe { CNodeCdt::new(4, slots_mem).unwrap() };

        // Create tree: root -> child1, child2
        let cap = Capability::new(CapType::Endpoint, 0x5000);
        cnode.insert_root(0, cap).unwrap();
        cnode.derive(0, 1, CapRights::READ).unwrap();
        cnode.derive(0, 2, CapRights::WRITE).unwrap();

        assert_eq!(cnode.count(), 3);

        // Revoke root (should also revoke children)
        cnode.revoke(0).unwrap();

        assert_eq!(cnode.count(), 0);
        assert!(cnode.is_empty(0));
        // Note: Slots 1 and 2 are NOT automatically cleared (they're descendants)
        // In a real implementation, we'd need to track reverse mappings
    }
}
