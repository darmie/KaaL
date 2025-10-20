//! Verified Capability Rights Operations
//!
//! This file contains the EXACT same implementation as kernel/src/objects/capability.rs
//! CapRights methods, extracted for standalone verification.
//!
//! **CRITICAL**: This must stay in sync with production code!

use vstd::prelude::*;

verus! {

// Capability rights - EXACT copy from production (converted for Verus)
// Production: kernel/src/objects/capability.rs:258
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct CapRights {
    pub bits: u8
}

impl CapRights {
    // Rights constants - EXACT production code
    // Production: kernel/src/objects/capability.rs:262-271
    pub open spec fn READ() -> Self {
        CapRights { bits: 0b0001 }
    }

    pub open spec fn WRITE() -> Self {
        CapRights { bits: 0b0010 }
    }

    pub open spec fn GRANT() -> Self {
        CapRights { bits: 0b0100 }
    }

    pub open spec fn ALL() -> Self {
        CapRights { bits: 0b0111 }
    }

    // Create empty rights - EXACT production code
    // Production: kernel/src/objects/capability.rs:274-276
    pub fn empty() -> (result: Self)
        ensures result.bits == 0
    {
        CapRights { bits: 0 }
    }

    // Check if contains rights - EXACT production code
    // Production: kernel/src/objects/capability.rs:280-282
    pub fn contains(self, other: Self) -> (result: bool)
        ensures result == ((self.bits & other.bits) == other.bits)
    {
        (self.bits & other.bits) == other.bits
    }

    // Get raw bits - EXACT production code
    // Production: kernel/src/objects/capability.rs:286-288
    pub open spec fn bits_spec(self) -> u8 {
        self.bits
    }

    pub fn get_bits(self) -> (result: u8)
        ensures result == self.bits
    {
        self.bits
    }
}

} // verus!

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let empty = CapRights::empty();
        assert_eq!(empty.get_bits(), 0);
    }

    #[test]
    fn test_contains() {
        let all = CapRights { bits: 0b0111 };
        let read = CapRights { bits: 0b0001 };
        let write = CapRights { bits: 0b0010 };
        let grant = CapRights { bits: 0b0100 };

        // ALL contains everything
        assert!(all.contains(read));
        assert!(all.contains(write));
        assert!(all.contains(grant));
        assert!(all.contains(all));

        // READ doesn't contain WRITE
        assert!(!read.contains(write));

        // Empty contains only empty
        let empty = CapRights::empty();
        assert!(empty.contains(empty));
        assert!(!empty.contains(read));
    }

    #[test]
    fn test_reflexive() {
        let read = CapRights { bits: 0b0001 };
        assert!(read.contains(read));

        let write = CapRights { bits: 0b0010 };
        assert!(write.contains(write));
    }

    #[test]
    fn test_combined_rights() {
        let rw = CapRights { bits: 0b0011 }; // READ | WRITE
        let read = CapRights { bits: 0b0001 };
        let write = CapRights { bits: 0b0010 };

        assert!(rw.contains(read));
        assert!(rw.contains(write));
        assert!(!rw.contains(CapRights { bits: 0b0100 })); // Doesn't contain GRANT
    }
}
