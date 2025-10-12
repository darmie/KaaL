//! ARM64 page table implementation
//!
//! Implements the ARMv8-A 4-level page table structure:
//! - L0 (PGD): 512GB per entry
//! - L1 (PUD): 1GB per entry
//! - L2 (PMD): 2MB per entry
//! - L3 (PTE): 4KB per entry
//!
//! # Design (seL4-inspired)
//! - Separate TTBR0 (user) and TTBR1 (kernel) translation tables
//! - 48-bit virtual address space (256TB)
//! - Type-safe page table entries with bitflags

use bitflags::bitflags;
use crate::memory::{PhysAddr, VirtAddr, PAGE_SIZE};

/// Number of entries in a page table (512 entries for 4KB pages)
pub const ENTRIES_PER_TABLE: usize = 512;

/// Page table entry type
pub type PageTableEntry = u64;

bitflags! {
    /// Page table entry flags (ARMv8-A descriptor format)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageTableFlags: u64 {
        /// Valid entry (descriptor type)
        const VALID         = 1 << 0;

        /// Page/Block descriptor (0 = block, 1 = table or page)
        const TABLE_OR_PAGE = 1 << 1;

        // Memory attributes (AttrIndx[2:0])
        const ATTR_INDEX_0  = 0 << 2;
        const ATTR_INDEX_1  = 1 << 2;
        const ATTR_INDEX_2  = 2 << 2;
        const ATTR_INDEX_3  = 3 << 2;
        const ATTR_INDEX_4  = 4 << 2;
        const ATTR_INDEX_5  = 5 << 2;
        const ATTR_INDEX_6  = 6 << 2;
        const ATTR_INDEX_7  = 7 << 2;

        // Access permissions (AP[2:1])
        const AP_RW_EL1     = 0 << 6;  // Read/write, EL1 only
        const AP_RW_ALL     = 1 << 6;  // Read/write, all ELs
        const AP_RO_EL1     = 2 << 6;  // Read-only, EL1 only
        const AP_RO_ALL     = 3 << 6;  // Read-only, all ELs

        // Shareability (SH[1:0])
        const NON_SHAREABLE = 0 << 8;
        const OUTER_SHARE   = 2 << 8;
        const INNER_SHARE   = 3 << 8;

        /// Access flag (must be 1 to avoid access faults)
        const ACCESSED      = 1 << 10;

        /// Not global (nG)
        const NOT_GLOBAL    = 1 << 11;

        // Execution permissions
        const UXN           = 1 << 54; // Unprivileged execute never
        const PXN           = 1 << 53; // Privileged execute never

        // Common combinations

        /// Normal memory, cacheable
        const NORMAL        = Self::ATTR_INDEX_0.bits();

        /// Device memory (MMIO)
        const DEVICE        = Self::ATTR_INDEX_1.bits();

        /// Kernel read/write data
        const KERNEL_DATA   = Self::VALID.bits()
                            | Self::TABLE_OR_PAGE.bits()
                            | Self::AP_RW_EL1.bits()
                            | Self::ACCESSED.bits()
                            | Self::INNER_SHARE.bits()
                            | Self::NORMAL.bits()
                            | Self::UXN.bits()
                            | Self::PXN.bits();

        /// Kernel read-only data
        const KERNEL_RODATA = Self::VALID.bits()
                            | Self::TABLE_OR_PAGE.bits()
                            | Self::AP_RO_EL1.bits()
                            | Self::ACCESSED.bits()
                            | Self::INNER_SHARE.bits()
                            | Self::NORMAL.bits()
                            | Self::UXN.bits()
                            | Self::PXN.bits();

        /// Kernel executable code
        const KERNEL_CODE   = Self::VALID.bits()
                            | Self::TABLE_OR_PAGE.bits()
                            | Self::AP_RO_EL1.bits()
                            | Self::ACCESSED.bits()
                            | Self::INNER_SHARE.bits()
                            | Self::NORMAL.bits()
                            | Self::UXN.bits();

        /// Device/MMIO mapping
        const KERNEL_DEVICE = Self::VALID.bits()
                            | Self::TABLE_OR_PAGE.bits()
                            | Self::AP_RW_EL1.bits()
                            | Self::ACCESSED.bits()
                            | Self::OUTER_SHARE.bits()
                            | Self::DEVICE.bits()
                            | Self::UXN.bits()
                            | Self::PXN.bits();
    }
}

/// Page table (aligned to 4KB)
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; ENTRIES_PER_TABLE],
}

impl PageTable {
    /// Create a new empty page table
    pub const fn new() -> Self {
        Self {
            entries: [0; ENTRIES_PER_TABLE],
        }
    }

    /// Clear all entries
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = 0;
        }
    }

    /// Get the physical address of this page table
    pub fn phys_addr(&self) -> PhysAddr {
        PhysAddr::new(self as *const _ as usize)
    }

    /// Set an entry to point to a physical address with flags
    pub fn set_entry(&mut self, index: usize, addr: PhysAddr, flags: PageTableFlags) {
        debug_assert!(index < ENTRIES_PER_TABLE);
        debug_assert!(addr.is_aligned(PAGE_SIZE));

        // Clear lower 12 bits (reserved for flags) and upper 16 bits
        let addr_bits = addr.as_usize() & 0x0000_FFFF_FFFF_F000;
        self.entries[index] = addr_bits as u64 | flags.bits();
    }

    /// Get the physical address from an entry (if valid)
    pub fn get_addr(&self, index: usize) -> Option<PhysAddr> {
        debug_assert!(index < ENTRIES_PER_TABLE);

        let entry = self.entries[index];
        if entry & PageTableFlags::VALID.bits() == 0 {
            return None;
        }

        // Extract physical address (bits [47:12])
        let addr = (entry & 0x0000_FFFF_FFFF_F000) as usize;
        Some(PhysAddr::new(addr))
    }

    /// Check if an entry is valid
    pub fn is_valid(&self, index: usize) -> bool {
        debug_assert!(index < ENTRIES_PER_TABLE);
        self.entries[index] & PageTableFlags::VALID.bits() != 0
    }

    /// Clear an entry
    pub fn clear_entry(&mut self, index: usize) {
        debug_assert!(index < ENTRIES_PER_TABLE);
        self.entries[index] = 0;
    }

    /// Get entry flags
    pub fn get_flags(&self, index: usize) -> PageTableFlags {
        debug_assert!(index < ENTRIES_PER_TABLE);
        PageTableFlags::from_bits_truncate(self.entries[index])
    }
}

/// Page table level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTableLevel {
    /// Level 0: 512GB per entry (PGD)
    L0 = 0,
    /// Level 1: 1GB per entry (PUD)
    L1 = 1,
    /// Level 2: 2MB per entry (PMD)
    L2 = 2,
    /// Level 3: 4KB per entry (PTE)
    L3 = 3,
}

impl PageTableLevel {
    /// Get the shift for this level
    pub const fn shift(self) -> usize {
        match self {
            PageTableLevel::L0 => 39, // bits [47:39]
            PageTableLevel::L1 => 30, // bits [38:30]
            PageTableLevel::L2 => 21, // bits [29:21]
            PageTableLevel::L3 => 12, // bits [20:12]
        }
    }

    /// Get the size of a block/page at this level
    pub const fn block_size(self) -> usize {
        1 << self.shift()
    }

    /// Get the table index for a virtual address at this level
    pub const fn index(self, vaddr: VirtAddr) -> usize {
        (vaddr.as_usize() >> self.shift()) & 0x1FF // 9 bits = 512 entries
    }

    /// Check if this level supports block mappings
    pub const fn supports_blocks(self) -> bool {
        matches!(self, PageTableLevel::L1 | PageTableLevel::L2)
    }

    /// Get the next level (if any)
    pub const fn next(self) -> Option<PageTableLevel> {
        match self {
            PageTableLevel::L0 => Some(PageTableLevel::L1),
            PageTableLevel::L1 => Some(PageTableLevel::L2),
            PageTableLevel::L2 => Some(PageTableLevel::L3),
            PageTableLevel::L3 => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_table_size() {
        assert_eq!(core::mem::size_of::<PageTable>(), 4096);
        assert_eq!(core::mem::align_of::<PageTable>(), 4096);
    }

    #[test]
    fn test_page_table_flags() {
        let flags = PageTableFlags::KERNEL_DATA;
        assert!(flags.contains(PageTableFlags::VALID));
        assert!(flags.contains(PageTableFlags::ACCESSED));
    }

    #[test]
    fn test_page_table_level() {
        assert_eq!(PageTableLevel::L0.shift(), 39);
        assert_eq!(PageTableLevel::L1.shift(), 30);
        assert_eq!(PageTableLevel::L2.shift(), 21);
        assert_eq!(PageTableLevel::L3.shift(), 12);

        assert_eq!(PageTableLevel::L1.block_size(), 1024 * 1024 * 1024); // 1GB
        assert_eq!(PageTableLevel::L2.block_size(), 2 * 1024 * 1024);    // 2MB
        assert_eq!(PageTableLevel::L3.block_size(), 4096);                // 4KB
    }

    #[test]
    fn test_table_index() {
        let vaddr = VirtAddr::new(0xFFFF_0000_1234_5678);

        // L0 index: bits [47:39]
        let l0_idx = PageTableLevel::L0.index(vaddr);
        assert_eq!(l0_idx, 0x100); // 256

        // L3 index: bits [20:12]
        let l3_idx = PageTableLevel::L3.index(vaddr);
        assert_eq!(l3_idx, 0x345);
    }
}
