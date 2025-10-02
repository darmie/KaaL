//! Memory Allocator - Heap and DMA memory management
//!
//! # Purpose
//! Provides efficient memory allocation for system components using
//! buddy allocator algorithm for heap and dedicated pools for DMA.
//!
//! # Integration Points
//! - Depends on: Capability Broker (untyped memory)
//! - Provides to: All components requiring dynamic memory
//! - Capabilities required: Untyped memory capabilities
//!
//! # Architecture
//! - Buddy allocator for general heap (O(log n) allocation)
//! - Pool allocator for DMA (O(1) allocation, fixed sizes)
//! - Zero overhead for embedded use (no std dependency)
//!
//! # Testing Strategy
//! - Unit tests: Allocation patterns, fragmentation, alignment
//! - Integration tests: Stress tests, memory leaks
//! - Benchmarks: Allocation/deallocation latency

use thiserror::Error;

/// Memory allocation errors
#[derive(Debug, Error)]
pub enum AllocError {
    #[error("Out of memory (requested: {size} bytes)")]
    OutOfMemory { size: usize },

    #[error("Invalid alignment (must be power of 2): {alignment}")]
    InvalidAlignment { alignment: usize },

    #[error("Invalid size (must be > 0)")]
    InvalidSize,

    #[error("Double free detected at address {addr:#x}")]
    DoubleFree { addr: usize },
}

pub type Result<T> = core::result::Result<T, AllocError>;

/// Buddy allocator for general-purpose heap allocation
pub struct BuddyAllocator {
    base: usize,
    size: usize,
    // TODO: Implement buddy allocator data structures
}

impl BuddyAllocator {
    /// Create a new buddy allocator
    ///
    /// # Arguments
    /// * `base` - Base address of memory region
    /// * `size` - Total size in bytes (must be power of 2)
    ///
    /// # Errors
    /// Returns error if size is not power of 2
    pub fn new(base: usize, size: usize) -> Result<Self> {
        if !size.is_power_of_two() {
            return Err(AllocError::InvalidAlignment { alignment: size });
        }

        Ok(Self { base, size })
    }

    /// Allocate memory block
    ///
    /// # Arguments
    /// * `size` - Size in bytes
    /// * `align` - Required alignment (must be power of 2)
    ///
    /// # Returns
    /// Pointer to allocated memory
    ///
    /// # Errors
    /// Returns error if out of memory or invalid parameters
    pub fn allocate(&mut self, size: usize, align: usize) -> Result<*mut u8> {
        if size == 0 {
            return Err(AllocError::InvalidSize);
        }

        if !align.is_power_of_two() {
            return Err(AllocError::InvalidAlignment { alignment: align });
        }

        // TODO: Implement buddy allocation algorithm
        Err(AllocError::OutOfMemory { size })
    }

    /// Deallocate memory block
    ///
    /// # Arguments
    /// * `ptr` - Pointer returned by allocate
    /// * `size` - Original allocation size
    ///
    /// # Safety
    /// Caller must ensure:
    /// - ptr was returned by this allocator
    /// - size matches original allocation
    /// - No double frees
    ///
    /// # Errors
    /// Returns error if double free detected
    pub unsafe fn deallocate(&mut self, ptr: *mut u8, size: usize) -> Result<()> {
        // TODO: Implement buddy deallocation
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buddy_allocator_creation() {
        // Valid power-of-2 size
        let alloc = BuddyAllocator::new(0x1000, 4096);
        assert!(alloc.is_ok());

        // Invalid size (not power of 2)
        let alloc = BuddyAllocator::new(0x1000, 4000);
        assert!(matches!(alloc, Err(AllocError::InvalidAlignment { .. })));
    }

    #[test]
    fn test_invalid_allocations() {
        let mut alloc = BuddyAllocator::new(0x1000, 4096).unwrap();

        // Zero size
        assert!(matches!(
            alloc.allocate(0, 8),
            Err(AllocError::InvalidSize)
        ));

        // Invalid alignment
        assert!(matches!(
            alloc.allocate(64, 3),
            Err(AllocError::InvalidAlignment { .. })
        ));
    }
}
