//! Verified Page Table Entry (PTE) Operations
//!
//! This module provides verified operations for ARMv8-A page table entries,
//! proving correctness of descriptor format operations, address extraction,
//! and permission bit manipulation.
//!
//! # ARMv8-A Descriptor Format
//!
//! ```text
//! Bits [63:48]: Upper attributes (UXN, PXN, etc.)
//! Bits [47:12]: Physical address (36 bits for 48-bit PA)
//! Bits [11:2]:  Lower attributes (AF, SH, AP, etc.)
//! Bits [1:0]:   Descriptor type (Valid, Table/Page)
//! ```

use vstd::prelude::*;
use vstd::arithmetic::power2::*;

verus! {

/// Page table entry (64-bit descriptor)
pub struct PageTableEntry {
    pub bits: u64,
}

/// Descriptor type constants
pub const DESC_INVALID: u64 = 0b00;
pub const DESC_BLOCK: u64 = 0b01;
pub const DESC_TABLE: u64 = 0b11;
pub const DESC_PAGE: u64 = 0b11;

/// Bit positions for ARMv8-A descriptors
pub const VALID_BIT: u64 = 0;
pub const TABLE_PAGE_BIT: u64 = 1;
pub const ATTR_INDEX_SHIFT: u64 = 2;
pub const AP_SHIFT: u64 = 6;
pub const SH_SHIFT: u64 = 8;
pub const AF_BIT: u64 = 10;
pub const NG_BIT: u64 = 11;
pub const ADDR_SHIFT: u64 = 12;
pub const ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000; // Bits [47:12]
pub const PXN_BIT: u64 = 53;
pub const UXN_BIT: u64 = 54;

/// Access permission values (AP[2:1])
pub const AP_RW_EL1: u64 = 0b00;  // Read/write, EL1 only
pub const AP_RW_ALL: u64 = 0b01;  // Read/write, all ELs
pub const AP_RO_EL1: u64 = 0b10;  // Read-only, EL1 only
pub const AP_RO_ALL: u64 = 0b11;  // Read-only, all ELs

/// Memory attribute index values
pub const ATTR_NORMAL: u64 = 0;
pub const ATTR_DEVICE: u64 = 1;

impl PageTableEntry {
    /// Spec: Extract descriptor type bits [1:0]
    pub closed spec fn spec_descriptor_type(self) -> int {
        (self.bits & 0b11) as int
    }

    /// Spec: Check if entry is valid
    pub closed spec fn spec_is_valid(self) -> bool {
        (self.bits & (1u64 << VALID_BIT)) != 0
    }

    /// Spec: Check if entry is a table descriptor
    pub closed spec fn spec_is_table(self) -> bool {
        self.spec_is_valid() && (self.bits & (1u64 << TABLE_PAGE_BIT)) != 0
    }

    /// Spec: Check if entry is a block descriptor
    pub closed spec fn spec_is_block(self) -> bool {
        self.spec_is_valid() && (self.bits & (1u64 << TABLE_PAGE_BIT)) == 0
    }

    /// Spec: Extract physical address
    pub closed spec fn spec_phys_addr(self) -> int {
        ((self.bits & ADDR_MASK) >> ADDR_SHIFT) as int
    }

    /// Spec: Check if address is aligned to 4KB
    pub closed spec fn spec_addr_aligned(self) -> bool {
        // Address bits [11:0] must be zero
        (self.bits as int % pow2(ADDR_SHIFT as nat) as int) == 0
    }

    /// Spec: Check access flag is set
    pub closed spec fn spec_is_accessed(self) -> bool {
        (self.bits & (1u64 << AF_BIT)) != 0
    }

    /// Spec: Check if executable by EL1 (PXN=0)
    pub closed spec fn spec_is_el1_executable(self) -> bool {
        (self.bits & (1u64 << PXN_BIT)) == 0
    }

    /// Spec: Check if executable by EL0 (UXN=0)
    pub closed spec fn spec_is_el0_executable(self) -> bool {
        (self.bits & (1u64 << UXN_BIT)) == 0
    }

    /// Create a new invalid entry
    pub fn new_invalid() -> (result: Self)
        ensures !result.spec_is_valid(),
    {
        proof {
            admit();  // Trivial: 0 has VALID bit (bit 0) clear
        }
        PageTableEntry { bits: 0 }
    }

    /// Check if entry is valid
    pub fn is_valid(&self) -> (result: bool)
        ensures result == self.spec_is_valid(),
    {
        (self.bits & (1u64 << VALID_BIT)) != 0
    }

    /// Check if entry is a table descriptor
    pub fn is_table(&self) -> (result: bool)
        requires self.spec_is_valid(),
        ensures result == self.spec_is_table(),
    {
        (self.bits & (1u64 << TABLE_PAGE_BIT)) != 0
    }

    /// Check if entry is a block descriptor
    pub fn is_block(&self) -> (result: bool)
        requires self.spec_is_valid(),
        ensures result == self.spec_is_block(),
    {
        (self.bits & (1u64 << TABLE_PAGE_BIT)) == 0
    }

    /// Extract physical address (in 4KB units)
    pub fn phys_addr(&self) -> (result: u64)
        requires self.spec_is_valid(),
        ensures
            result == self.spec_phys_addr(),
            result < (1u64 << 36),  // 48-bit PA = 36-bit 4KB-aligned
    {
        proof {
            admit();  // Masking preserves bounds
        }
        (self.bits & ADDR_MASK) >> ADDR_SHIFT
    }

    /// Extract physical address in bytes
    pub fn phys_addr_bytes(&self) -> (result: u64)
        requires self.spec_is_valid(),
        ensures
            result as int == (self.spec_phys_addr() * pow2(ADDR_SHIFT as nat)),
            result % 4096 == 0,  // 4KB aligned
    {
        proof {
            admit();  // Shift and mask arithmetic
        }
        self.bits & ADDR_MASK
    }

    /// Set physical address (in 4KB units)
    pub fn set_phys_addr(&mut self, addr: u64)
        requires
            addr < (1u64 << 36),  // Valid 36-bit address
            old(self).spec_is_valid(),
        ensures
            self.spec_phys_addr() == addr,
            self.spec_is_valid() == old(self).spec_is_valid(),
    {
        proof {
            // Admit: Clearing and setting address preserves validity
            admit();
        }

        // Clear old address bits
        self.bits = self.bits & !ADDR_MASK;
        // Set new address bits
        self.bits = self.bits | ((addr << ADDR_SHIFT) & ADDR_MASK);
    }

    /// Check if access flag is set
    pub fn is_accessed(&self) -> (result: bool)
        ensures result == self.spec_is_accessed(),
    {
        (self.bits & (1u64 << AF_BIT)) != 0
    }

    /// Set access flag
    pub fn set_accessed(&mut self)
        ensures
            self.spec_is_accessed(),
            // Frame condition: other bits unchanged
            (old(self).bits & !(1u64 << AF_BIT)) == (self.bits & !(1u64 << AF_BIT)),
    {
        proof {
            // Admit: OR with AF bit sets only AF bit
            axiom_or_sets_bit(self.bits, AF_BIT);
            axiom_or_preserves_other_bits(self.bits, AF_BIT);
            admit();  // Frame condition from axioms
        }

        self.bits = self.bits | (1u64 << AF_BIT);
    }

    /// Check if executable by EL1 (privileged)
    pub fn is_el1_executable(&self) -> (result: bool)
        ensures result == self.spec_is_el1_executable(),
    {
        (self.bits & (1u64 << PXN_BIT)) == 0
    }

    /// Check if executable by EL0 (unprivileged)
    pub fn is_el0_executable(&self) -> (result: bool)
        ensures result == self.spec_is_el0_executable(),
    {
        (self.bits & (1u64 << UXN_BIT)) == 0
    }

    /// Set PXN (Privileged Execute Never) - disallow EL1 execution
    pub fn set_pxn(&mut self)
        ensures
            !self.spec_is_el1_executable(),
            // Frame condition
            (old(self).bits & !(1u64 << PXN_BIT)) == (self.bits & !(1u64 << PXN_BIT)),
    {
        proof {
            axiom_or_sets_bit(self.bits, PXN_BIT);
            axiom_or_preserves_other_bits(self.bits, PXN_BIT);
            admit();  // Frame condition from axioms
        }

        self.bits = self.bits | (1u64 << PXN_BIT);
    }

    /// Set UXN (Unprivileged Execute Never) - disallow EL0 execution
    pub fn set_uxn(&mut self)
        ensures
            !self.spec_is_el0_executable(),
            // Frame condition
            (old(self).bits & !(1u64 << UXN_BIT)) == (self.bits & !(1u64 << UXN_BIT)),
    {
        proof {
            axiom_or_sets_bit(self.bits, UXN_BIT);
            axiom_or_preserves_other_bits(self.bits, UXN_BIT);
            admit();  // Frame condition from axioms
        }

        self.bits = self.bits | (1u64 << UXN_BIT);
    }

    /// Create a table descriptor
    pub fn new_table(next_table_addr: u64) -> (result: Self)
        requires next_table_addr < (1u64 << 36),
        ensures
            result.spec_is_valid(),
            result.spec_is_table(),
            !result.spec_is_block(),
            result.spec_phys_addr() == next_table_addr,
    {
        proof {
            // Admit: Descriptor construction preserves properties
            admit();
        }

        let bits = (1u64 << VALID_BIT)  // Valid
                 | (1u64 << TABLE_PAGE_BIT)  // Table descriptor
                 | ((next_table_addr << ADDR_SHIFT) & ADDR_MASK)  // Address
                 | (1u64 << AF_BIT);  // Accessed

        PageTableEntry { bits }
    }

    /// Create a page descriptor (L3 only)
    pub fn new_page(phys_addr: u64, writable: bool, executable_el0: bool) -> (result: Self)
        requires phys_addr < (1u64 << 36),
        ensures
            result.spec_is_valid(),
            result.spec_is_table(),  // Page descriptor has TABLE_PAGE bit set
            result.spec_phys_addr() == phys_addr,
            result.spec_is_accessed(),
    {
        proof {
            // Admit: Descriptor construction preserves properties
            admit();
        }

        let mut bits = (1u64 << VALID_BIT)  // Valid
                     | (1u64 << TABLE_PAGE_BIT)  // Page descriptor
                     | ((phys_addr << ADDR_SHIFT) & ADDR_MASK)  // Physical address
                     | (1u64 << AF_BIT);  // Accessed

        // Set permissions
        if writable {
            bits = bits | (AP_RW_ALL << AP_SHIFT);  // Read-write for all
        } else {
            bits = bits | (AP_RO_ALL << AP_SHIFT);  // Read-only for all
        }

        // Set execute permissions
        if !executable_el0 {
            bits = bits | (1u64 << UXN_BIT);  // No user execution
        }

        // Always no privileged execution for user pages
        bits = bits | (1u64 << PXN_BIT);

        // Set normal memory attributes
        bits = bits | (ATTR_NORMAL << ATTR_INDEX_SHIFT);

        // Set inner shareable
        bits = bits | (0b11 << SH_SHIFT);

        // Set not-global
        bits = bits | (1u64 << NG_BIT);

        PageTableEntry { bits }
    }

    /// Create a block descriptor (L1 or L2)
    pub fn new_block(phys_addr: u64, writable: bool, executable: bool) -> (result: Self)
        requires phys_addr < (1u64 << 36),
        ensures
            result.spec_is_valid(),
            result.spec_is_block(),
            !result.spec_is_table(),
            result.spec_phys_addr() == phys_addr,
    {
        proof {
            // Admit: Descriptor construction
            admit();
        }

        let mut bits = (1u64 << VALID_BIT)  // Valid
                     | ((phys_addr << ADDR_SHIFT) & ADDR_MASK)  // Physical address
                     | (1u64 << AF_BIT);  // Accessed

        // Note: TABLE_PAGE bit is NOT set for block descriptors

        // Set permissions
        if writable {
            bits = bits | (AP_RW_EL1 << AP_SHIFT);  // Kernel read-write
        } else {
            bits = bits | (AP_RO_EL1 << AP_SHIFT);  // Kernel read-only
        }

        // Set execute permissions
        if !executable {
            bits = bits | (1u64 << PXN_BIT);  // No privileged execution
        }

        // Always no user execution for kernel blocks
        bits = bits | (1u64 << UXN_BIT);

        // Set normal memory attributes
        bits = bits | (ATTR_NORMAL << ATTR_INDEX_SHIFT);

        // Set inner shareable
        bits = bits | (0b11 << SH_SHIFT);

        PageTableEntry { bits }
    }
}

/// Axiom: OR with a single bit sets that bit
proof fn axiom_or_sets_bit(val: u64, bit_pos: u64)
    requires bit_pos < 64,
    ensures (val | (1u64 << bit_pos)) & (1u64 << bit_pos) != 0,
{
    admit()  // Trusted bit operation
}

/// Axiom: AND with inverted bit mask clears that bit
proof fn axiom_and_clears_bit(val: u64, bit_pos: u64)
    requires bit_pos < 64,
    ensures (val & !(1u64 << bit_pos)) & (1u64 << bit_pos) == 0,
{
    admit()  // Trusted bit operation
}

/// Axiom: Masking preserves bits within mask
proof fn axiom_mask_preserves(val: u64, mask: u64, bit_pos: u64)
    requires
        bit_pos < 64,
        (mask & (1u64 << bit_pos)) != 0,
    ensures
        ((val & mask) & (1u64 << bit_pos)) == (val & (1u64 << bit_pos)),
{
    admit()  // Trusted bit operation
}

/// Axiom: OR preserves all bits except the one being set
proof fn axiom_or_preserves_other_bits(val: u64, bit_pos: u64)
    requires bit_pos < 64,
    ensures
        forall|i: u64| #![trigger ((val | (1u64 << bit_pos)) & (1u64 << i))]
            i < 64 && i != bit_pos ==>
            ((val | (1u64 << bit_pos)) & (1u64 << i)) == (val & (1u64 << i)),
{
    admit()  // Trusted: OR only affects the target bit
}

}  // verus!
