//! Memory management subsystem
//!
//! This module provides the memory management infrastructure for the KaaL microkernel,
//! including:
//! - Physical memory (frame) allocation
//! - Virtual memory (page table) management
//! - Kernel heap allocation
//!
//! # Design Philosophy (seL4-inspired)
//! - Type-safe address handling (PhysAddr vs VirtAddr)
//! - Explicit memory management with no hidden allocations
//! - Frame allocator tracks physical pages
//! - Page tables managed through explicit operations
//!
//! # Chapter 2 Components
//! - `address`: Type-safe physical and virtual addresses
//! - `frame_allocator`: Physical page frame allocator
//! - `paging`: Page table abstraction (TODO)
//! - `heap`: Kernel heap allocator (TODO)

pub mod address;
pub mod frame_allocator;
pub mod paging;
pub mod heap;

pub use address::{PhysAddr, VirtAddr, PageFrameNumber};
pub use address::{PAGE_SIZE, LARGE_PAGE_SIZE, HUGE_PAGE_SIZE};
pub use address::{KERNEL_BASE, USER_MAX};
pub use paging::{PageMapper, PageSize, MappingError};

use frame_allocator::FrameAllocator;
use crate::kprintln;

/// Global frame allocator (initialized during boot)
static FRAME_ALLOCATOR: spin::Once<spin::Mutex<FrameAllocator>> = spin::Once::new();

/// Initialize the memory subsystem
///
/// This must be called early during boot, after the DTB has been parsed
/// but before any dynamic memory allocation is needed.
///
/// # Safety
/// - Must be called exactly once during boot
/// - Must be called before any memory allocation
pub unsafe fn init(
    kernel_start: PhysAddr,
    kernel_end: PhysAddr,
    ram_start: PhysAddr,
    ram_size: usize,
) {
    kprintln!("[memory] Initializing memory subsystem");
    kprintln!("  RAM:    {:#x} - {:#x} ({}MB)",
        ram_start.as_usize(),
        ram_start.as_usize() + ram_size,
        ram_size / (1024 * 1024)
    );
    kprintln!("  Kernel: {:#x} - {:#x} ({}KB)",
        kernel_start.as_usize(),
        kernel_end.as_usize(),
        (kernel_end.as_usize() - kernel_start.as_usize()) / 1024
    );

    // Initialize frame allocator
    let mut allocator = FrameAllocator::new();
    allocator.add_region(ram_start, ram_size);

    // Reserve everything from RAM start up to end of kernel
    // This includes: DTB, elfloader, kernel code/data, and stack
    let reserved_size = kernel_end.as_usize() - ram_start.as_usize();
    allocator.reserve_region(ram_start, reserved_size);

    let free_frames = allocator.free_frames();
    let total_frames = allocator.total_frames();
    kprintln!("  Frames: {}/{} free ({}MB usable)",
        free_frames,
        total_frames,
        (free_frames * PAGE_SIZE) / (1024 * 1024)
    );

    FRAME_ALLOCATOR.call_once(|| spin::Mutex::new(allocator));
}

/// Allocate a physical frame
///
/// Returns None if no frames are available.
pub fn alloc_frame() -> Option<PageFrameNumber> {
    FRAME_ALLOCATOR
        .get()
        .and_then(|allocator| allocator.lock().alloc())
}

/// Deallocate a physical frame
///
/// # Safety
/// - The frame must have been allocated by `alloc_frame`
/// - The frame must not be in use
pub unsafe fn dealloc_frame(pfn: PageFrameNumber) {
    if let Some(allocator) = FRAME_ALLOCATOR.get() {
        allocator.lock().dealloc(pfn);
    }
}

/// Get memory statistics
pub fn memory_stats() -> Option<(usize, usize)> {
    FRAME_ALLOCATOR.get().map(|allocator| {
        let lock = allocator.lock();
        (lock.free_frames(), lock.total_frames())
    })
}
