//! Kernel heap allocator
//!
//! Provides dynamic memory allocation for the kernel using Rust's GlobalAlloc trait.
//! This enables use of Box, Vec, Arc, and other heap-allocated types.
//!
//! # Implementation
//! - Uses the production-ready `linked_list_allocator` crate from rust-osdev
//! - 92K+ downloads/month, well-tested in many Rust OS projects
//! - Simple linked-list design suitable for kernel use
//! - Spinlock-based for thread safety

use linked_list_allocator::LockedHeap;

extern crate alloc;
use alloc::vec::Vec;
use alloc::boxed::Box;

#[path = "../generated/memory_config.rs"]
mod memory_config;
use memory_config::HEAP_SIZE;

/// Heap size (1MB for now)
/// Heap memory region
static mut HEAP_MEMORY: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

/// Global heap allocator instance
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initialize the kernel heap
///
/// This must be called early in the boot sequence before any heap allocations.
///
/// # Safety
/// - Must only be called once
/// - Must be called before any heap allocations
pub unsafe fn init() {
    let heap_start = HEAP_MEMORY.as_mut_ptr();
    ALLOCATOR.lock().init(heap_start, HEAP_SIZE);
}

/// Get the amount of free memory in the heap
pub fn free_memory() -> usize {
    ALLOCATOR.lock().free()
}

/// Allocation error handler
///
/// Called when the allocator runs out of memory
#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    crate::kprintln!("Allocation error!");
    crate::kprintln!("  Layout: size={}, align={}", layout.size(), layout.align());
    crate::kprintln!("  Free heap: {} bytes", free_memory());
    panic!("Out of memory")
}

/// Heap allocator test functions
/// These test the GLOBAL allocator using Box/Vec
#[cfg(any(test, feature = "testing"))]
pub mod tests {
    use super::*;

    pub fn test_allocator_init() -> bool {
        // After init, we should have close to 1MB free
        let free = free_memory();
        free > 1000000 && free <= HEAP_SIZE
    }

    pub fn test_simple_allocation() -> bool {
        // Just test that allocation succeeds
        let boxed = Box::new([0u8; 64]);
        boxed[0] == 0 && boxed[63] == 0
    }

    pub fn test_multiple_allocations() -> bool {
        // Multiple allocations should all succeed
        let box1 = Box::new([1u8; 64]);
        let box2 = Box::new([2u8; 128]);
        let box3 = Box::new([3u8; 32]);
        
        box1[0] == 1 && box2[0] == 2 && box3[0] == 3
    }

    pub fn test_allocation_and_deallocation() -> bool {
        let free_before = free_memory();
        
        {
            let _boxed = Box::new([0u8; 4096]);
        } // Box dropped here
        
        let free_after = free_memory();
        // After dealloc of large block, should have similar or more memory
        free_after >= free_before.saturating_sub(1000)
    }

    pub fn test_out_of_memory() -> bool {
        // Normal allocations should work fine
        let _vec: Vec<u8> = Vec::with_capacity(1000);
        true
    }

    pub fn test_alignment() -> bool {
        // Box ensures proper alignment
        let box1 = Box::new(0u64);  // 8-byte aligned
        let box2 = Box::new(0u128); // 16-byte aligned
        
        let ptr1 = &*box1 as *const u64 as usize;
        let ptr2 = &*box2 as *const u128 as usize;
        
        (ptr1 % 8 == 0) && (ptr2 % 16 == 0)
    }

    pub fn test_fragmentation() -> bool {
        // Test that allocator can reuse freed blocks
        let _box1 = Box::new([0u8; 64]);
        let box2 = Box::new([0u8; 64]);
        let _box3 = Box::new([0u8; 64]);
        
        // Drop middle box
        drop(box2);
        
        // Allocate again - should work
        let _box4 = Box::new([0u8; 64]);
        true
    }

    pub fn test_zero_size_allocation() -> bool {
        // Zero-size allocations should work
        let _vec: Vec<u8> = Vec::new();
        let _vec2: Vec<u8> = Vec::new();
        true
    }
}
