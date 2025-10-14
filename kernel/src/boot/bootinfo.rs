//! Boot information structure and global storage
//!
//! This module defines the BootInfo structure that contains essential boot parameters
//! passed from the elfloader to the kernel. It provides safe access to boot information
//! throughout the kernel lifecycle.

use crate::memory::PhysAddr;

/// Boot information passed from elfloader to kernel
///
/// The elfloader passes these parameters via ARM64 registers (x0-x5):
/// - x0 = root_task_start: Physical start address of root task image
/// - x1 = root_task_end: Physical end address of root task image
/// - x2 = pv_offset: Physical-to-virtual offset for address translation
/// - x3 = root_task_entry: Virtual entry point of root task
/// - x4 = dtb_addr: Physical address of device tree blob
/// - x5 = dtb_size: Size of device tree blob in bytes
#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    /// Physical start address of root task ELF image
    pub root_task_start: PhysAddr,

    /// Physical end address of root task ELF image
    pub root_task_end: PhysAddr,

    /// Physical-to-virtual offset for address translation
    pub pv_offset: usize,

    /// Virtual entry point of root task (from ELF header)
    pub root_task_entry: usize,

    /// Physical address of device tree blob
    pub dtb_addr: PhysAddr,

    /// Size of device tree blob in bytes
    pub dtb_size: usize,
}

impl BootInfo {
    /// Create a new BootInfo from raw boot parameters
    pub const fn new(
        root_task_start: PhysAddr,
        root_task_end: PhysAddr,
        pv_offset: usize,
        root_task_entry: usize,
        dtb_addr: PhysAddr,
        dtb_size: usize,
    ) -> Self {
        Self {
            root_task_start,
            root_task_end,
            pv_offset,
            root_task_entry,
            dtb_addr,
            dtb_size,
        }
    }

    /// Get the size of the root task image in bytes
    pub fn root_task_size(&self) -> usize {
        self.root_task_end - self.root_task_start
    }

    /// Check if boot info is valid (non-zero addresses)
    pub fn is_valid(&self) -> bool {
        self.root_task_start.as_usize() != 0
            && self.root_task_end > self.root_task_start
            && self.root_task_entry != 0
            && self.dtb_addr.as_usize() != 0
            && self.dtb_size > 0
    }
}

/// Global boot information storage
static mut BOOT_INFO: Option<BootInfo> = None;

/// Initialize global boot info (called once during kernel initialization)
///
/// # Safety
/// Must be called exactly once during kernel initialization before any other
/// code attempts to read boot info.
pub unsafe fn init_boot_info(boot_info: BootInfo) {
    BOOT_INFO = Some(boot_info);
}

/// Get reference to global boot info
///
/// Returns None if boot info hasn't been initialized yet.
pub fn get_boot_info() -> Option<BootInfo> {
    unsafe { BOOT_INFO }
}

/// Get reference to global boot info or panic
///
/// # Panics
/// Panics if boot info hasn't been initialized.
pub fn get_boot_info_or_panic() -> BootInfo {
    get_boot_info().expect("Boot info not initialized")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootinfo_creation() {
        let boot_info = BootInfo::new(
            0x41000000, // root_task_start
            0x41010000, // root_task_end (64KB)
            0,          // pv_offset
            0x41000000, // root_task_entry
            0x40000000, // dtb_addr
            8192,       // dtb_size (8KB)
        );

        assert_eq!(boot_info.root_task_start, 0x41000000);
        assert_eq!(boot_info.root_task_end, 0x41010000);
        assert_eq!(boot_info.root_task_size(), 0x10000); // 64KB
        assert_eq!(boot_info.dtb_size, 8192);
        assert!(boot_info.is_valid());
    }

    #[test]
    fn test_bootinfo_invalid() {
        let boot_info = BootInfo::new(0, 0, 0, 0, 0, 0);
        assert!(!boot_info.is_valid());
    }

    #[test]
    fn test_global_boot_info() {
        let boot_info = BootInfo::new(
            0x41000000,
            0x41010000,
            0,
            0x41000000,
            0x40000000,
            8192,
        );

        unsafe {
            init_boot_info(boot_info);
        }

        let retrieved = get_boot_info().expect("Boot info should be initialized");
        assert_eq!(retrieved.root_task_start, 0x41000000);
        assert_eq!(retrieved.dtb_size, 8192);
    }
}
