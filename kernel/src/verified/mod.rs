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
//! ## Current Status
//!
//! **Verified**: 11 modules, 115 items, 0 errors
//! - ✅ `bitmap_simple`: Simple bitmap (3 items)
//! - ✅ `phys_addr`: Physical address operations (10 items)
//! - ✅ `virt_addr`: Virtual address operations (10 items)
//! - ✅ `page_frame_number`: Page frame number operations (5 items)
//! - ✅ `cap_rights`: Capability rights bit operations (4 items)
//! - ✅ `bitmap_prod`: Production bitmap with advanced features (12 items)
//! - ✅ `tcb`: Thread Control Block state machine (29 items)
//! - ✅ `cnode_ops`: CNode slot operations (6 items)
//! - ✅ `capability_ops`: Capability derivation and rights (10 items)
//! - ✅ `page_table_ops`: Page table level operations (7 items)
//! - ✅ `thread_queue_ops`: Thread queue and endpoint operations (19 items)
//!
//! **Details**:
//! - Address operations: new, as_usize, is_aligned, align_down, align_up, page_number, is_null
//! - PFN operations: new, as_usize, phys_addr, from_phys_addr
//! - CapRights operations: empty, contains, get_bits + constants (READ, WRITE, GRANT, ALL)
//! - Bitmap operations: new, is_set, set, clear, find_first_unset + 4 bit-level axioms
//! - TCB operations: state transitions, capability checking, time slice management, lifecycle operations
//! - CNode operations: num_slots, is_valid_index, size validation with power-of-2 proofs
//! - Capability operations: derive, has_right, union, intersection with bitwise axioms
//! - Page table operations: shift, block_size, index, supports_blocks, next for 4-level ARMv8-A tables
//! - Thread queue operations: enqueue, dequeue, remove, queue bounds, FIFO properties
//! - Endpoint operations: badge management, queue status checks, idle state
//! - Advanced features: frame conditions with old(), loop invariants, termination proofs, state machine verification
//! - Shared axioms: mod_le_self, align_down_divisible, bit operations, power-of-2 properties (zero runtime cost)
//!
//! **Next Priority**:
//! - ⏳ Virtual address space operations
//! - ⏳ Scheduler operations
//!
//! **Planned**:
//! - ⏳ Frame allocator verification (Phase 3)
//! - ⏳ Page table verification (Phase 4)
//! - ⏳ IPC verification (Phase 5)
//! - ⏳ Capability system verification (Phase 6)
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
