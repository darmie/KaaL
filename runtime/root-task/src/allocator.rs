//! Root-task allocator configuration
//!
//! Uses the shared kaal_allocator with root-task specific heap region.

use kaal_allocator::BumpAllocator;

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