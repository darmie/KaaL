//! Memory Manager
//!
//! Manages memory allocation from untyped regions.

use crate::{BrokerError, Result, boot_info::BootInfo};

/// Memory region
#[derive(Debug)]
pub struct MemoryRegion {
    /// Physical address
    pub phys_addr: usize,
    /// Size in bytes
    pub size: usize,
    /// Capability slot
    pub cap_slot: usize,
}

/// Memory Manager
pub struct MemoryManager {
    /// Copy of boot info for untyped regions
    boot_info: Option<&'static BootInfo>,
    /// Next untyped region to allocate from
    next_untyped_idx: usize,
}

impl MemoryManager {
    /// Create from boot info
    pub(crate) fn new_from_boot_info(boot_info: &'static BootInfo) -> Self {
        Self {
            boot_info: Some(boot_info),
            next_untyped_idx: 0,
        }
    }

    /// Create new (legacy, for tests)
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self {
            boot_info: None,
            next_untyped_idx: 0,
        }
    }

    /// Allocate memory
    pub(crate) fn allocate(&mut self, size: usize, cap_slot: usize) -> Result<MemoryRegion> {
        // Make syscall to kernel
        let phys_addr = unsafe {
            let mut addr: usize;
            core::arch::asm!(
                "mov x8, {syscall_num}",
                "mov x0, {size}",
                "svc #0",
                "mov {result}, x0",
                syscall_num = in(reg) 0x11u64, // SYS_MEMORY_ALLOCATE
                size = in(reg) size,
                result = out(reg) addr,
                out("x8") _,
                out("x0") _,
            );
            addr
        };

        if phys_addr == usize::MAX {
            return Err(BrokerError::OutOfMemory);
        }

        Ok(MemoryRegion {
            phys_addr,
            size,
            cap_slot,
        })
    }
}
