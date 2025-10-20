//! Verified Bitmap Operations
//!
//! This module provides a simple bitmap with verified operations.
//! Used as an introduction to Verus verification for the KaaL microkernel.
//!
//! ## Verified Properties
//!
//! - `new()`: Creates bitmap with all bits false
//! - `set(i)`: Sets bit i to true, preserves all other bits
//! - `clear(i)`: Sets bit i to false, preserves all other bits
//! - `is_set(i)`: Returns true iff bit i is set
//! - `find_first_unset()`: Returns smallest unset index, or None if all set

#![allow(unused_imports)]
use builtin::*;
use builtin_macros::*;
use vstd::prelude::*;

verus! {

/// A bitmap represented as a fixed-size array
pub struct Bitmap {
    bits: [bool; 64],
}

impl Bitmap {
    /// Specification: What does it mean for a bit to be set?
    pub closed spec fn bit_is_set(self, index: usize) -> bool {
        index < 64 && self.bits[index as int]
    }

    /// Specification: How many bits are set?
    pub closed spec fn count_set(self) -> int
        decreases 64
    {
        self.count_set_up_to(64)
    }

    /// Helper spec: Count set bits up to index
    spec fn count_set_up_to(self, index: int) -> int
        decreases index
    {
        if index <= 0 {
            0
        } else {
            let prev = self.count_set_up_to(index - 1);
            if self.bits[(index - 1) as int] {
                prev + 1
            } else {
                prev
            }
        }
    }

    /// Create a new empty bitmap
    ///
    /// Ensures: All bits are false
    pub fn new() -> (result: Self)
        ensures
            forall|i: int| 0 <= i < 64 ==> !result.bit_is_set(i as usize),
            result.count_set() == 0,
    {
        Bitmap {
            bits: [false; 64],
        }
    }

    /// Set a bit to true
    ///
    /// Requires: index < 64
    /// Ensures: The bit at index is now true
    pub fn set(&mut self, index: usize)
        requires
            index < 64,
        ensures
            self.bit_is_set(index),
            forall|i: usize| i != index ==> self.bit_is_set(i) == old(self).bit_is_set(i),
    {
        self.bits[index] = true;
    }

    /// Clear a bit to false
    ///
    /// Requires: index < 64
    /// Ensures: The bit at index is now false
    pub fn clear(&mut self, index: usize)
        requires
            index < 64,
        ensures
            !self.bit_is_set(index),
            forall|i: usize| i != index ==> self.bit_is_set(i) == old(self).bit_is_set(i),
    {
        self.bits[index] = false;
    }

    /// Check if a bit is set
    ///
    /// Requires: index < 64
    /// Ensures: Result matches specification
    pub fn is_set(&self, index: usize) -> (result: bool)
        requires
            index < 64,
        ensures
            result == self.bit_is_set(index),
    {
        self.bits[index]
    }

    /// Find first unset bit
    ///
    /// Ensures: If Some(i), then bit i is not set
    ///          If None, then all bits are set
    pub fn find_first_unset(&self) -> (result: Option<usize>)
        ensures
            match result {
                Some(i) => i < 64 && !self.bit_is_set(i),
                None => forall|i: usize| i < 64 ==> self.bit_is_set(i),
            }
    {
        let mut i: usize = 0;

        while i < 64
            invariant
                i <= 64,
                forall|j: usize| j < i ==> self.bit_is_set(j),
        {
            if !self.bits[i] {
                return Some(i);
            }
            i = i + 1;
        }

        None
    }
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_new() {
        let bm = Bitmap::new();
        for i in 0..64 {
            assert!(!bm.is_set(i));
        }
    }

    #[test]
    fn test_bitmap_set_clear() {
        let mut bm = Bitmap::new();
        bm.set(5);
        assert!(bm.is_set(5));
        bm.clear(5);
        assert!(!bm.is_set(5));
    }

    #[test]
    fn test_find_first_unset() {
        let mut bm = Bitmap::new();
        assert_eq!(bm.find_first_unset(), Some(0));

        bm.set(0);
        assert_eq!(bm.find_first_unset(), Some(1));

        // Set all bits
        for i in 0..64 {
            bm.set(i);
        }
        assert_eq!(bm.find_first_unset(), None);
    }
}
