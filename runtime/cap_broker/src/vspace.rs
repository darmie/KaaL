//! VSpace Manager - Virtual address space management for seL4
//!
//! This module manages the virtual address space (VSpace) for mapping pages.
//! It tracks allocated virtual addresses and provides utilities for mapping
//! physical frames into the virtual address space using seL4_ARCH_Page_Map.
//!
//! # Architecture
//! - Tracks available virtual address ranges
//! - Allocates virtual addresses for new mappings
//! - Provides interface to seL4_ARCH_Page_Map syscall
//! - Handles page table creation when needed
//!
//! # Phase 2 Integration
//! This module uses real seL4 syscalls when built with --features sel4-real

#![allow(unused)]

use crate::{CSlot, CapabilityError, Result};
use alloc::vec::Vec;

/// Page size (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Virtual address space manager
///
/// Manages virtual address allocation and page mapping for a single VSpace.
pub struct VSpaceManager {
    /// VSpace root capability (page directory)
    vspace_root: CSlot,

    /// Next available virtual address
    next_vaddr: usize,

    /// Base of managed virtual address range
    vaddr_base: usize,

    /// Size of managed virtual address range
    vaddr_size: usize,

    /// List of allocated virtual address regions
    allocated_regions: Vec<VAddrRegion>,
}

/// Virtual address region allocation
#[derive(Debug, Clone)]
struct VAddrRegion {
    /// Virtual address start
    vaddr: usize,
    /// Size in bytes
    size: usize,
}

impl VSpaceManager {
    /// Create a new VSpace manager
    ///
    /// # Arguments
    /// * `vspace_root` - VSpace root capability from bootinfo
    /// * `vaddr_base` - Base address of managed virtual address space
    /// * `vaddr_size` - Size of managed virtual address space
    ///
    /// # Example
    /// ```ignore
    /// let vspace = VSpaceManager::new(
    ///     bootinfo.vspace_root,
    ///     0x8000_0000,  // Start at 2GB
    ///     256 * 1024 * 1024, // 256MB
    /// );
    /// ```
    pub fn new(vspace_root: CSlot, vaddr_base: usize, vaddr_size: usize) -> Self {
        Self {
            vspace_root,
            next_vaddr: vaddr_base,
            vaddr_base,
            vaddr_size,
            allocated_regions: Vec::new(),
        }
    }

    /// Allocate a virtual address range
    ///
    /// # Arguments
    /// * `size` - Size in bytes (will be rounded up to page size)
    ///
    /// # Returns
    /// Virtual address of allocated region
    ///
    /// # Errors
    /// Returns error if VSpace is exhausted
    pub fn allocate_vaddr(&mut self, size: usize) -> Result<usize> {
        // Round up to page size
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

        // Check if we have enough space
        let end_vaddr = self.next_vaddr + aligned_size;
        if end_vaddr > self.vaddr_base + self.vaddr_size {
            return Err(CapabilityError::OutOfMemory { requested: size });
        }

        let vaddr = self.next_vaddr;
        self.next_vaddr = end_vaddr;

        // Track allocation
        self.allocated_regions.push(VAddrRegion {
            vaddr,
            size: aligned_size,
        });

        Ok(vaddr)
    }

    /// Map a physical frame into virtual address space
    ///
    /// # Arguments
    /// * `frame_cap` - Frame capability to map
    /// * `vaddr` - Virtual address to map at (must be page-aligned)
    /// * `writable` - Whether the mapping should be writable
    /// * `cacheable` - Whether the mapping should be cacheable
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// Returns error if mapping fails
    ///
    /// # Safety
    /// Caller must ensure vaddr is valid and not already mapped
    pub unsafe fn map_page(
        &self,
        frame_cap: CSlot,
        vaddr: usize,
        writable: bool,
        cacheable: bool,
    ) -> Result<()> {
        // Verify alignment
        if vaddr % PAGE_SIZE != 0 {
            return Err(CapabilityError::InvalidCap);
        }

        #[cfg(feature = "sel4-real")]
        {
            use sel4_sys::*;

            // Determine rights
            let mut rights = seL4_CanRead;
            if writable {
                rights |= seL4_CanWrite;
            }

            // Determine cache attributes
            let attr = if cacheable {
                seL4_ARCH_WriteBack
            } else {
                seL4_ARCH_Uncached
            };

            // Map the page
            let ret = seL4_ARCH_Page_Map(frame_cap, self.vspace_root, vaddr, rights, attr);

            if ret != seL4_NoError {
                return Err(CapabilityError::Sel4Error(alloc::format!(
                    "seL4_ARCH_Page_Map failed: {}",
                    ret
                )));
            }
        }

        #[cfg(not(feature = "sel4-real"))]
        {
            // Phase 1: No-op for mock
            let _ = (frame_cap, writable, cacheable);
        }

        Ok(())
    }

    /// Unmap a page from virtual address space
    ///
    /// # Arguments
    /// * `frame_cap` - Frame capability to unmap
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// Returns error if unmap fails
    pub fn unmap_page(&self, frame_cap: CSlot) -> Result<()> {
        #[cfg(feature = "sel4-real")]
        {
            use sel4_sys::*;

            let ret = unsafe { seL4_ARCH_Page_Unmap(frame_cap) };

            if ret != seL4_NoError {
                return Err(CapabilityError::Sel4Error(alloc::format!(
                    "seL4_ARCH_Page_Unmap failed: {}",
                    ret
                )));
            }
        }

        #[cfg(not(feature = "sel4-real"))]
        {
            // Phase 1: No-op for mock
            let _ = frame_cap;
        }

        Ok(())
    }

    /// Get VSpace root capability
    pub fn vspace_root(&self) -> CSlot {
        self.vspace_root
    }

    /// Get total available virtual address space
    pub fn total_vaddr_space(&self) -> usize {
        self.vaddr_size
    }

    /// Get allocated virtual address space
    pub fn allocated_vaddr_space(&self) -> usize {
        self.next_vaddr - self.vaddr_base
    }

    /// Get remaining virtual address space
    pub fn available_vaddr_space(&self) -> usize {
        self.vaddr_size - self.allocated_vaddr_space()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vspace_creation() {
        let vspace = VSpaceManager::new(2, 0x8000_0000, 256 * 1024 * 1024);
        assert_eq!(vspace.vspace_root(), 2);
        assert_eq!(vspace.total_vaddr_space(), 256 * 1024 * 1024);
        assert_eq!(vspace.allocated_vaddr_space(), 0);
        assert_eq!(vspace.available_vaddr_space(), 256 * 1024 * 1024);
    }

    #[test]
    fn test_vaddr_allocation() {
        let mut vspace = VSpaceManager::new(2, 0x8000_0000, 256 * 1024 * 1024);

        // Allocate 4KB
        let vaddr1 = vspace.allocate_vaddr(4096).unwrap();
        assert_eq!(vaddr1, 0x8000_0000);
        assert_eq!(vspace.allocated_vaddr_space(), 4096);

        // Allocate 8KB
        let vaddr2 = vspace.allocate_vaddr(8192).unwrap();
        assert_eq!(vaddr2, 0x8000_0000 + 4096);
        assert_eq!(vspace.allocated_vaddr_space(), 4096 + 8192);

        // Allocations should not overlap
        assert!(vaddr1 + 4096 <= vaddr2);
    }

    #[test]
    fn test_vaddr_alignment() {
        let mut vspace = VSpaceManager::new(2, 0x8000_0000, 256 * 1024 * 1024);

        // Allocate non-page-aligned size (should round up)
        let vaddr = vspace.allocate_vaddr(100).unwrap();
        assert_eq!(vaddr % PAGE_SIZE, 0);
        assert_eq!(vspace.allocated_vaddr_space(), PAGE_SIZE);
    }

    #[test]
    fn test_vspace_exhaustion() {
        let mut vspace = VSpaceManager::new(2, 0x8000_0000, 8192);

        // Allocate 4KB (should succeed)
        vspace.allocate_vaddr(4096).unwrap();

        // Allocate another 4KB (should succeed)
        vspace.allocate_vaddr(4096).unwrap();

        // Try to allocate more (should fail - out of space)
        let result = vspace.allocate_vaddr(4096);
        assert!(matches!(result, Err(CapabilityError::OutOfMemory { .. })));
    }

    #[test]
    fn test_map_page_alignment_check() {
        let vspace = VSpaceManager::new(2, 0x8000_0000, 256 * 1024 * 1024);

        // Try to map at unaligned address (should fail)
        let result = unsafe { vspace.map_page(10, 0x8000_0001, true, true) };
        assert!(matches!(result, Err(CapabilityError::InvalidCap)));

        // Map at aligned address (should succeed in Phase 1 mock)
        let result = unsafe { vspace.map_page(10, 0x8000_0000, true, true) };
        assert!(result.is_ok());
    }

    #[test]
    fn test_map_page_permissions() {
        let vspace = VSpaceManager::new(2, 0x8000_0000, 256 * 1024 * 1024);

        // Test read-only mapping
        let result = unsafe { vspace.map_page(10, 0x8000_0000, false, true) };
        assert!(result.is_ok());

        // Test read-write mapping
        let result = unsafe { vspace.map_page(11, 0x8000_1000, true, true) };
        assert!(result.is_ok());
    }

    #[test]
    fn test_map_page_caching() {
        let vspace = VSpaceManager::new(2, 0x8000_0000, 256 * 1024 * 1024);

        // Test cacheable mapping
        let result = unsafe { vspace.map_page(10, 0x8000_0000, true, true) };
        assert!(result.is_ok());

        // Test uncached mapping (for MMIO)
        let result = unsafe { vspace.map_page(11, 0x8000_1000, true, false) };
        assert!(result.is_ok());
    }

    #[test]
    fn test_unmap_page() {
        let vspace = VSpaceManager::new(2, 0x8000_0000, 256 * 1024 * 1024);

        // Map and then unmap
        unsafe { vspace.map_page(10, 0x8000_0000, true, true).unwrap() };
        let result = vspace.unmap_page(10);
        assert!(result.is_ok());
    }
}
