//! Simple bump allocator for root-task
//!
//! This allocator is used to provide heap allocation for root-task,
//! enabling use of alloc-based collections like Vec and BTreeMap.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;

/// Simple bump allocator for root-task
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

/// Root-task heap region (256KB at 32MB mark)
/// This is placed in high memory to avoid conflicts with loaded components
const HEAP_START: usize = 0x200_0000; // 32MB
const HEAP_SIZE: usize = 0x40000;     // 256KB

/// Global allocator instance for root-task
#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new(HEAP_START, HEAP_SIZE);

/// Initialize the allocator (can be called explicitly if needed)
pub fn init() {
    // Nothing to do for bump allocator
    // Memory region is statically defined
}