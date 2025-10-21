//! Capability Derivation Tree (CDT)
//!
//! This module implements the Capability Derivation Tree for tracking
//! parent-child relationships between capabilities, enabling safe revocation.
//!
//! ## Terminology - Don't Confuse CapNode with CNode!
//!
//! - **CapNode** (this file): CDT tree node wrapping a SINGLE capability with parent/child pointers
//! - **CNode** (cnode.rs): Capability container holding MULTIPLE capability slots (like an array)
//!
//! Think: CapNode = tree node, CNode = array/table
//!
//! ## Design
//!
//! Following seL4's approach, we track capability derivation using a tree structure:
//! - Original capabilities are roots (no parent)
//! - Derived capabilities are children
//! - Children form a linked list via next_sibling
//!
//! ## Revocation
//!
//! When revoking a capability, we recursively revoke all descendants to ensure
//! no dangling capabilities remain after the parent is revoked.
//!
//! ## Memory
//!
//! Each CDT node adds 24 bytes of overhead (3 pointers):
//! - parent: *mut CapNode
//! - first_child: *mut CapNode
//! - next_sibling: *mut CapNode

use super::capability::{Capability, CapRights, CapError};

/// CDT node wrapping a capability with derivation tree links
#[repr(C)]
pub struct CapNode {
    /// The capability itself (32 bytes)
    pub capability: Capability,

    /// Parent in the derivation tree (None for original/root capabilities)
    pub parent: Option<*mut CapNode>,

    /// First child in the derivation tree
    pub first_child: Option<*mut CapNode>,

    /// Next sibling (children form a linked list under parent)
    pub next_sibling: Option<*mut CapNode>,
}

impl CapNode {
    /// Create a new root capability (no parent)
    ///
    /// # Arguments
    /// * `cap` - The capability to wrap in a CDT node
    ///
    /// # Returns
    /// A new CapNode with no parent or children
    pub const fn new_root(cap: Capability) -> Self {
        CapNode {
            capability: cap,
            parent: None,
            first_child: None,
            next_sibling: None,
        }
    }

    /// Create a child capability node
    ///
    /// # Arguments
    /// * `cap` - The derived capability
    /// * `parent` - Pointer to the parent node
    ///
    /// # Returns
    /// A new CapNode linked to its parent
    pub const fn new_child(cap: Capability, parent: *mut CapNode) -> Self {
        CapNode {
            capability: cap,
            parent: Some(parent),
            first_child: None,
            next_sibling: None,
        }
    }

    /// Check if this is a root capability (no parent)
    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Check if this capability has children
    #[inline]
    pub fn has_children(&self) -> bool {
        self.first_child.is_some()
    }

    /// Get the capability
    #[inline]
    pub fn capability(&self) -> &Capability {
        &self.capability
    }

    /// Get mutable reference to the capability
    #[inline]
    pub fn capability_mut(&mut self) -> &mut Capability {
        &mut self.capability
    }

    /// Derive a child capability with reduced rights
    ///
    /// This creates a new CDT node and links it as a child of this node.
    ///
    /// # Arguments
    /// * `new_rights` - Rights for the derived capability (must be subset of parent)
    /// * `allocator` - Function to allocate a new CDT node
    ///
    /// # Returns
    /// Pointer to the newly created child node, or error if derivation failed
    ///
    /// # Safety
    /// Caller must ensure the allocator returns a valid pointer to uninitialized memory
    pub unsafe fn derive_child<F>(
        &mut self,
        new_rights: CapRights,
        allocator: F,
    ) -> Result<*mut CapNode, CapError>
    where
        F: FnOnce(CapNode) -> *mut CapNode,
    {
        // Derive the capability (validates rights)
        let child_cap = self.capability.derive(new_rights)?;

        // Create child node
        let child_node = CapNode::new_child(child_cap, self as *mut CapNode);

        // Allocate and initialize the child node
        let child_ptr = allocator(child_node);

        // Link child into our children list
        (*child_ptr).next_sibling = self.first_child;
        self.first_child = Some(child_ptr);

        Ok(child_ptr)
    }

    /// Mint a child capability with a badge (for endpoints)
    ///
    /// # Arguments
    /// * `badge` - Badge to add to the endpoint capability
    /// * `allocator` - Function to allocate a new CDT node
    ///
    /// # Returns
    /// Pointer to the newly created child node, or error if minting failed
    ///
    /// # Safety
    /// Caller must ensure the allocator returns a valid pointer to uninitialized memory
    pub unsafe fn mint_child<F>(
        &mut self,
        badge: u64,
        allocator: F,
    ) -> Result<*mut CapNode, CapError>
    where
        F: FnOnce(CapNode) -> *mut CapNode,
    {
        // Mint the capability (validates it's an endpoint)
        let child_cap = self.capability.mint(badge)?;

        // Create child node
        let child_node = CapNode::new_child(child_cap, self as *mut CapNode);

        // Allocate and initialize the child node
        let child_ptr = allocator(child_node);

        // Link child into our children list
        (*child_ptr).next_sibling = self.first_child;
        self.first_child = Some(child_ptr);

        Ok(child_ptr)
    }

    /// Remove a specific child from this node's child list
    ///
    /// # Arguments
    /// * `child_ptr` - Pointer to the child to remove
    ///
    /// # Safety
    /// Caller must ensure child_ptr is actually a child of this node
    unsafe fn remove_child(&mut self, child_ptr: *mut CapNode) {
        let mut prev: Option<*mut CapNode> = None;
        let mut current = self.first_child;

        while let Some(curr_ptr) = current {
            if curr_ptr == child_ptr {
                // Found it - remove from list
                let next = (*curr_ptr).next_sibling;
                if let Some(prev_ptr) = prev {
                    (*prev_ptr).next_sibling = next;
                } else {
                    self.first_child = next;
                }
                return;
            }

            prev = Some(curr_ptr);
            current = (*curr_ptr).next_sibling;
        }
    }

    /// Revoke this capability and all descendants (recursive)
    ///
    /// This performs a depth-first traversal of the derivation tree,
    /// revoking all children before revoking this node.
    ///
    /// # Arguments
    /// * `deallocator` - Function to free a CDT node
    ///
    /// # Safety
    /// - Caller must ensure no other references to this node or its descendants exist
    /// - After this call, the node and all descendants are freed
    /// - Caller is responsible for removing this node from parent's child list if needed
    pub unsafe fn revoke_recursive<F>(
        &mut self,
        deallocator: &mut F,
    ) where
        F: FnMut(*mut CapNode),
    {
        // Revoke all children first (depth-first traversal)
        let mut child = self.first_child;
        while let Some(child_ptr) = child {
            let child_node = &mut *child_ptr;
            let next = child_node.next_sibling;

            // Recursively revoke child and its descendants
            child_node.revoke_recursive(deallocator);

            // Free the child node
            deallocator(child_ptr);

            child = next;
        }

        // Clear children list
        self.first_child = None;

        // Nullify this capability
        self.capability = Capability::null();

        // If we have a parent, remove ourselves from its child list
        if let Some(parent_ptr) = self.parent {
            let parent = &mut *parent_ptr;
            parent.remove_child(self as *mut CapNode);
        }
    }

    /// Count total descendants (children + grandchildren + ...)
    ///
    /// Used for debugging and testing.
    #[cfg(feature = "debug")]
    pub unsafe fn count_descendants(&self) -> usize {
        let mut count = 0;
        let mut child = self.first_child;

        while let Some(child_ptr) = child {
            let child_node = &*child_ptr;
            count += 1; // Count this child
            count += child_node.count_descendants(); // Recursively count its descendants
            child = child_node.next_sibling;
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::capability::CapType;

    /// Simple static allocator for testing (no Vec, no global allocator needed)
    struct TestAllocator {
        buffer: [CapNode; 100],
        next_idx: usize,
    }

    impl TestAllocator {
        fn new() -> Self {
            TestAllocator {
                buffer: [CapNode::new_root(Capability::null()); 100],
                next_idx: 0,
            }
        }

        fn alloc(&mut self, node: CapNode) -> *mut CapNode {
            if self.next_idx >= self.buffer.len() {
                panic!("Test allocator out of memory");
            }
            self.buffer[self.next_idx] = node;
            let ptr = &mut self.buffer[self.next_idx] as *mut CapNode;
            self.next_idx += 1;
            ptr
        }

        fn dealloc(&mut self, _ptr: *mut CapNode) {
            // Simplified: in real allocator, would actually free memory
        }
    }

    #[test]
    fn test_root_creation() {
        let cap = Capability::new(CapType::Endpoint, 0x1000);
        let node = CapNode::new_root(cap);

        assert!(node.is_root());
        assert!(!node.has_children());
    }

    #[test]
    fn test_derive_child() {
        let cap = Capability::new(CapType::Endpoint, 0x1000);
        let mut root = CapNode::new_root(cap);
        let mut alloc = TestAllocator::new();

        unsafe {
            // Derive child with READ rights only
            let child_ptr = root.derive_child(
                CapRights::READ,
                |node| alloc.alloc(node)
            ).expect("Derivation failed");

            assert!(root.has_children());
            assert_eq!(root.first_child, Some(child_ptr));

            // Verify child has reduced rights
            let child = &*child_ptr;
            assert!(!child.is_root());
            assert_eq!(child.parent, Some(&mut root as *mut CapNode));
            assert_eq!(child.capability().rights(), CapRights::READ);
        }
    }

    #[test]
    fn test_revoke_single_child() {
        let cap = Capability::new(CapType::Endpoint, 0x1000);
        let mut root = CapNode::new_root(cap);
        let mut alloc = TestAllocator::new();

        unsafe {
            // Create child
            let child_ptr = root.derive_child(
                CapRights::READ,
                |node| alloc.alloc(node)
            ).unwrap();

            // Revoke the child
            let child = &mut *child_ptr;
            child.revoke_recursive(&mut |ptr| alloc.dealloc(ptr));

            // Child should be nullified
            assert!(child.capability().is_null());

            // Root should no longer have this child
            assert!(!root.has_children());
        }
    }

    #[test]
    fn test_revoke_tree() {
        let cap = Capability::new(CapType::Endpoint, 0x1000);
        let mut root = CapNode::new_root(cap);
        let mut alloc = TestAllocator::new();

        unsafe {
            // Create tree: root -> child1, child2 -> grandchild
            let child1 = root.derive_child(
                CapRights::READ | CapRights::WRITE,
                |node| alloc.alloc(node)
            ).unwrap();

            let child2 = root.derive_child(
                CapRights::READ,
                |node| alloc.alloc(node)
            ).unwrap();

            let grandchild = (*child2).derive_child(
                CapRights::READ,
                |node| alloc.alloc(node)
            ).unwrap();

            // Revoke child2 (should also revoke grandchild)
            (*child2).revoke_recursive(&mut |ptr| alloc.dealloc(ptr));

            // child2 and grandchild should be nullified
            assert!((*child2).capability().is_null());
            assert!((*grandchild).capability().is_null());

            // child1 should still be valid
            assert!(!(*child1).capability().is_null());

            // Root should still have child1 but not child2
            assert!(root.has_children());
            assert_eq!(root.first_child, Some(child1));
        }
    }
}
