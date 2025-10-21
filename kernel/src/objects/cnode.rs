//! CNode (Capability Node) Implementation
//!
//! CNodes are containers for capabilities. They form a tree structure that
//! makes up a thread's capability address space (CSpace).
//!
//! ## Design
//!
//! - Each CNode contains 2^n capability slots (power of 2)
//! - Slots are indexed 0 to (2^n - 1)
//! - Empty slots contain null capabilities
//! - CNodes can be linked to form a tree structure
//!
//! ## Capability Addressing
//!
//! Capabilities are addressed using a path through the CNode tree:
//! ```
//! CSpace Root (CNode)
//!   ├─[0] → Page capability
//!   ├─[1] → TCB capability
//!   ├─[2] → Sub-CNode
//!   │      ├─[0] → Endpoint
//!   │      └─[1] → Endpoint
//!   └─[3] → Endpoint capability
//! ```

use crate::memory::PhysAddr;
use super::{Capability, CapError};
use core::ptr;

/// CNode - a container for capabilities
///
/// CNodes are arrays of capability slots that form a thread's capability
/// address space. Each slot can contain one capability.
pub struct CNode {
    /// Number of slots as a power of 2 (2^size_bits slots)
    size_bits: u8,

    /// Physical address of the capability array
    slots_paddr: PhysAddr,

    /// Number of capabilities currently stored
    count: usize,
}

impl CNode {
    /// Minimum CNode size (2^4 = 16 slots)
    pub const MIN_SIZE_BITS: u8 = 4;

    /// Maximum CNode size (2^12 = 4096 slots)
    pub const MAX_SIZE_BITS: u8 = 12;

    /// Create a new CNode at the given physical address
    ///
    /// # Arguments
    /// * `size_bits` - Number of slots as power of 2 (must be 4-12)
    /// * `paddr` - Physical address of pre-allocated memory for slots
    ///
    /// # Safety
    /// - Memory at `paddr` must be valid and large enough for 2^size_bits capabilities
    /// - Memory must remain valid for the lifetime of the CNode
    pub unsafe fn new(size_bits: u8, paddr: PhysAddr) -> Result<Self, CapError> {
        if !(Self::MIN_SIZE_BITS..=Self::MAX_SIZE_BITS).contains(&size_bits) {
            return Err(CapError::InvalidOperation);
        }

        let cnode = Self {
            size_bits,
            slots_paddr: paddr,
            count: 0,
        };

        // Initialize all slots to null capabilities
        let slots = cnode.slots_mut();
        for i in 0..cnode.num_slots() {
            ptr::write(slots.add(i), Capability::null());
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

    /// Get a pointer to the slots array
    #[inline]
    fn slots_ptr(&self) -> *const Capability {
        self.slots_paddr.as_usize() as *const Capability
    }

    /// Get a mutable pointer to the slots array
    #[inline]
    fn slots_mut(&self) -> *mut Capability {
        self.slots_paddr.as_usize() as *mut Capability
    }

    /// Check if an index is valid for this CNode
    #[inline]
    fn is_valid_index(&self, index: usize) -> bool {
        index < self.num_slots()
    }

    /// Look up a capability by index
    ///
    /// Returns None if the index is out of bounds or the slot is empty.
    pub fn lookup(&self, index: usize) -> Option<&Capability> {
        if !self.is_valid_index(index) {
            return None;
        }

        let cap = unsafe { &*self.slots_ptr().add(index) };
        if cap.is_null() {
            None
        } else {
            Some(cap)
        }
    }

    /// Look up a capability by index (mutable)
    ///
    /// Returns None if the index is out of bounds or the slot is empty.
    pub fn lookup_mut(&mut self, index: usize) -> Option<&mut Capability> {
        if !self.is_valid_index(index) {
            return None;
        }

        let cap = unsafe { &mut *self.slots_mut().add(index) };
        if cap.is_null() {
            None
        } else {
            Some(cap)
        }
    }

    /// Check if a slot is empty
    pub fn is_empty(&self, index: usize) -> bool {
        if !self.is_valid_index(index) {
            return false;
        }

        let cap = unsafe { &*self.slots_ptr().add(index) };
        cap.is_null()
    }

    /// Insert a capability at the specified index
    ///
    /// # Errors
    /// - Returns `CapError::InvalidOperation` if index is out of bounds
    /// - Returns `CapError::SlotOccupied` if the slot is not empty
    pub fn insert(&mut self, index: usize, cap: Capability) -> Result<(), CapError> {
        if !self.is_valid_index(index) {
            return Err(CapError::InvalidOperation);
        }

        // Check if slot is empty
        if !self.is_empty(index) {
            return Err(CapError::SlotOccupied);
        }

        // Insert the capability
        unsafe {
            ptr::write(self.slots_mut().add(index), cap);
        }

        self.count += 1;
        Ok(())
    }

    /// Delete a capability at the specified index
    ///
    /// Replaces the capability with a null capability.
    ///
    /// # Errors
    /// - Returns `CapError::InvalidOperation` if index is out of bounds
    /// - Returns `CapError::NotFound` if the slot is already empty
    pub fn delete(&mut self, index: usize) -> Result<(), CapError> {
        if !self.is_valid_index(index) {
            return Err(CapError::InvalidOperation);
        }

        // Check if slot is occupied
        if self.is_empty(index) {
            return Err(CapError::NotFound);
        }

        // Replace with null capability
        unsafe {
            ptr::write(self.slots_mut().add(index), Capability::null());
        }

        self.count -= 1;
        Ok(())
    }

    /// Move a capability from one slot to another
    ///
    /// The source slot must be occupied, and the destination slot must be empty.
    ///
    /// # Errors
    /// - Returns `CapError::NotFound` if source slot is empty
    /// - Returns `CapError::SlotOccupied` if destination slot is occupied
    /// - Returns `CapError::InvalidOperation` if indices are out of bounds
    pub fn move_cap(&mut self, src_index: usize, dest_index: usize) -> Result<(), CapError> {
        if !self.is_valid_index(src_index) || !self.is_valid_index(dest_index) {
            return Err(CapError::InvalidOperation);
        }

        if self.is_empty(src_index) {
            return Err(CapError::NotFound);
        }

        if !self.is_empty(dest_index) {
            return Err(CapError::SlotOccupied);
        }

        // Copy capability from source to destination
        unsafe {
            let cap = ptr::read(self.slots_ptr().add(src_index));
            ptr::write(self.slots_mut().add(dest_index), cap);
            ptr::write(self.slots_mut().add(src_index), Capability::null());
        }

        Ok(())
    }

    /// Copy a capability from one slot to another
    ///
    /// Unlike move, this leaves the source capability intact.
    ///
    /// # Errors
    /// - Returns `CapError::NotFound` if source slot is empty
    /// - Returns `CapError::SlotOccupied` if destination slot is occupied
    /// - Returns `CapError::InvalidOperation` if indices are out of bounds
    pub fn copy_cap(&mut self, src_index: usize, dest_index: usize) -> Result<(), CapError> {
        if !self.is_valid_index(src_index) || !self.is_valid_index(dest_index) {
            return Err(CapError::InvalidOperation);
        }

        if self.is_empty(src_index) {
            return Err(CapError::NotFound);
        }

        if !self.is_empty(dest_index) {
            return Err(CapError::SlotOccupied);
        }

        // Copy capability from source to destination
        unsafe {
            let cap = ptr::read(self.slots_ptr().add(src_index));
            ptr::write(self.slots_mut().add(dest_index), cap);
        }

        self.count += 1;
        Ok(())
    }

    /// Mutate a capability in place
    ///
    /// This allows deriving a capability with reduced rights or minting
    /// with a badge, replacing the original capability.
    ///
    /// # Errors
    /// - Returns `CapError::NotFound` if slot is empty
    /// - Returns `CapError::InvalidOperation` if index is out of bounds
    pub fn mutate<F>(&mut self, index: usize, f: F) -> Result<(), CapError>
    where
        F: FnOnce(&Capability) -> Result<Capability, CapError>,
    {
        if !self.is_valid_index(index) {
            return Err(CapError::InvalidOperation);
        }

        if self.is_empty(index) {
            return Err(CapError::NotFound);
        }

        unsafe {
            let cap = ptr::read(self.slots_ptr().add(index));
            let new_cap = f(&cap)?;
            ptr::write(self.slots_mut().add(index), new_cap);
        }

        Ok(())
    }

    /// Iterate over all capabilities in this CNode
    pub fn iter(&self) -> CNodeIterator<'_> {
        CNodeIterator {
            cnode: self,
            index: 0,
        }
    }

    /// Find the first empty slot
    ///
    /// Returns None if all slots are occupied.
    pub fn find_empty(&self) -> Option<usize> {
        (0..self.num_slots()).find(|&i| self.is_empty(i))
    }
}

/// Iterator over capabilities in a CNode
pub struct CNodeIterator<'a> {
    cnode: &'a CNode,
    index: usize,
}

impl<'a> Iterator for CNodeIterator<'a> {
    type Item = (usize, &'a Capability);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.cnode.num_slots() {
            let index = self.index;
            self.index += 1;

            if let Some(cap) = self.cnode.lookup(index) {
                return Some((index, cap));
            }
        }
        None
    }
}

impl core::fmt::Debug for CNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CNode")
            .field("size_bits", &self.size_bits)
            .field("num_slots", &self.num_slots())
            .field("count", &self.count)
            .field("slots_paddr", &format_args!("{:#x}", self.slots_paddr.as_usize()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::CapType;

    #[test]
    fn cnode_size() {
        // Allocate memory for a 16-slot CNode
        let mut memory = [Capability::null(); 16];
        let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

        let cnode = unsafe { CNode::new(4, paddr).unwrap() };
        assert_eq!(cnode.num_slots(), 16);
        assert_eq!(cnode.count(), 0);
    }

    #[test]
    fn cnode_insert_lookup() {
        let mut memory = [Capability::null(); 16];
        let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

        let mut cnode = unsafe { CNode::new(4, paddr).unwrap() };

        // Insert a capability
        let cap = Capability::new(CapType::Endpoint, 0x1000);
        cnode.insert(5, cap).unwrap();

        // Look it up
        let found = cnode.lookup(5).unwrap();
        assert_eq!(found.object_ptr(), 0x1000);
        assert_eq!(cnode.count(), 1);
    }

    #[test]
    fn cnode_delete() {
        let mut memory = [Capability::null(); 16];
        let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

        let mut cnode = unsafe { CNode::new(4, paddr).unwrap() };

        let cap = Capability::new(CapType::Endpoint, 0x1000);
        cnode.insert(3, cap).unwrap();
        assert_eq!(cnode.count(), 1);

        cnode.delete(3).unwrap();
        assert_eq!(cnode.count(), 0);
        assert!(cnode.is_empty(3));
    }

    #[test]
    fn cnode_move() {
        let mut memory = [Capability::null(); 16];
        let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

        let mut cnode = unsafe { CNode::new(4, paddr).unwrap() };

        let cap = Capability::new(CapType::Endpoint, 0x1000);
        cnode.insert(2, cap).unwrap();

        cnode.move_cap(2, 8).unwrap();
        assert!(cnode.is_empty(2));
        assert!(!cnode.is_empty(8));
        assert_eq!(cnode.count(), 1);
    }

    #[test]
    fn cnode_copy() {
        let mut memory = [Capability::null(); 16];
        let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

        let mut cnode = unsafe { CNode::new(4, paddr).unwrap() };

        let cap = Capability::new(CapType::Endpoint, 0x1000);
        cnode.insert(1, cap).unwrap();

        cnode.copy_cap(1, 7).unwrap();
        assert!(!cnode.is_empty(1));
        assert!(!cnode.is_empty(7));
        assert_eq!(cnode.count(), 2);
    }
}
