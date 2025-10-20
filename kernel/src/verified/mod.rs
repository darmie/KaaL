//! Verified Kernel Modules
//!
//! This module contains mathematically verified implementations
//! of critical kernel data structures and algorithms.
//!
//! ## Verification Approach
//!
//! We use [Verus](https://github.com/verus-lang/verus) to verify correctness properties.
//! Each module in this directory has:
//! - **Specifications**: What the code should do (spec functions)
//! - **Implementations**: How the code works (exec functions)
//! - **Proofs**: Why the code is correct (proof blocks)
//!
//! ## Verified Modules
//!
//! - `bitmap`: Simple bitmap operations (learning example)
//! - `frame`: Frame allocator (Phase 2)
//! - `pagetable`: Page table operations (Phase 2)
//! - `ipc`: IPC message passing (Phase 3)
//! - `capability`: Capability system (Phase 4)
//!
//! ## Usage
//!
//! Verified modules can be used alongside unverified code:
//!
//! ```rust
//! use kernel::verified::bitmap::Bitmap;
//!
//! let mut bm = Bitmap::new();  // Verified to be all zeros
//! bm.set(5);                    // Verified to set only bit 5
//! assert!(bm.is_set(5));        // Verified to return true
//! ```
//!
//! ## Verification Status
//!
//! See [docs/verification/](../../docs/verification/) for:
//! - Setup instructions
//! - Verification workflow
//! - Proof techniques
//! - Status tracking
//!
//! ## Note on Feature Flags
//!
//! Verified code uses conditional compilation:
//! - With `verification` feature: Uses Verus (`vstd` library)
//! - Without `verification` feature: Standard Rust (no verification)

#[cfg(feature = "verification")]
pub mod bitmap;

#[cfg(not(feature = "verification"))]
pub mod bitmap {
    //! Non-verified bitmap (fallback when verification disabled)
    //!
    //! This provides the same API but without verification overhead.
    //! Used when building the actual kernel binary.

    pub struct Bitmap {
        bits: [bool; 64],
    }

    impl Bitmap {
        pub fn new() -> Self {
            Bitmap { bits: [false; 64] }
        }

        pub fn set(&mut self, index: usize) {
            if index < 64 {
                self.bits[index] = true;
            }
        }

        pub fn clear(&mut self, index: usize) {
            if index < 64 {
                self.bits[index] = false;
            }
        }

        pub fn is_set(&self, index: usize) -> bool {
            index < 64 && self.bits[index]
        }

        pub fn find_first_unset(&self) -> Option<usize> {
            for i in 0..64 {
                if !self.bits[i] {
                    return Some(i);
                }
            }
            None
        }
    }
}
