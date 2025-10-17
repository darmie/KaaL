//! Simple bump allocator for components
//!
//! This allocator is suitable for components that don't need sophisticated
//! memory management. It allocates from a fixed-size heap and never frees.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;

/// Simple bump allocator
pub struct BumpAllocator {
    heap_start: UnsafeCell<usize>,
    heap_end: usize,
    next: UnsafeCell<usize>,
}

unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    /// Create a new bump allocator
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

/// Static heap for components (64KB) - starts at a fixed address
const HEAP_START: usize = 0x100_0000; // 16MB mark in virtual memory
const HEAP_SIZE: usize = 0x10000; // 64KB

/// Global allocator instance
#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new(HEAP_START, HEAP_SIZE);

/// Initialize the allocator (called by component startup)
pub fn init() {
    // Nothing to do for bump allocator
}