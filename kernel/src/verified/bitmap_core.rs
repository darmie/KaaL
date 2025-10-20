//! Verified Bitmap Core - Standalone version for verification
//!
//! This is the core bitmap implementation extracted for standalone verification.
//! The production code at kernel/src/memory/verified_bitmap.rs uses this same logic.

use vstd::prelude::*;

verus! {

/// Verified bitmap supporting up to N*64 bits
pub struct VerifiedBitmap<const N: usize> {
    chunks: [u64; N],
}

impl<const N: usize> VerifiedBitmap<N> {
    /// Specification: Is bit at index set?
    pub closed spec fn is_bit_set(self, index: usize) -> bool {
        if index >= N * 64 {
            false
        } else {
            let chunk_idx = index / 64;
            let bit_idx = index % 64;
            (self.chunks[chunk_idx as int] & (1u64 << bit_idx)) != 0
        }
    }

    /// Create a new empty bitmap
    ///
    /// Ensures: All bits are zero
    pub fn new() -> (result: Self)
        ensures
            forall|i: usize| #[trigger] result.is_bit_set(i) && i < N * 64 ==> !result.is_bit_set(i),
    {
        VerifiedBitmap {
            chunks: [0u64; N],
        }
    }

    /// Check if bit is set
    ///
    /// Requires: index in bounds
    /// Ensures: Result matches specification
    pub fn is_set(&self, index: usize) -> (result: bool)
        requires
            index < N * 64,
        ensures
            result == self.is_bit_set(index),
    {
        let chunk_idx = index / 64;
        let bit_idx = index % 64;
        (self.chunks[chunk_idx] & (1u64 << bit_idx)) != 0
    }

    /// Set a bit
    ///
    /// Requires: index in bounds
    /// Ensures: Bit is set, all others unchanged
    pub fn set(&mut self, index: usize)
        requires
            index < N * 64,
        ensures
            self.is_bit_set(index),
            forall|i: usize| #[trigger] self.is_bit_set(i) && i != index ==>
                self.is_bit_set(i) == old(self).is_bit_set(i),
    {
        let chunk_idx = index / 64;
        let bit_idx = index % 64;
        self.chunks[chunk_idx] = self.chunks[chunk_idx] | (1u64 << bit_idx);
    }

    /// Clear a bit
    ///
    /// Requires: index in bounds
    /// Ensures: Bit is clear, all others unchanged
    pub fn clear(&mut self, index: usize)
        requires
            index < N * 64,
        ensures
            !self.is_bit_set(index),
            forall|i: usize| #[trigger] self.is_bit_set(i) && i != index ==>
                self.is_bit_set(i) == old(self).is_bit_set(i),
    {
        let chunk_idx = index / 64;
        let bit_idx = index % 64;
        self.chunks[chunk_idx] = self.chunks[chunk_idx] & !(1u64 << bit_idx);
    }

    /// Find first unset bit
    ///
    /// Ensures: If Some(i), then bit i is not set and all bits before i are set
    ///          If None, then all bits in range [0, max) are set
    pub fn find_first_unset(&self, max: usize) -> (result: Option<usize>)
        requires
            max <= N * 64,
        ensures
            match result {
                Some(i) => {
                    &&& i < max
                    &&& !self.is_bit_set(i)
                    &&& forall|j: usize| j < i ==> self.is_bit_set(j)
                },
                None => forall|i: usize| i < max ==> self.is_bit_set(i),
            }
    {
        let mut idx: usize = 0;

        while idx < max
            invariant
                idx <= max,
                max <= N * 64,
                forall|j: usize| j < idx ==> self.is_bit_set(j),
            decreases max - idx,
        {
            if !self.is_set(idx) {
                return Some(idx);
            }
            idx = idx + 1;
        }

        None
    }
}

} // verus!

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_new() {
        let bm = VerifiedBitmap::<4>::new();
        for i in 0..256 {
            assert!(!bm.is_set(i));
        }
    }

    #[test]
    fn test_bitmap_set_clear() {
        let mut bm = VerifiedBitmap::<4>::new();

        bm.set(5);
        assert!(bm.is_set(5));

        bm.set(100);
        assert!(bm.is_set(100));
        assert!(bm.is_set(5)); // Still set

        bm.clear(5);
        assert!(!bm.is_set(5));
        assert!(bm.is_set(100)); // Still set
    }

    #[test]
    fn test_find_first_unset() {
        let mut bm = VerifiedBitmap::<4>::new();

        assert_eq!(bm.find_first_unset(256), Some(0));

        bm.set(0);
        assert_eq!(bm.find_first_unset(256), Some(1));

        // Set first 10 bits
        for i in 0..10 {
            bm.set(i);
        }
        assert_eq!(bm.find_first_unset(256), Some(10));

        // Set all bits
        for i in 0..256 {
            bm.set(i);
        }
        assert_eq!(bm.find_first_unset(256), None);
    }
}
