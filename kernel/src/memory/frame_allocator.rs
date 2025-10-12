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
//! # Future Optimizations
//! - Buddy allocator for better performance
//! - Free list for O(1) allocation
//! - NUMA-aware allocation
//!
//! # Implementation Notes
//! - Uses a fixed-size bitmap (supports up to MAX_FRAMES physical frames)
//! - Each bit represents one 4KB page frame
//! - 1 = allocated, 0 = free

use crate::memory::address::{PhysAddr, PageFrameNumber, PAGE_SIZE};

/// Maximum number of physical frames we can track
/// For 1GB RAM: 1GB / 4KB = 256K frames = 32KB bitmap
const MAX_FRAMES: usize = 256 * 1024; // 1GB / 4KB

/// Physical frame allocator
///
/// Tracks physical memory frames using a bitmap.
/// Frame numbers are relative to ram_base (not absolute physical addresses).
pub struct FrameAllocator {
    /// Bitmap tracking frame allocation (1 = allocated, 0 = free)
    bitmap: [u64; MAX_FRAMES / 64],

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
            bitmap: [0; MAX_FRAMES / 64],
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
            if frame < MAX_FRAMES {
                self.mark_free(frame);
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
            if frame < MAX_FRAMES && self.is_free(frame) {
                self.mark_allocated(frame);
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

        // Scan bitmap for first free frame (simple linear search)
        for (chunk_idx, &chunk) in self.bitmap.iter().enumerate() {
            if chunk != !0 {
                // This chunk has at least one free frame
                for bit_idx in 0..64 {
                    let frame = chunk_idx * 64 + bit_idx;
                    if frame >= self.total_frames {
                        return None;
                    }

                    if self.is_free(frame) {
                        self.mark_allocated(frame);
                        self.free_frames -= 1;
                        // Convert relative frame number to absolute physical address
                        let phys_addr = self.ram_base + (frame * PAGE_SIZE);
                        return Some(PageFrameNumber::from_phys_addr(PhysAddr::new(phys_addr)));
                    }
                }
            }
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
        if frame < MAX_FRAMES && !self.is_free(frame) {
            self.mark_free(frame);
            self.free_frames += 1;
        }
    }

    /// Check if a frame is free
    #[inline]
    fn is_free(&self, frame: usize) -> bool {
        let chunk_idx = frame / 64;
        let bit_idx = frame % 64;
        (self.bitmap[chunk_idx] & (1u64 << bit_idx)) == 0
    }

    /// Mark a frame as allocated
    #[inline]
    fn mark_allocated(&mut self, frame: usize) {
        let chunk_idx = frame / 64;
        let bit_idx = frame % 64;
        self.bitmap[chunk_idx] |= 1u64 << bit_idx;
    }

    /// Mark a frame as free
    #[inline]
    fn mark_free(&mut self, frame: usize) {
        let chunk_idx = frame / 64;
        let bit_idx = frame % 64;
        self.bitmap[chunk_idx] &= !(1u64 << bit_idx);
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
