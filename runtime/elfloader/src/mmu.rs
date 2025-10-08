// MMU and page table management for ARM64

use crate::utils::{align_down, align_up, PAGE_SIZE};
use crate::uart_println;

/// Page table entry flags
const PTE_VALID: u64 = 1 << 0;
const PTE_TABLE: u64 = 1 << 1;
const PTE_PAGE: u64 = 1 << 1;
const PTE_AF: u64 = 1 << 10; // Access flag
const PTE_SH: u64 = 3 << 8;  // Inner shareable
const PTE_ATTR_NORMAL: u64 = 0 << 2; // Normal memory
const PTE_ATTR_DEVICE: u64 = 1 << 2; // Device memory

/// Page table levels
const PT_LEVELS: usize = 3;

/// Page table
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [u64; 512],
}

impl PageTable {
    pub const fn new() -> Self {
        Self {
            entries: [0; 512],
        }
    }

    pub fn as_ptr(&self) -> usize {
        self as *const _ as usize
    }

    /// Map a physical address to a virtual address
    pub fn map_page(&mut self, vaddr: usize, paddr: usize, flags: u64) {
        let idx = (vaddr >> 12) & 0x1ff;
        self.entries[idx] = (paddr as u64 & !0xfff) | flags | PTE_VALID | PTE_AF;
    }

    /// Map a 2MB block
    pub fn map_block(&mut self, vaddr: usize, paddr: usize, flags: u64) {
        let idx = (vaddr >> 21) & 0x1ff;
        self.entries[idx] = (paddr as u64 & !(0x1fffff)) | flags | PTE_VALID | PTE_AF;
    }
}

/// Simple page table manager
pub struct PageTableManager {
    l1_table: PageTable,
    l2_table: PageTable,
    l3_table: PageTable,
}

impl PageTableManager {
    pub fn new() -> Self {
        Self {
            l1_table: PageTable::new(),
            l2_table: PageTable::new(),
            l3_table: PageTable::new(),
        }
    }

    /// Set up identity mapping for elfloader
    pub fn setup_identity_map(&mut self, start: usize, end: usize) {
        uart_println!("Setting up identity map: {:#x} - {:#x}", start, end);

        let start_page = align_down(start, PAGE_SIZE);
        let end_page = align_up(end, PAGE_SIZE);

        // Map using 2MB blocks for simplicity
        let start_block = align_down(start_page, 0x200000);
        let end_block = align_up(end_page, 0x200000);

        for addr in (start_block..end_block).step_by(0x200000) {
            self.l2_table.map_block(addr, addr, PTE_ATTR_NORMAL | PTE_SH);
        }

        // Link L1 to L2
        let l2_addr = self.l2_table.as_ptr();
        self.l1_table.entries[0] = (l2_addr as u64) | PTE_TABLE | PTE_VALID;
    }

    /// Get L1 table address for TTBR0
    pub fn get_ttbr0(&self) -> usize {
        self.l1_table.as_ptr()
    }

    /// Get default TCR value for 39-bit VA
    pub fn get_tcr() -> u64 {
        // T0SZ = 25 (39-bit VA), T1SZ = 25
        // TG0 = 0 (4KB granule), TG1 = 2 (4KB granule)
        // IPS = 0 (32-bit PA)
        (25 << 0) | (25 << 16) | (0 << 14) | (2 << 30) | (0 << 32)
    }

    /// Get default MAIR value
    pub fn get_mair() -> u64 {
        // Attr0 = Normal memory, write-back
        // Attr1 = Device-nGnRnE
        (0xff << 0) | (0x00 << 8)
    }
}
