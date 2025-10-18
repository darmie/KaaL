//! Shared bump allocator for KaaL runtime
//!
//! This allocator is used by root-task, IPC library, and other runtime components
//! that need heap allocation in a no_std environment.

#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;

/// Simple bump allocator for runtime components
///
/// This allocator allocates from a fixed-size heap and never frees memory.
/// It's suitable for long-lived runtime components that don't need deallocation.
pub struct BumpAllocator {
    heap_start: UnsafeCell<usize>,
    heap_end: usize,
    next: UnsafeCell<usize>,
}

unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    /// Create a new bump allocator with given heap region
    pub const fn new(heap_start: usize, heap_size: usize) -> Self {
        Self {
            heap_start: UnsafeCell::new(heap_start),
            heap_end: heap_start + heap_size,
            next: UnsafeCell::new(heap_start),
        }
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        // Get current allocation pointer
        let next = self.next.get();
        let alloc_start = (*next + align - 1) & !(align - 1); // Align up
        let alloc_end = alloc_start + size;

        // Check if we have enough space
        if alloc_end > self.heap_end {
            return ptr::null_mut();
        }

        // Update next pointer
        *next = alloc_end;

        alloc_start as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // No-op: bump allocator doesn't free memory
    }
}
