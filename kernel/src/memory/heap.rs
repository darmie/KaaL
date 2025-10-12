//! Kernel heap allocator
//!
//! Provides dynamic memory allocation for the kernel using Rust's GlobalAlloc trait.
//! This enables use of Box, Vec, Arc, and other heap-allocated types.
//!
//! # Design
//! - Simple linked-list allocator for simplicity
//! - Locks using spin::Mutex for thread safety
//! - Allocated from a fixed heap region
//!
//! # Future Improvements
//! - Buddy allocator for better performance
//! - Slab allocator for common object sizes
//! - Per-CPU heaps to reduce contention

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use spin::Mutex;

/// Heap size (1MB for now)
const HEAP_SIZE: usize = 1024 * 1024;

/// Heap memory region
static mut HEAP_MEMORY: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

/// Free block in the heap
struct FreeBlock {
    size: usize,
    next: Option<&'static mut FreeBlock>,
}

impl FreeBlock {
    /// Create a new free block at the given address
    unsafe fn new(addr: usize, size: usize) -> &'static mut Self {
        let block = addr as *mut FreeBlock;
        (*block).size = size;
        (*block).next = None;
        &mut *block
    }

    /// Get the address of this block
    fn addr(&self) -> usize {
        self as *const _ as usize
    }
}

/// Linked-list heap allocator
pub struct LinkedListAllocator {
    head: Option<&'static mut FreeBlock>,
}

impl LinkedListAllocator {
    /// Create a new empty allocator
    pub const fn new() -> Self {
        Self { head: None }
    }

    /// Initialize the allocator with a heap region
    ///
    /// # Safety
    /// - The heap region must be valid and unused
    /// - This should only be called once
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.head = Some(FreeBlock::new(heap_start, heap_size));
    }

    /// Allocate memory with the given layout
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        unsafe {
            // Find a suitable free block
            let mut current_ptr: *mut Option<&'static mut FreeBlock> = &mut self.head;
            let mut iterations = 0;

            loop {
                iterations += 1;
                if iterations > 1000 {
                    // Prevent infinite loop
                    return null_mut();
                }

                if (*current_ptr).is_none() {
                    return null_mut();
                }

                let block = (*current_ptr).as_mut().unwrap();
                let block_start = block.addr();
                let block_end = block_start + block.size;

                // Align the allocation
                let alloc_start = align_up(block_start, layout.align());
                let alloc_end = alloc_start.saturating_add(layout.size());

                if alloc_end <= block_end {
                    // Block is large enough
                    let remaining_size = block_end - alloc_end;
                    let next = block.next.take();

                    if remaining_size > 0 && remaining_size >= core::mem::size_of::<FreeBlock>() {
                        // Split the block
                        let new_block = FreeBlock::new(alloc_end, remaining_size);
                        new_block.next = next;
                        *current_ptr = Some(new_block);
                    } else {
                        // Use the entire block
                        *current_ptr = next;
                    }

                    return alloc_start as *mut u8;
                }

                // Move to next block
                current_ptr = &mut block.next as *mut Option<&'static mut FreeBlock>;
            }
        }
    }

    /// Deallocate memory at the given address
    ///
    /// # Safety
    /// - The pointer must have been allocated by this allocator
    /// - The layout must match the original allocation
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let addr = ptr as usize;
        let size = layout.size();

        // Create a new free block
        let new_block = FreeBlock::new(addr, size);

        // Insert into free list (simplified - no coalescing)
        new_block.next = self.head.take();
        self.head = Some(new_block);
    }

    /// Get the total size of free memory
    pub fn free_size(&self) -> usize {
        let mut total = 0;
        let mut current = self.head.as_ref();

        while let Some(block) = current {
            total += block.size;
            current = block.next.as_ref();
        }

        total
    }
}

/// Global heap allocator instance
static ALLOCATOR: Mutex<LinkedListAllocator> = Mutex::new(LinkedListAllocator::new());

/// Global allocator implementation
struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATOR.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATOR.lock().dealloc(ptr, layout)
    }
}

/// Set the global allocator
#[global_allocator]
static GLOBAL_ALLOCATOR: KernelAllocator = KernelAllocator;

/// Initialize the kernel heap
///
/// # Safety
/// - Must be called exactly once during boot
/// - Must be called before any heap allocation
pub unsafe fn init() {
    let heap_start = HEAP_MEMORY.as_ptr() as usize;
    ALLOCATOR.lock().init(heap_start, HEAP_SIZE);
}

/// Get the amount of free heap memory
pub fn free_memory() -> usize {
    ALLOCATOR.lock().free_size()
}

/// Align value up to alignment
#[inline]
fn align_up(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}

/// Allocation error handler (called when out of memory)
#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    crate::kprintln!("FATAL: Heap allocation failed!");
    crate::kprintln!("  Requested: {} bytes, align: {}", layout.size(), layout.align());
    crate::kprintln!("  Free heap: {} bytes", free_memory());
    panic!("Out of memory")
}
