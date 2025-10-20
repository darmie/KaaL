//! Simple Verified Bitmap - Standalone test file

use vstd::prelude::*;

verus! {

pub struct Bitmap {
    bits: [bool; 64],
}

impl Bitmap {
    pub closed spec fn bit_is_set(self, index: usize) -> bool {
        index < 64 && self.bits[index as int]
    }

    pub fn new() -> (result: Self)
        ensures
            forall|i: int| #[trigger] result.bit_is_set(i as usize) && 0 <= i < 64 ==> !result.bit_is_set(i as usize),
    {
        Bitmap {
            bits: [false; 64],
        }
    }

    pub fn set(&mut self, index: usize)
        requires
            index < 64,
        ensures
            self.bit_is_set(index),
            forall|i: usize| #[trigger] self.bit_is_set(i) && i != index ==> self.bit_is_set(i) == old(self).bit_is_set(i),
    {
        self.bits[index] = true;
    }

    pub fn is_set(&self, index: usize) -> (result: bool)
        requires
            index < 64,
        ensures
            result == self.bit_is_set(index),
    {
        self.bits[index]
    }
}

} // verus!

fn main() {}
