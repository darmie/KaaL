//! CDT Node Allocator
//!
//! This module provides allocation and deallocation of CDT nodes.
//!
//! ## Design
//!
//! We use a simple bump allocator for CDT nodes during Phase 2.
//! This can be upgraded to a more sophisticated allocator (slab, buddy) later.
//!
//! ## Memory Layout
//!
//! CDT nodes are allocated from a dedicated memory region:
//! - Size: CapNode = 56 bytes (32 byte cap + 24 bytes tree pointers)
//! - Alignment: 8 bytes (pointer alignment)
//! - Capacity: Configurable at init (default: 4096 nodes = 224 KB)

use super::cdt::CapNode;
use crate::memory::{PhysAddr, PageFrameNumber};
use core::sync::atomic::{AtomicUsize, Ordering};

/// CDT allocator configuration
pub struct CdtAllocatorConfig {
    /// Base physical address of CDT region
    pub base: PhysAddr,
    /// Total size in bytes
    pub size: usize,
}

impl CdtAllocatorConfig {
    /// Create config with default size (4096 nodes = ~224 KB)
    pub const fn with_capacity(base: PhysAddr, node_count: usize) -> Self {
        Self {
            base,
            size: node_count * core::mem::size_of::<CapNode>(),
        }
    }

    /// Maximum number of nodes this config can hold
    pub const fn max_nodes(&self) -> usize {
        self.size / core::mem::size_of::<CapNode>()
    }
}

/// Simple bump allocator for CDT nodes
///
/// This allocator:
/// - Allocates CDT nodes sequentially from a fixed region
/// - Does NOT support deallocation (simplified for now)
/// - Thread-safe via atomic operations
///
/// **Future**: Upgrade to slab allocator with free list
pub struct CdtAllocator {
    /// Base address of CDT region
    base: usize,
    /// Current allocation offset (in bytes)
    offset: AtomicUsize,
    /// Total size of region
    size: usize,
}

impl CdtAllocator {
    /// Create a new CDT allocator (uninitialized)
    pub const fn new() -> Self {
        CdtAllocator {
            base: 0,
            offset: AtomicUsize::new(0),
            size: 0,
        }
    }

    /// Initialize the allocator with a memory region
    ///
    /// # Safety
    /// - Must be called exactly once
    /// - `config.base` must point to valid, unused physical memory
    /// - Memory region must not overlap with other kernel structures
    pub unsafe fn init(&mut self, config: CdtAllocatorConfig) {
        self.base = config.base.as_usize();
        self.offset.store(0, Ordering::Relaxed);
        self.size = config.size;

        crate::kprintln!("[cdt] Initialized CDT allocator:");
        crate::kprintln!("      Base: {:#x}", self.base);
        crate::kprintln!("      Size: {} bytes ({} nodes max)",
                        self.size, config.max_nodes());
    }

    /// Allocate a CDT node
    ///
    /// Returns a pointer to uninitialized memory for a CapNode.
    ///
    /// # Returns
    /// - `Some(*mut CapNode)` if allocation succeeded
    /// - `None` if out of memory
    ///
    /// # Safety
    /// Caller must initialize the returned memory before use
    pub fn alloc(&self) -> Option<*mut CapNode> {
        let node_size = core::mem::size_of::<CapNode>();
        let node_align = core::mem::align_of::<CapNode>();

        // Atomically reserve space
        let current = self.offset.fetch_add(node_size, Ordering::Relaxed);

        // Check if we exceeded capacity
        if current + node_size > self.size {
            // Out of memory - restore offset
            self.offset.fetch_sub(node_size, Ordering::Relaxed);
            return None;
        }

        // Calculate aligned address
        let addr = self.base + current;
        let aligned_addr = (addr + node_align - 1) & !(node_align - 1);

        Some(aligned_addr as *mut CapNode)
    }

    /// Deallocate a CDT node
    ///
    /// # Note
    /// Current implementation (bump allocator) does NOT actually free memory.
    /// This is a placeholder for future slab allocator implementation.
    ///
    /// # Safety
    /// - `ptr` must have been allocated by this allocator
    /// - `ptr` must not be used after deallocation
    pub unsafe fn dealloc(&self, _ptr: *mut CapNode) {
        // Bump allocator: no actual deallocation
        // TODO: Implement free list when upgrading to slab allocator
    }

    /// Get current allocation statistics
    pub fn stats(&self) -> CdtAllocatorStats {
        let used_bytes = self.offset.load(Ordering::Relaxed);
        let node_size = core::mem::size_of::<CapNode>();

        CdtAllocatorStats {
            nodes_allocated: used_bytes / node_size,
            bytes_used: used_bytes,
            bytes_free: self.size.saturating_sub(used_bytes),
            total_capacity: self.size / node_size,
        }
    }

    /// Check if allocator is initialized
    pub fn is_initialized(&self) -> bool {
        self.base != 0 && self.size != 0
    }
}

/// CDT allocator statistics
#[derive(Debug, Clone, Copy)]
pub struct CdtAllocatorStats {
    /// Number of nodes allocated
    pub nodes_allocated: usize,
    /// Bytes currently used
    pub bytes_used: usize,
    /// Bytes remaining
    pub bytes_free: usize,
    /// Total node capacity
    pub total_capacity: usize,
}

impl CdtAllocatorStats {
    /// Calculate utilization percentage
    pub fn utilization(&self) -> usize {
        if self.total_capacity == 0 {
            0
        } else {
            (self.nodes_allocated * 100) / self.total_capacity
        }
    }
}

/// Global CDT allocator instance
static mut CDT_ALLOCATOR: CdtAllocator = CdtAllocator::new();

/// Initialize the global CDT allocator
///
/// # Safety
/// Must be called exactly once during kernel initialization
pub unsafe fn init_cdt_allocator(config: CdtAllocatorConfig) {
    CDT_ALLOCATOR.init(config);
}

/// Allocate a CDT node from the global allocator
///
/// # Returns
/// Pointer to uninitialized CapNode memory, or None if out of memory
///
/// # Safety
/// Caller must initialize the returned memory
pub fn alloc_cdt_node() -> Option<*mut CapNode> {
    unsafe { CDT_ALLOCATOR.alloc() }
}

/// Deallocate a CDT node
///
/// # Safety
/// - `ptr` must have been allocated by `alloc_cdt_node`
/// - `ptr` must not be used after this call
pub unsafe fn dealloc_cdt_node(ptr: *mut CapNode) {
    CDT_ALLOCATOR.dealloc(ptr);
}

/// Get CDT allocator statistics
pub fn cdt_allocator_stats() -> CdtAllocatorStats {
    unsafe { CDT_ALLOCATOR.stats() }
}

/// Check if CDT allocator is initialized
pub fn is_cdt_allocator_initialized() -> bool {
    unsafe { CDT_ALLOCATOR.is_initialized() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocator_init() {
        let mut allocator = CdtAllocator::new();
        assert!(!allocator.is_initialized());

        unsafe {
            let config = CdtAllocatorConfig::with_capacity(
                PhysAddr::new(0x1000000),
                100
            );
            allocator.init(config);
        }

        assert!(allocator.is_initialized());
        let stats = allocator.stats();
        assert_eq!(stats.nodes_allocated, 0);
        assert_eq!(stats.total_capacity, 100);
    }

    #[test]
    fn test_alloc() {
        let mut allocator = CdtAllocator::new();
        unsafe {
            let config = CdtAllocatorConfig::with_capacity(
                PhysAddr::new(0x1000000),
                10
            );
            allocator.init(config);
        }

        // Allocate first node
        let node1 = allocator.alloc();
        assert!(node1.is_some());

        // Allocate second node
        let node2 = allocator.alloc();
        assert!(node2.is_some());

        // Nodes should have different addresses
        assert_ne!(node1.unwrap(), node2.unwrap());

        let stats = allocator.stats();
        assert_eq!(stats.nodes_allocated, 2);
    }

    #[test]
    fn test_out_of_memory() {
        let mut allocator = CdtAllocator::new();
        unsafe {
            let config = CdtAllocatorConfig::with_capacity(
                PhysAddr::new(0x1000000),
                2  // Only 2 nodes
            );
            allocator.init(config);
        }

        // Allocate all available nodes
        assert!(allocator.alloc().is_some());
        assert!(allocator.alloc().is_some());

        // Third allocation should fail
        assert!(allocator.alloc().is_none());
    }

    #[test]
    fn test_stats_utilization() {
        let mut allocator = CdtAllocator::new();
        unsafe {
            let config = CdtAllocatorConfig::with_capacity(
                PhysAddr::new(0x1000000),
                100
            );
            allocator.init(config);
        }

        // Allocate 25 nodes
        for _ in 0..25 {
            allocator.alloc();
        }

        let stats = allocator.stats();
        assert_eq!(stats.utilization(), 25); // 25% utilization
    }
}
