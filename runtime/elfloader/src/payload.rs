//! Payload structures (shared with elfloader-builder)

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Memory region to be loaded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    /// Physical address to load at
    pub paddr: usize,
    /// Virtual address (for segments)
    pub vaddr: usize,
    /// Size in bytes
    pub size: usize,
    /// Offset in the serialized data (after payload metadata)
    pub data_offset: usize,
    /// Size of data to copy (may be less than size for BSS)
    pub data_size: usize,
}

/// Complete payload metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct Payload {
    /// Kernel regions
    pub kernel_regions: Vec<Region>,
    /// Kernel entry point (virtual address)
    pub kernel_entry: usize,
    /// User (root task) regions
    pub user_regions: Vec<Region>,
    /// User entry point (virtual address)
    pub user_entry: usize,
    /// Total size of all region data
    pub total_data_size: usize,
}

impl Payload {
    /// Calculate the kernel's physical address range
    pub fn kernel_paddr_range(&self) -> (usize, usize) {
        let mut min = usize::MAX;
        let mut max = 0;
        for region in &self.kernel_regions {
            if region.paddr < min {
                min = region.paddr;
            }
            let end = region.paddr + region.size;
            if end > max {
                max = end;
            }
        }
        (min, max)
    }

    /// Calculate the user's physical address range
    pub fn user_paddr_range(&self) -> (usize, usize) {
        let mut min = usize::MAX;
        let mut max = 0;
        for region in &self.user_regions {
            if region.paddr < min {
                min = region.paddr;
            }
            let end = region.paddr + region.size;
            if end > max {
                max = end;
            }
        }
        (min, max)
    }

    /// Load all regions from serialized data into physical memory
    pub unsafe fn load_to_memory(&self, data: &[u8]) {
        crate::uart_println!("Loading kernel regions...");
        for region in &self.kernel_regions {
            self.load_region(region, data);
        }

        crate::uart_println!("Loading user regions...");
        for region in &self.user_regions {
            self.load_region(region, data);
        }
    }

    unsafe fn load_region(&self, region: &Region, data: &[u8]) {
        let dest = region.paddr as *mut u8;
        let src = &data[region.data_offset..region.data_offset + region.data_size];

        crate::uart_println!("  {:#x} <- {} bytes", region.paddr, region.data_size);

        // Copy data
        core::ptr::copy_nonoverlapping(src.as_ptr(), dest, src.len());

        // Zero BSS if needed
        if region.size > region.data_size {
            let bss_start = dest.add(region.data_size);
            let bss_size = region.size - region.data_size;
            core::ptr::write_bytes(bss_start, 0, bss_size);
            crate::uart_println!("  {:#x} <- {} bytes zeroed (BSS)",
                region.paddr + region.data_size, bss_size);
        }
    }
}
