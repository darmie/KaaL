//! Memory management
//!
//! Higher-level abstractions for memory allocation and mapping.

use crate::{Result, syscall};

/// Memory permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    bits: usize,
}

impl Permissions {
    /// Read permission
    pub const READ: Self = Self { bits: 0x1 };
    /// Write permission
    pub const WRITE: Self = Self { bits: 0x2 };
    /// Execute permission
    pub const EXEC: Self = Self { bits: 0x4 };
    /// Read + Write
    pub const RW: Self = Self { bits: 0x3 };
    /// Read + Execute
    pub const RX: Self = Self { bits: 0x5 };
    /// Read + Write + Execute
    pub const RWX: Self = Self { bits: 0x7 };

    /// Get raw permission bits
    pub fn bits(&self) -> usize {
        self.bits
    }

    /// Combine permissions
    pub fn or(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }
}

/// Physical memory allocation
///
/// Represents a physical memory frame allocated from the kernel.
///
/// # Example
/// ```no_run
/// use kaal_sdk::memory::PhysicalMemory;
///
/// let mem = PhysicalMemory::allocate(4096)?;
/// println!("Allocated at phys: {:#x}", mem.phys_addr());
/// ```
pub struct PhysicalMemory {
    phys_addr: usize,
    size: usize,
}

impl PhysicalMemory {
    /// Allocate physical memory
    ///
    /// # Arguments
    /// * `size` - Size in bytes (will be rounded up to page size)
    pub fn allocate(size: usize) -> Result<Self> {
        let phys_addr = syscall::memory_allocate(size)?;
        Ok(Self { phys_addr, size })
    }

    /// Get physical address
    pub fn phys_addr(&self) -> usize {
        self.phys_addr
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        self.size
    }
}

/// Mapped memory region
///
/// Represents memory that has been mapped into the virtual address space.
/// Automatically unmaps on drop.
///
/// # Example
/// ```no_run
/// use kaal_sdk::memory::{PhysicalMemory, MappedMemory, Permissions};
///
/// let phys = PhysicalMemory::allocate(4096)?;
/// let mapped = MappedMemory::map(phys.phys_addr(), 4096, Permissions::RW)?;
///
/// // Use mapped memory at mapped.virt_addr()
/// ```
pub struct MappedMemory {
    virt_addr: usize,
    size: usize,
}

impl MappedMemory {
    /// Map physical memory into virtual address space
    ///
    /// # Arguments
    /// * `phys_addr` - Physical address to map
    /// * `size` - Size in bytes
    /// * `permissions` - Memory permissions
    pub fn map(phys_addr: usize, size: usize, permissions: Permissions) -> Result<Self> {
        let virt_addr = syscall::memory_map(phys_addr, size, permissions.bits())?;
        Ok(Self { virt_addr, size })
    }

    /// Get virtual address
    pub fn virt_addr(&self) -> usize {
        self.virt_addr
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get as a pointer
    pub fn as_ptr<T>(&self) -> *const T {
        self.virt_addr as *const T
    }

    /// Get as a mutable pointer
    pub fn as_mut_ptr<T>(&self) -> *mut T {
        self.virt_addr as *mut T
    }

    /// Get as a byte slice (unsafe - caller must ensure proper alignment and validity)
    pub unsafe fn as_slice(&self) -> &[u8] {
        core::slice::from_raw_parts(self.as_ptr(), self.size)
    }

    /// Get as a mutable byte slice (unsafe - caller must ensure proper alignment and validity)
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        core::slice::from_raw_parts_mut(self.as_mut_ptr(), self.size)
    }
}

impl Drop for MappedMemory {
    fn drop(&mut self) {
        // Best effort unmap - ignore errors
        let _ = syscall::memory_unmap(self.virt_addr, self.size);
    }
}
