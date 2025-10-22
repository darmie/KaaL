//! Page table management and virtual memory mapping
//!
//! Provides high-level interface for managing page tables and virtual memory mappings.
//!
//! # Design (seL4-inspired)
//! - Explicit page table allocation (no hidden allocations)
//! - Walking page tables to create mappings
//! - Support for different page sizes (4KB, 2MB, 1GB)

use crate::arch::aarch64::page_table::{
    PageTable, PageTableFlags, PageTableLevel,
};
use crate::memory::{PhysAddr, VirtAddr, alloc_frame};
use crate::memory::{PAGE_SIZE, LARGE_PAGE_SIZE, HUGE_PAGE_SIZE};

/// Page mapping error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingError {
    /// Failed to allocate a frame for page table
    FrameAllocFailed,
    /// Address is not properly aligned
    AddressMisaligned,
    /// Mapping already exists at this address
    AlreadyMapped,
    /// Invalid page table level for operation
    InvalidLevel,
}

/// Page size for mappings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSize {
    /// 4KB page (L3)
    Size4KB = 4096,
    /// 2MB large page (L2 block)
    Size2MB = 2 * 1024 * 1024,
    /// 1GB huge page (L1 block)
    Size1GB = 1024 * 1024 * 1024,
}

impl PageSize {
    /// Get the page table level for this page size
    pub const fn level(&self) -> PageTableLevel {
        match self {
            PageSize::Size4KB => PageTableLevel::L3,
            PageSize::Size2MB => PageTableLevel::L2,
            PageSize::Size1GB => PageTableLevel::L1,
        }
    }

    /// Get the size in bytes
    pub const fn bytes(&self) -> usize {
        *self as usize
    }

    /// Check if address is aligned to this page size
    pub const fn is_aligned(&self, addr: usize) -> bool {
        addr & (self.bytes() - 1) == 0
    }
}

/// Page mapper for managing page tables
pub struct PageMapper {
    /// Root page table (L0)
    root: &'static mut PageTable,
}

impl PageMapper {
    /// Create a new page mapper with the given root page table
    ///
    /// # Safety
    /// - The root table must be properly initialized and aligned
    /// - The root table must have a valid physical address
    pub unsafe fn new(root: &'static mut PageTable) -> Self {
        Self { root }
    }

    /// Map a virtual page to a physical frame
    ///
    /// # Arguments
    /// - `vaddr`: Virtual address to map (must be page-aligned)
    /// - `paddr`: Physical address to map to (must be page-aligned)
    /// - `flags`: Page table entry flags
    /// - `page_size`: Size of page to map (4KB, 2MB, or 1GB)
    ///
    /// # Returns
    /// - `Ok(())` if mapping succeeded
    /// - `Err(MappingError)` if mapping failed
    pub fn map(
        &mut self,
        vaddr: VirtAddr,
        paddr: PhysAddr,
        flags: PageTableFlags,
        page_size: PageSize,
    ) -> Result<(), MappingError> {
        // Check alignment
        if !page_size.is_aligned(vaddr.as_usize()) || !page_size.is_aligned(paddr.as_usize()) {
            return Err(MappingError::AddressMisaligned);
        }

        // Walk to the target level, allocating tables as needed
        let target_level = page_size.level();
        let table = self.walk_to_level(vaddr, target_level, true)?;

        // Set the entry
        let index = target_level.index(vaddr);

        if table.is_valid(index) {
            return Err(MappingError::AlreadyMapped);
        }

        // Adjust flags for block vs page entries:
        // - Block entries (L1/L2): TABLE_OR_PAGE bit must be 0
        // - Page entries (L3): TABLE_OR_PAGE bit must be 1
        let mut entry_flags = flags;
        if page_size != PageSize::Size4KB {
            // This is a block entry (1GB or 2MB), clear TABLE_OR_PAGE bit
            entry_flags.remove(PageTableFlags::TABLE_OR_PAGE);
        }

        table.set_entry(index, paddr, entry_flags);
        Ok(())
    }

    /// Unmap a virtual page
    ///
    /// # Arguments
    /// - `vaddr`: Virtual address to unmap (must be page-aligned)
    /// - `page_size`: Size of page to unmap
    ///
    /// # Returns
    /// - `Ok(())` if unmapping succeeded
    /// - `Err(MappingError)` if unmapping failed
    pub fn unmap(
        &mut self,
        vaddr: VirtAddr,
        page_size: PageSize,
    ) -> Result<(), MappingError> {
        // Check alignment
        if !page_size.is_aligned(vaddr.as_usize()) {
            return Err(MappingError::AddressMisaligned);
        }

        // Walk to the target level
        let target_level = page_size.level();
        let table = self.walk_to_level(vaddr, target_level, false)?;

        // Clear the entry
        let index = target_level.index(vaddr);
        table.clear_entry(index);

        // TODO: Deallocate empty page tables
        // TODO: TLB invalidation

        Ok(())
    }

    /// Translate a virtual address to a physical address
    ///
    /// Walks the page tables to find the physical address mapping.
    pub fn translate(&self, vaddr: VirtAddr) -> Option<PhysAddr> {
        let mut table = self.root as *const PageTable;
        let mut level = PageTableLevel::L0;

        loop {
            let index = level.index(vaddr);
            let entry = unsafe { (*table).entries[index] };

            // Check if entry is valid
            if entry & PageTableFlags::VALID.bits() == 0 {
                return None;
            }

            // Check if this is a block entry (1GB or 2MB)
            let is_table = entry & PageTableFlags::TABLE_OR_PAGE.bits() != 0;
            if !is_table && level.supports_blocks() {
                // Block entry - extract address and add offset
                let block_addr = (entry & 0x0000_FFFF_FFFF_F000) as usize;
                let offset = vaddr.as_usize() & (level.block_size() - 1);
                return Some(PhysAddr::new(block_addr + offset));
            }

            // Move to next level
            match level.next() {
                Some(next_level) => {
                    level = next_level;
                    let next_table_addr = (entry & 0x0000_FFFF_FFFF_F000) as usize;
                    table = next_table_addr as *const PageTable;
                }
                None => {
                    // L3 entry - page mapping
                    let page_addr = (entry & 0x0000_FFFF_FFFF_F000) as usize;
                    let offset = vaddr.as_usize() & (PAGE_SIZE - 1);
                    return Some(PhysAddr::new(page_addr + offset));
                }
            }
        }
    }

    /// Walk page tables to a specific level, optionally allocating tables
    ///
    /// # Arguments
    /// - `vaddr`: Virtual address to walk to
    /// - `target_level`: Target level to walk to
    /// - `allocate`: If true, allocate missing tables; if false, return error
    ///
    /// # Returns
    /// - `Ok(table)`: Reference to the page table at target level
    /// - `Err(MappingError)`: If walking failed
    fn walk_to_level(
        &mut self,
        vaddr: VirtAddr,
        target_level: PageTableLevel,
        allocate: bool,
    ) -> Result<&mut PageTable, MappingError> {
        let mut table = self.root as *mut PageTable;
        let mut level = PageTableLevel::L0;

        while level != target_level {
            let index = level.index(vaddr);
            let entry = unsafe { &mut (*table).entries[index] };

            if *entry & PageTableFlags::VALID.bits() == 0 {
                // Entry is empty
                if !allocate {
                    return Err(MappingError::FrameAllocFailed);
                }

                // Allocate a new page table
                let frame = match alloc_frame() {
                    Some(f) => f,
                    None => {
                        crate::kprintln!("[paging] ERROR: Failed to allocate frame for page table at level {:?}", level);
                        return Err(MappingError::FrameAllocFailed);
                    }
                };
                let phys_addr = frame.phys_addr();

                // Zero the new table
                let new_table = phys_addr.as_usize() as *mut PageTable;
                unsafe { (*new_table).zero() };

                // Set entry to point to new table
                let flags = PageTableFlags::VALID | PageTableFlags::TABLE_OR_PAGE;
                *entry = (phys_addr.as_usize() as u64) | flags.bits();
            }

            // Move to next level
            let next_table_addr = (*entry & 0x0000_FFFF_FFFF_F000) as usize;
            table = next_table_addr as *mut PageTable;
            level = level.next().unwrap();
        }

        Ok(unsafe { &mut *table })
    }

    /// Get the physical address of the root page table
    pub fn root_phys_addr(&self) -> PhysAddr {
        PhysAddr::new(self.root as *const _ as usize)
    }

    /// Debug: Walk page tables and print translation for a virtual address
    pub fn debug_walk(&self, vaddr: VirtAddr) {
        use crate::kprintln;

        kprintln!("  [walk] Translating {:#x}:", vaddr.as_usize());

        let mut table = self.root as *const PageTable;
        let mut level = PageTableLevel::L0;

        loop {
            let index = level.index(vaddr);
            let entry = unsafe { (*table).entries[index] };

            kprintln!("    L{} [{}]: {:#018x}", level as u8, index, entry);

            // Check if entry is valid
            if entry & PageTableFlags::VALID.bits() == 0 {
                kprintln!("      -> INVALID (not mapped)");
                return;
            }

            // Decode flags
            let is_table = entry & PageTableFlags::TABLE_OR_PAGE.bits() != 0;
            let has_af = entry & PageTableFlags::ACCESSED.bits() != 0;
            let uxn = (entry >> 54) & 1;  // UXN bit (user execute never)
            let pxn = (entry >> 53) & 1;  // PXN bit (privileged execute never)

            kprintln!("      -> VALID | {} | AF={} | UXN={} | PXN={}",
                if is_table { "TABLE" } else { "BLOCK" },
                if has_af { "1" } else { "0" },
                uxn, pxn
            );

            // Check if this is a block entry (1GB or 2MB)
            if !is_table && level.supports_blocks() {
                // Block entry - extract address
                let block_addr = (entry & 0x0000_FFFF_FFFF_F000) as usize;
                let offset = vaddr.as_usize() & (level.block_size() - 1);
                let phys_addr = block_addr + offset;
                kprintln!("      -> Translates to {:#x} (block @ {:#x} + offset {:#x})",
                    phys_addr, block_addr, offset);
                return;
            }

            // Move to next level
            match level.next() {
                Some(next_level) => {
                    level = next_level;
                    let next_table_addr = (entry & 0x0000_FFFF_FFFF_F000) as usize;
                    kprintln!("      -> Next table at {:#x}", next_table_addr);
                    table = next_table_addr as *const PageTable;
                }
                None => {
                    // L3 entry - page mapping
                    let page_addr = (entry & 0x0000_FFFF_FFFF_F000) as usize;
                    let offset = vaddr.as_usize() & (PAGE_SIZE - 1);
                    let phys_addr = page_addr + offset;
                    kprintln!("      -> Translates to {:#x} (page @ {:#x} + offset {:#x})",
                        phys_addr, page_addr, offset);
                    return;
                }
            }
        }
    }
}

/// Identity map a memory region (vaddr == paddr)
///
/// Maps a contiguous physical memory region to the same virtual address.
///
/// # Arguments
/// - `mapper`: Page mapper to use
/// - `start`: Start address (both virtual and physical)
/// - `size`: Size of region in bytes
/// - `flags`: Page table entry flags
///
/// # Returns
/// - `Ok(())` if mapping succeeded
/// - `Err(MappingError)` if mapping failed
pub fn identity_map_region(
    mapper: &mut PageMapper,
    start: usize,
    size: usize,
    flags: PageTableFlags,
) -> Result<(), MappingError> {
    let mut addr = start;
    let end = start + size;

    while addr < end {
        let vaddr = VirtAddr::new(addr);
        let paddr = PhysAddr::new(addr);

        // Try to use large pages when possible
        if addr.is_multiple_of(HUGE_PAGE_SIZE) && (end - addr) >= HUGE_PAGE_SIZE {
            // Use 1GB page
            mapper.map(vaddr, paddr, flags, PageSize::Size1GB)?;
            addr += HUGE_PAGE_SIZE;
        } else if addr.is_multiple_of(LARGE_PAGE_SIZE) && (end - addr) >= LARGE_PAGE_SIZE {
            // Use 2MB page
            mapper.map(vaddr, paddr, flags, PageSize::Size2MB)?;
            addr += LARGE_PAGE_SIZE;
        } else {
            // Use 4KB page
            mapper.map(vaddr, paddr, flags, PageSize::Size4KB)?;
            addr += PAGE_SIZE;
        }
    }

    Ok(())
}
