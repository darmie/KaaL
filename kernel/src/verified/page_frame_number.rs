//! Verified PageFrameNumber Operations
//!
//! This file contains the EXACT same implementation as kernel/src/memory/address.rs
//! PageFrameNumber methods, extracted for standalone verification.
//!
//! **CRITICAL**: This must stay in sync with production code!

use vstd::prelude::*;

verus! {

// Page size constant (4KB)
pub const PAGE_SIZE: usize = 4096;

// Page frame number - EXACT copy from production (converted to named field for Verus)
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct PageFrameNumber {
    pub pfn: usize
}

impl PageFrameNumber {
    // Create new PFN - EXACT production code
    // Production: kernel/src/memory/address.rs:293-295
    pub fn new(pfn: usize) -> Self {
        PageFrameNumber { pfn }
    }

    // Get as usize - EXACT production code
    // Production: kernel/src/memory/address.rs:308-310
    pub open spec fn as_usize(self) -> usize {
        self.pfn
    }

    // Get physical address - EXACT production code
    // Production: kernel/src/memory/address.rs:303-305
    pub fn phys_addr(self) -> (result: usize)
        requires self.pfn as int * PAGE_SIZE as int <= usize::MAX as int
        ensures result == self.pfn * PAGE_SIZE
    {
        self.pfn * PAGE_SIZE
    }

    // Create PFN from physical address - EXACT production code
    // Production: kernel/src/memory/address.rs:298-300
    pub fn from_phys_addr(addr: usize) -> (result: Self)
        ensures result.as_usize() == addr / PAGE_SIZE
    {
        PageFrameNumber { pfn: addr / PAGE_SIZE }
    }
}

} // verus!

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let pfn = PageFrameNumber::new(42);
        assert_eq!(pfn.as_usize(), 42);
    }

    #[test]
    fn test_phys_addr() {
        let pfn = PageFrameNumber::new(1);
        assert_eq!(pfn.phys_addr(), 4096);

        let pfn2 = PageFrameNumber::new(10);
        assert_eq!(pfn2.phys_addr(), 40960);
    }

    #[test]
    fn test_from_phys_addr() {
        let pfn = PageFrameNumber::from_phys_addr(0x1000);
        assert_eq!(pfn.as_usize(), 1);

        let pfn2 = PageFrameNumber::from_phys_addr(0x2500);
        assert_eq!(pfn2.as_usize(), 2); // Truncates down

        let pfn3 = PageFrameNumber::from_phys_addr(0);
        assert_eq!(pfn3.as_usize(), 0);
    }

    #[test]
    fn test_roundtrip() {
        // Test that converting address -> PFN -> address (page-aligned) works
        let aligned_addr = 0x5000; // 5 * 4096
        let pfn = PageFrameNumber::from_phys_addr(aligned_addr);
        assert_eq!(pfn.phys_addr(), aligned_addr);
    }
}
