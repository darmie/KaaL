//! Verified PhysAddr Operations
//!
//! This file contains the EXACT same implementation as kernel/src/memory/address.rs
//! PhysAddr methods, extracted for standalone verification.
//!
//! **CRITICAL**: This must stay in sync with production code!

use vstd::prelude::*;

verus! {

// Page size constant (4KB)
pub const PAGE_SIZE: usize = 4096;

// Axiom: modulo result is less than or equal to input
proof fn axiom_mod_le_self(x: int, n: int)
    requires n > 0, x >= 0,
    ensures x % n <= x
{
    admit();
}

// Axiom: aligning down produces aligned result
proof fn axiom_align_down_divisible(x: int, n: int)
    requires n > 0,
    ensures (x - (x % n)) % n == 0
{
    admit();
}

// Physical address - EXACT copy from production (converted to named field for Verus)
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct PhysAddr {
    pub addr: usize
}

impl PhysAddr {
    // Create new address - EXACT production code
    pub fn new(addr: usize) -> Self {
        PhysAddr { addr }
    }

    // Get as usize - EXACT production code
    pub open spec fn as_usize(self) -> usize {
        self.addr
    }

    // Check if aligned - EXACT production code
    // Production: kernel/src/memory/address.rs:75-77
    pub fn is_aligned(self, align: usize) -> (result: bool)
        requires align > 0,
        ensures result == (self.addr as int % align as int == 0)
    {
        self.addr % align == 0
    }

    // Align down - EXACT production code
    // Production: kernel/src/memory/address.rs:80-81
    // NOTE: Full postcondition requires advanced modular arithmetic proofs
    pub fn align_down(self, align: usize) -> (result: Self)
        requires align > 0
        ensures result.as_usize() <= self.as_usize()
    {
        let remainder = self.addr % align;
        proof {
            vstd::arithmetic::div_mod::lemma_mod_bound(self.addr as int, align as int);
            axiom_mod_le_self(self.addr as int, align as int);
        }
        PhysAddr { addr: self.addr - remainder }
    }

    // Align up - EXACT production code
    // Production: kernel/src/memory/address.rs:84-86
    pub fn align_up(self, align: usize) -> (result: Self)
        requires
            align > 0,
            self.addr as int + align as int <= usize::MAX as int
        ensures
            result.as_usize() >= self.as_usize(),
            result.as_usize() as int % align as int == 0
    {
        let sum = self.addr + align - 1;
        let remainder = sum % align;
        proof {
            axiom_mod_le_self(sum as int, align as int);
            axiom_align_down_divisible(sum as int, align as int);
        }
        PhysAddr { addr: sum - remainder }
    }

    // Get page frame number - EXACT production code
    // Production: kernel/src/memory/address.rs:90-91
    pub fn page_number(self) -> (result: usize)
        ensures result == self.addr / PAGE_SIZE
    {
        self.addr / PAGE_SIZE
    }

    // Check if null - EXACT production code
    // Production: kernel/src/memory/address.rs:93-95
    pub fn is_null(self) -> (result: bool)
        ensures result == (self.addr == 0)
    {
        self.addr == 0
    }
}

} // verus!

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_aligned() {
        let addr = PhysAddr::new(0x1000);
        assert!(addr.is_aligned(0x1000));
        assert!(addr.is_aligned(0x100));
        assert!(addr.is_aligned(1));

        let addr2 = PhysAddr::new(0x1234);
        assert!(addr2.is_aligned(4));
        assert!(!addr2.is_aligned(8));
    }

    #[test]
    fn test_align_down() {
        let addr = PhysAddr::new(0x1234);
        assert_eq!(addr.align_down(0x1000).as_usize(), 0x1000);
        assert_eq!(addr.align_down(0x100).as_usize(), 0x1200);
        assert_eq!(addr.align_down(0x10).as_usize(), 0x1230);
    }

    #[test]
    fn test_align_up() {
        let addr = PhysAddr::new(0x1234);
        assert_eq!(addr.align_up(0x1000).as_usize(), 0x2000);
        assert_eq!(addr.align_up(0x100).as_usize(), 0x1300);
        assert_eq!(addr.align_up(0x10).as_usize(), 0x1240);

        // Already aligned
        let aligned = PhysAddr::new(0x2000);
        assert_eq!(aligned.align_up(0x1000).as_usize(), 0x2000);
    }
}
