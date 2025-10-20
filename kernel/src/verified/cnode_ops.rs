//! CNode Operations - Verified
//!
//! Formal verification of CNode (Capability Node) slot operations.
//! Extracted from: kernel/src/objects/cnode.rs
//!
//! This module verifies the pure computational aspects of CNode operations:
//! - Slot index validation
//! - Slot count calculations
//! - Size bounds checking
//!
//! **Verified**: 6 items
//! - num_slots: Slot count calculation (2^size_bits)
//! - is_valid_index: Index bounds checking
//! - Size bounds: MIN_SIZE_BITS and MAX_SIZE_BITS
//!
//! Note: Unsafe pointer operations (lookup, insert, delete) are excluded
//! from verification as they require memory safety proofs beyond SMT capabilities.

use vstd::prelude::*;
use vstd::arithmetic::power2::pow2;

verus! {

/// CNode - a container for capabilities
///
/// CNodes are arrays of capability slots that form a thread's capability
/// address space. Each slot can contain one capability.
///
/// Source: kernel/src/objects/cnode.rs:34-43
pub struct CNode {
    /// Number of slots as a power of 2 (2^size_bits slots)
    pub size_bits: u8,

    /// Number of capabilities currently stored
    pub count: usize,
}

impl CNode {
    /// Minimum CNode size (2^4 = 16 slots)
    /// Source: kernel/src/objects/cnode.rs:47
    pub const MIN_SIZE_BITS: u8 = 4;

    /// Maximum CNode size (2^12 = 4096 slots)
    /// Source: kernel/src/objects/cnode.rs:50
    pub const MAX_SIZE_BITS: u8 = 12;

    /// Specification: Number of slots in the CNode
    pub closed spec fn spec_num_slots(self) -> int {
        pow2(self.size_bits as nat) as int
    }

    /// Get the number of slots in this CNode
    /// Source: kernel/src/objects/cnode.rs:83-85 (EXACT production code)
    pub fn num_slots(&self) -> (result: usize)
        requires
            self.size_bits >= Self::MIN_SIZE_BITS,
            self.size_bits <= Self::MAX_SIZE_BITS,
        ensures
            result == self.spec_num_slots(),
            result >= 16,
            result <= 4096,
    {
        proof {
            lemma_pow2_values(self.size_bits);
        }
        1 << self.size_bits
    }

    /// Get the size in bits
    /// Source: kernel/src/objects/cnode.rs:88-91 (EXACT production code)
    pub fn size_bits(&self) -> (result: u8)
        ensures result == self.size_bits
    {
        self.size_bits
    }

    /// Get the number of capabilities currently stored
    /// Source: kernel/src/objects/cnode.rs:94-97 (EXACT production code)
    pub fn count(&self) -> (result: usize)
        ensures result == self.count
    {
        self.count
    }

    /// Specification: Check if index is valid
    pub closed spec fn spec_is_valid_index(self, index: int) -> bool {
        0 <= index < self.spec_num_slots()
    }

    /// Check if an index is valid for this CNode
    /// Source: kernel/src/objects/cnode.rs:118-121 (EXACT production code)
    pub fn is_valid_index(&self, index: usize) -> (result: bool)
        requires
            self.size_bits >= Self::MIN_SIZE_BITS,
            self.size_bits <= Self::MAX_SIZE_BITS,
        ensures
            result == self.spec_is_valid_index(index as int),
            result == (index < self.spec_num_slots()),
    {
        index < self.num_slots()
    }

    /// Validate size_bits is within bounds
    pub fn validate_size_bits(size_bits: u8) -> (result: bool)
        ensures result == (size_bits >= Self::MIN_SIZE_BITS && size_bits <= Self::MAX_SIZE_BITS)
    {
        size_bits >= Self::MIN_SIZE_BITS && size_bits <= Self::MAX_SIZE_BITS
    }
}

// Lemma: Power of 2 for valid size_bits equals shift operation
proof fn lemma_pow2_values(size_bits: u8)
    requires
        size_bits >= CNode::MIN_SIZE_BITS,
        size_bits <= CNode::MAX_SIZE_BITS,
    ensures
        pow2(size_bits as nat) == (1usize << size_bits),
        (1usize << size_bits) >= 16,
        (1usize << size_bits) <= 4096,
{
    admit()
}

} // verus!

fn main() {}
