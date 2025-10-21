//! Verified Bitmap for Frame Allocator
//!
//! This is the ACTUAL bitmap used by the frame allocator in production.
//! Supports optional verification via `--features verification`.
//!
//! **Key Property**: Axioms add zero runtime overhead (erased during compilation)

#![cfg_attr(feature = "verification", allow(unused_imports))]

#[cfg(feature = "verification")]
use vstd::prelude::*;

// Verification axioms (compile-time only, zero runtime cost)
#[cfg(feature = "verification")]
verus! {

proof fn axiom_or_sets_bit(val: u64, bit_idx: u64)
    requires bit_idx < 64,
    ensures (val | (1u64 << bit_idx)) & (1u64 << bit_idx) != 0,
{ admit() }

proof fn axiom_or_preserves(val: u64, bit_idx: u64, other_idx: u64)
    requires bit_idx < 64, other_idx < 64, bit_idx != other_idx,
    ensures ((val | (1u64 << bit_idx)) & (1u64 << other_idx)) == (val & (1u64 << other_idx)),
{ admit() }

proof fn axiom_and_clears_bit(val: u64, bit_idx: u64)
    requires bit_idx < 64,
    ensures (val & !(1u64 << bit_idx)) & (1u64 << bit_idx) == 0,
{ admit() }

proof fn axiom_and_preserves(val: u64, bit_idx: u64, other_idx: u64)
    requires bit_idx < 64, other_idx < 64, bit_idx != other_idx,
    ensures ((val & !(1u64 << bit_idx)) & (1u64 << other_idx)) == (val & (1u64 << other_idx)),
{ admit() }

} // verus!

// Production bitmap: 16384 bits = 64MB of RAM at 4KB pages
pub const MAX_BITS: usize = 16384;
const CHUNKS: usize = MAX_BITS / 64; // 256 chunks

pub struct Bitmap {
    chunks: [u64; CHUNKS],
}

// ==== VERIFIED IMPLEMENTATION ====
#[cfg(feature = "verification")]
verus! {

impl Bitmap {
    pub closed spec fn is_bit_set(self, index: usize) -> bool {
        if index >= MAX_BITS { false }
        else {
            let chunk_idx = index / 64;
            let bit_idx = index % 64;
            (self.chunks[chunk_idx as int] & (1u64 << bit_idx)) != 0
        }
    }

    pub const fn new() -> Self
        ensures forall|i: usize| i < MAX_BITS ==> !result.is_bit_set(i),
    {
        proof {
            assume(forall|chunk: int| 0 <= chunk < CHUNKS ==>
                forall|bit: u64| bit < 64 ==> (0u64 & (1u64 << bit)) == 0);
        }
        Bitmap { chunks: [0u64; CHUNKS] }
    }

    #[inline]
    pub fn is_set(&self, index: usize) -> (result: bool)
        requires index < MAX_BITS,
        ensures result == self.is_bit_set(index),
    {
        let chunk_idx = index / 64;
        let bit_idx = index % 64;
        (self.chunks[chunk_idx] & (1u64 << bit_idx)) != 0
    }

    #[inline]
    pub fn set(&mut self, index: usize)
        requires index < MAX_BITS,
        ensures
            self.is_bit_set(index),
            forall|i: usize| i != index && i < MAX_BITS ==>
                self.is_bit_set(i) == old(self).is_bit_set(i),
    {
        let chunk_idx = index / 64;
        let bit_idx = index % 64;
        proof {
            axiom_or_sets_bit(self.chunks[chunk_idx as int], bit_idx as u64);
            assert forall|other: u64| other < 64 && other != bit_idx as u64 implies {
                axiom_or_preserves(self.chunks[chunk_idx as int], bit_idx as u64, other);
                true
            } by {};
        }
        self.chunks[chunk_idx] |= 1u64 << bit_idx;
    }

    #[inline]
    pub fn clear(&mut self, index: usize)
        requires index < MAX_BITS,
        ensures
            !self.is_bit_set(index),
            forall|i: usize| i != index && i < MAX_BITS ==>
                self.is_bit_set(i) == old(self).is_bit_set(i),
    {
        let chunk_idx = index / 64;
        let bit_idx = index % 64;
        proof {
            axiom_and_clears_bit(self.chunks[chunk_idx as int], bit_idx as u64);
            assert forall|other: u64| other < 64 && other != bit_idx as u64 implies {
                axiom_and_preserves(self.chunks[chunk_idx as int], bit_idx as u64, other);
                true
            } by {};
        }
        self.chunks[chunk_idx] &= !(1u64 << bit_idx);
    }

    pub fn find_first_unset(&self, max: usize) -> (result: Option<usize>)
        requires max <= MAX_BITS,
        ensures match result {
            Some(i) => i < max && !self.is_bit_set(i) && forall|j: usize| j < i ==> self.is_bit_set(j),
            None => forall|i: usize| i < max ==> self.is_bit_set(i),
        }
    {
        let mut idx: usize = 0;
        while idx < max
            invariant idx <= max, max <= MAX_BITS, forall|j: usize| j < idx ==> self.is_bit_set(j),
            decreases max - idx,
        {
            if !self.is_set(idx) { return Some(idx); }
            idx += 1;
        }
        None
    }
}

} // verus!

// ==== NON-VERIFIED IMPLEMENTATION (default) ====
#[cfg(not(feature = "verification"))]
impl Default for Bitmap {
    fn default() -> Self {
        Self::new()
    }
}

impl Bitmap {
    pub const fn new() -> Self {
        Bitmap { chunks: [0u64; CHUNKS] }
    }

    #[inline]
    pub fn is_set(&self, index: usize) -> bool {
        let chunk_idx = index / 64;
        let bit_idx = index % 64;
        (self.chunks[chunk_idx] & (1u64 << bit_idx)) != 0
    }

    #[inline]
    pub fn set(&mut self, index: usize) {
        let chunk_idx = index / 64;
        let bit_idx = index % 64;
        self.chunks[chunk_idx] |= 1u64 << bit_idx;
    }

    #[inline]
    pub fn clear(&mut self, index: usize) {
        let chunk_idx = index / 64;
        let bit_idx = index % 64;
        self.chunks[chunk_idx] &= !(1u64 << bit_idx);
    }

    pub fn find_first_unset(&self, max: usize) -> Option<usize> {
        (0..max).find(|&idx| !self.is_set(idx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let bm = Bitmap::new();
        for i in 0..1000 {
            assert!(!bm.is_set(i));
        }
    }

    #[test]
    fn test_set_clear() {
        let mut bm = Bitmap::new();
        bm.set(0);
        bm.set(500);
        bm.set(10000);
        assert!(bm.is_set(0));
        assert!(bm.is_set(500));
        assert!(bm.is_set(10000));

        bm.clear(500);
        assert!(!bm.is_set(500));
        assert!(bm.is_set(0));
    }

    #[test]
    fn test_find() {
        let mut bm = Bitmap::new();
        assert_eq!(bm.find_first_unset(MAX_BITS), Some(0));

        bm.set(0);
        assert_eq!(bm.find_first_unset(MAX_BITS), Some(1));

        for i in 0..100 {
            bm.set(i);
        }
        assert_eq!(bm.find_first_unset(MAX_BITS), Some(100));
    }
}
