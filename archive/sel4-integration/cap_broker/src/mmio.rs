//! MMIO Mapping - Memory-Mapped I/O region management
//!
//! This module handles mapping physical device memory (MMIO) into virtual
//! address space using seL4 frame capabilities.
//!
//! # Phase 1 vs Phase 2
//! - Phase 1: Returns mock mapped regions without actual mapping
//! - Phase 2: Uses seL4_Untyped_Retype and seL4_ARCH_Page_Map for real mapping

use crate::{CSlot, CapabilityError, MappedRegion, Result};

// TODO PHASE 2: Import real seL4 constants
// use sel4_platform::adapter::{seL4_ARCH_4KPage, seL4_CanRead, seL4_CanWrite, seL4_ARCH_Uncached};

/// Page size (4KB)
pub const PAGE_SIZE: usize = 4096;

/// MMIO mapper - handles physical to virtual memory mapping
pub struct MmioMapper {
    /// Next available virtual address for MMIO
    next_vaddr: usize,

    /// Base virtual address for MMIO region
    mmio_base: usize,

    /// Size of MMIO region
    mmio_size: usize,
}

impl MmioMapper {
    /// Create a new MMIO mapper
    ///
    /// # Arguments
    /// * `base` - Base virtual address for MMIO mappings
    /// * `size` - Total size available for MMIO
    pub fn new(base: usize, size: usize) -> Self {
        Self {
            next_vaddr: base,
            mmio_base: base,
            mmio_size: size,
        }
    }

    /// Map a physical MMIO region into virtual memory
    ///
    /// # Arguments
    /// * `paddr` - Physical address of the device region
    /// * `size` - Size of the region in bytes
    /// * `cspace_allocator` - For allocating frame capability slots
    /// * `untyped_cap` - Untyped capability covering this physical region
    /// * `vspace_root` - Root VSpace capability
    ///
    /// # Returns
    /// MappedRegion with virtual address where the region is mapped
    ///
    /// # Errors
    /// Returns error if:
    /// - Out of virtual address space
    /// - Cannot allocate capability slots
    /// - seL4 retype or map operations fail
    pub fn map_region(
        &mut self,
        paddr: usize,
        size: usize,
        cspace_allocator: &mut dyn FnMut() -> Result<CSlot>,
        untyped_cap: CSlot,
        vspace_root: CSlot,
        cspace_root: CSlot,
    ) -> Result<MappedRegion> {
        // Align to page boundaries
        let start_offset = paddr % PAGE_SIZE;
        let aligned_paddr = paddr - start_offset;
        let aligned_size = ((size + start_offset + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;

        // Check if we have enough virtual address space
        if self.next_vaddr + aligned_size > self.mmio_base + self.mmio_size {
            return Err(CapabilityError::OutOfMemory {
                requested: aligned_size,
            });
        }

        let vaddr = self.next_vaddr;
        let num_pages = aligned_size / PAGE_SIZE;

        // TODO PHASE 2: Implement actual frame mapping
        // For Phase 1, we just return a mock mapped region
        #[cfg(not(feature = "runtime"))]
        {
            self.next_vaddr += aligned_size;
            return Ok(MappedRegion {
                vaddr: vaddr + start_offset,
                paddr,
                size,
            });
        }

        // PHASE 2: Real implementation
        #[cfg(feature = "runtime")]
        {
            for i in 0..num_pages {
                // Allocate capability slot for frame
                let frame_cap = cspace_allocator()?;

                // Retype untyped to frame
                unsafe {
                    let ret = sel4_platform::adapter::seL4_Untyped_Retype(
                        untyped_cap as u64,
                        sel4_platform::adapter::seL4_ARCH_4KPage as u64,
                        0, // size_bits (0 for 4K pages)
                        cspace_root as u64, // CSpace root for object creation
                        0, // node_index
                        0, // node_depth
                        frame_cap as u64,
                        1, // num_objects
                    );

                    if ret != sel4_platform::adapter::seL4_NoError {
                        return Err(CapabilityError::Sel4Error(alloc::format!(
                            "seL4_Untyped_Retype failed: {}",
                            ret
                        )));
                    }
                }

                // Map frame into VSpace
                unsafe {
                    let ret = sel4_platform::adapter::seL4_ARCH_Page_Map(
                        frame_cap as u64,
                        vspace_root as u64,
                        (vaddr + i * PAGE_SIZE) as u64,
                        sel4_platform::adapter::seL4_CanRead | sel4_platform::adapter::seL4_CanWrite,
                        sel4_platform::adapter::seL4_ARCH_Uncached, // Important for MMIO!
                    );

                    if ret != sel4_platform::adapter::seL4_NoError {
                        return Err(CapabilityError::Sel4Error(alloc::format!(
                            "seL4_ARCH_Page_Map failed: {}",
                            ret
                        )));
                    }
                }
            }

            self.next_vaddr += aligned_size;

            Ok(MappedRegion {
                vaddr: vaddr + start_offset,
                paddr,
                size,
            })
        }
    }

    /// Unmap a previously mapped MMIO region
    ///
    /// TODO PHASE 2: Implement actual unmapping
    pub fn unmap_region(&mut self, _region: &MappedRegion) -> Result<()> {
        // Phase 1: No-op
        // Phase 2: Use seL4_ARCH_Page_Unmap and delete frame capabilities
        Ok(())
    }

    /// Get the next available virtual address
    pub fn next_vaddr(&self) -> usize {
        self.next_vaddr
    }

    /// Get remaining virtual address space
    pub fn available_space(&self) -> usize {
        (self.mmio_base + self.mmio_size) - self.next_vaddr
    }
}

/// Helper to calculate number of pages needed
pub fn pages_needed(size: usize) -> usize {
    (size + PAGE_SIZE - 1) / PAGE_SIZE
}

/// Helper to align address down to page boundary
pub fn align_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

/// Helper to align address up to page boundary
pub fn align_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Helper to check if address is page-aligned
pub fn is_aligned(addr: usize) -> bool {
    addr % PAGE_SIZE == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmio_mapper_creation() {
        let mapper = MmioMapper::new(0x2000_0000, 256 * 1024 * 1024);
        assert_eq!(mapper.next_vaddr(), 0x2000_0000);
        assert_eq!(mapper.available_space(), 256 * 1024 * 1024);
    }

    #[test]
    fn test_page_alignment() {
        assert_eq!(align_down(0x1234), 0x1000);
        assert_eq!(align_up(0x1234), 0x2000);
        assert_eq!(align_up(0x1000), 0x1000);
        assert!(is_aligned(0x1000));
        assert!(!is_aligned(0x1234));
    }

    #[test]
    fn test_pages_needed() {
        assert_eq!(pages_needed(4096), 1);
        assert_eq!(pages_needed(4097), 2);
        assert_eq!(pages_needed(8192), 2);
        assert_eq!(pages_needed(100), 1);
    }

    #[test]
    fn test_map_region_phase1() {
        let mut mapper = MmioMapper::new(0x2000_0000, 1024 * 1024);

        // Mock allocator
        let mut next_cap = 100;
        let mut allocator = || {
            let cap = next_cap;
            next_cap += 1;
            Ok(cap)
        };

        // Map a 64KB region
        let region = mapper
            .map_region(
                0xFEBC0000,
                65536,
                &mut allocator,
                50, // untyped_cap
                10, // vspace_root
                11, // cspace_root
            )
            .unwrap();

        assert_eq!(region.paddr, 0xFEBC0000);
        assert_eq!(region.size, 65536);
        assert!(region.vaddr >= 0x2000_0000);
    }

    #[test]
    fn test_map_region_unaligned() {
        let mut mapper = MmioMapper::new(0x2000_0000, 1024 * 1024);

        let mut next_cap = 100;
        let mut allocator = || {
            let cap = next_cap;
            next_cap += 1;
            Ok(cap)
        };

        // Map unaligned region (starts at 0x100)
        let region = mapper
            .map_region(0xFEBC0100, 4000, &mut allocator, 50, 10, 11)
            .unwrap();

        assert_eq!(region.paddr, 0xFEBC0100);
        assert_eq!(region.size, 4000);

        // Virtual address should preserve offset within page
        assert_eq!(region.vaddr % PAGE_SIZE, 0x100);
    }

    #[test]
    fn test_map_multiple_regions() {
        let mut mapper = MmioMapper::new(0x2000_0000, 1024 * 1024);

        let mut next_cap = 100;
        let mut allocator = || {
            let cap = next_cap;
            next_cap += 1;
            Ok(cap)
        };

        // Map first region
        let region1 = mapper
            .map_region(0xFEBC0000, 4096, &mut allocator, 50, 10, 11)
            .unwrap();

        // Map second region
        let region2 = mapper
            .map_region(0xFEBD0000, 8192, &mut allocator, 51, 10, 11)
            .unwrap();

        // Regions should not overlap
        assert!(region1.vaddr + region1.size <= region2.vaddr);
    }

    #[test]
    fn test_out_of_virtual_space() {
        let mut mapper = MmioMapper::new(0x2000_0000, 4096); // Only 1 page

        let mut next_cap = 100;
        let mut allocator = || {
            let cap = next_cap;
            next_cap += 1;
            Ok(cap)
        };

        // Try to map 2 pages
        let result = mapper.map_region(0xFEBC0000, 8192, &mut allocator, 50, 10, 11);

        assert!(matches!(result, Err(CapabilityError::OutOfMemory { .. })));
    }
}
