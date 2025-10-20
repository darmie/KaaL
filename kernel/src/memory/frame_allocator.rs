//! Physical frame allocator
//!
//! This module implements a bitmap-based physical frame allocator for managing
//! 4KB page frames. The allocator tracks which physical memory frames are free
//! or allocated.
//!
//! # Design (seL4-inspired)
//! - Simple bitmap allocator for physical frames
//! - Tracks memory regions from device tree
//! - Reserves kernel and boot loader regions
//! - O(n) allocation (scan for free frame)
//!
//! # Implementation Notes
//! - Uses modular Bitmap from bitmap.rs (supports optional verification)
//! - Each bit represents one 4KB page frame
//! - 1 = allocated, 0 = free
//!
//! # Future Optimizations
//! - Buddy allocator for better performance
//! - Free list for O(1) allocation
//! - NUMA-aware allocation

use crate::memory::address::{PhysAddr, PageFrameNumber, PAGE_SIZE};
use crate::memory::bitmap::{Bitmap, MAX_BITS};


/// Physical frame allocator
///
/// Tracks physical memory frames using a modular Bitmap.
/// Frame numbers are relative to ram_base (not absolute physical addresses).
pub struct FrameAllocator {
    /// Bitmap tracking frame allocation (1 = allocated, 0 = free)
    bitmap: Bitmap,

    /// Total number of frames managed by this allocator
    total_frames: usize,

    /// Number of free frames available
    free_frames: usize,

    /// Base physical address of RAM (frame 0 corresponds to this address)
    ram_base: usize,
}

impl FrameAllocator {
    /// Create a new empty frame allocator
    pub const fn new() -> Self {
        Self {
            bitmap: Bitmap::new(),
            total_frames: 0,
            free_frames: 0,
            ram_base: 0,
        }
    }

    /// Add a physical memory region to the allocator
    ///
    /// # Arguments
    /// - `start`: Physical address of the start of the region
    /// - `size`: Size of the region in bytes
    pub fn add_region(&mut self, start: PhysAddr, size: usize) {
        // Set RAM base on first call
        if self.ram_base == 0 {
            self.ram_base = start.as_usize();
        }

        // Convert to frame numbers relative to ram_base
        let start_frame = (start.as_usize() - self.ram_base) / PAGE_SIZE;
        let num_frames = size / PAGE_SIZE;
        let end_frame = start_frame + num_frames;

        // Mark all frames in this region as free
        for frame in start_frame..end_frame {
            if frame < MAX_BITS {
                self.bitmap.clear(frame); // 0 = free
            }
        }

        self.total_frames += num_frames;
        self.free_frames += num_frames;
    }

    /// Reserve a physical memory region (mark as allocated)
    ///
    /// Used to reserve kernel code, boot loader, and other pre-allocated regions.
    ///
    /// # Arguments
    /// - `start`: Physical address of the start of the region
    /// - `size`: Size of the region in bytes
    pub fn reserve_region(&mut self, start: PhysAddr, size: usize) {
        // Convert to frame numbers relative to ram_base
        let start_frame = (start.as_usize() - self.ram_base) / PAGE_SIZE;
        let num_frames = (size + PAGE_SIZE - 1) / PAGE_SIZE;
        let end_frame = start_frame + num_frames;

        for frame in start_frame..end_frame {
            if frame < MAX_BITS && !self.bitmap.is_set(frame) {
                self.bitmap.set(frame); // 1 = allocated
                self.free_frames = self.free_frames.saturating_sub(1);
            }
        }
    }

    /// Allocate a physical frame
    ///
    /// Returns the page frame number of the allocated frame, or None if
    /// no frames are available.
    pub fn alloc(&mut self) -> Option<PageFrameNumber> {
        if self.free_frames == 0 {
            return None;
        }

        // Use bitmap's find_first_unset to find a free frame
        // Note: bitmap uses 1=allocated, 0=free
        if let Some(frame) = self.bitmap.find_first_unset(self.total_frames) {
            self.bitmap.set(frame); // Mark as allocated
            self.free_frames -= 1;

            // Convert relative frame number to absolute physical address
            let phys_addr = self.ram_base + (frame * PAGE_SIZE);
            return Some(PageFrameNumber::from_phys_addr(PhysAddr::new(phys_addr)));
        }

        None
    }

    /// Deallocate a physical frame
    ///
    /// # Safety
    /// - The frame must have been allocated by this allocator
    /// - The frame must not be in use
    pub fn dealloc(&mut self, pfn: PageFrameNumber) {
        // Convert absolute PFN to relative frame number
        let phys_addr = pfn.phys_addr().as_usize();
        if phys_addr < self.ram_base {
            return; // Invalid address
        }
        let frame = (phys_addr - self.ram_base) / PAGE_SIZE;
        if frame < MAX_BITS && self.bitmap.is_set(frame) {
            self.bitmap.clear(frame); // 0 = free
            self.free_frames += 1;
        }
    }

    /// Get the number of free frames
    pub fn free_frames(&self) -> usize {
        self.free_frames
    }

    /// Get the total number of frames
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_allocator_basic() {
        let mut allocator = FrameAllocator::new();

        // Add a 1MB region starting at 1MB
        allocator.add_region(PhysAddr::new(0x100000), 1024 * 1024);
        assert_eq!(allocator.total_frames(), 256); // 1MB / 4KB
        assert_eq!(allocator.free_frames(), 256);

        // Allocate a frame
        let frame1 = allocator.alloc().unwrap();
        assert_eq!(allocator.free_frames(), 255);

        // Allocate another frame
        let frame2 = allocator.alloc().unwrap();
        assert_eq!(allocator.free_frames(), 254);
        assert_ne!(frame1, frame2);

        // Deallocate first frame
        allocator.dealloc(frame1);
        assert_eq!(allocator.free_frames(), 255);

        // Deallocate second frame
        allocator.dealloc(frame2);
        assert_eq!(allocator.free_frames(), 256);
    }

    #[test]
    fn test_frame_allocator_reserve() {
        let mut allocator = FrameAllocator::new();

        // Add a 1MB region
        allocator.add_region(PhysAddr::new(0x100000), 1024 * 1024);
        let initial_free = allocator.free_frames();

        // Reserve first 64KB (16 pages)
        allocator.reserve_region(PhysAddr::new(0x100000), 64 * 1024);
        assert_eq!(allocator.free_frames(), initial_free - 16);
    }
}
