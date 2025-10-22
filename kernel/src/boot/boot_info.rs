//! Boot Information Structure
//!
//! This module defines the boot info structure passed from the kernel to the
//! root task. The boot info contains essential system information needed by
//! runtime services (Capability Broker, Memory Manager) to initialize.
//!
//! The boot info is placed in a known location in the root task's address space
//! and contains information about:
//! - Available untyped memory regions
//! - Device memory regions
//! - Initial capability slots
//! - System configuration

#![allow(dead_code)]

use core::mem::size_of;

/// Magic number to identify valid boot info (ASCII: "KAAL")
pub const BOOT_INFO_MAGIC: u32 = 0x4B41414C;

/// Boot info structure version
pub const BOOT_INFO_VERSION: u32 = 1;

/// Maximum number of untyped memory regions
pub const MAX_UNTYPED_REGIONS: usize = 128;

/// Maximum number of device regions
pub const MAX_DEVICE_REGIONS: usize = 32;

/// Maximum number of initial capability slots
pub const MAX_INITIAL_CAPS: usize = 256;

/// Untyped memory region descriptor
///
/// Describes a region of physical memory that can be retyped into kernel objects.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UntypedRegion {
    /// Physical address of the region
    pub paddr: u64,

    /// Size in bytes (must be power of 2)
    pub size_bits: u8,

    /// Whether this region is a device memory region
    pub is_device: bool,

    /// Reserved for alignment
    _reserved: [u8; 6],
}

impl UntypedRegion {
    pub fn new(paddr: u64, size_bits: u8, is_device: bool) -> Self {
        Self {
            paddr,
            size_bits,
            is_device,
            _reserved: [0; 6],
        }
    }

    pub fn size(&self) -> usize {
        1 << self.size_bits
    }
}

/// Device region descriptor
///
/// Describes a memory-mapped device region.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DeviceRegion {
    /// Physical address of the device MMIO region
    pub paddr: u64,

    /// Size in bytes
    pub size: u64,

    /// Device type identifier (platform-specific)
    pub device_type: u32,

    /// IRQ number (if applicable, otherwise 0xFFFFFFFF)
    pub irq: u32,
}

/// Initial capability slot descriptor
///
/// Describes a capability slot in the root task's initial CSpace.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CapabilitySlot {
    /// CSpace slot index
    pub slot: u64,

    /// Capability type
    pub cap_type: CapabilityType,

    /// Object address (physical or virtual depending on type)
    pub object_addr: u64,

    /// Object size/rights (interpretation depends on cap_type)
    pub size_or_rights: u64,
}

/// Capability types for initial capabilities
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityType {
    /// Null capability (empty slot)
    Null = 0,

    /// Untyped memory capability
    Untyped = 1,

    /// TCB capability
    Tcb = 2,

    /// CNode capability
    CNode = 3,

    /// Endpoint capability
    Endpoint = 4,

    /// VSpace (page table) capability
    VSpace = 5,

    /// Page capability
    Page = 6,

    /// Device frame capability
    DeviceFrame = 7,

    /// IRQ handler capability
    IrqHandler = 8,
}

/// Boot information structure
///
/// This structure is placed at a known location in the root task's address space
/// (typically at a fixed virtual address like 0x8000_0000).
#[repr(C)]
pub struct BootInfo {
    /// Magic number for validation
    pub magic: u32,

    /// Boot info structure version
    pub version: u32,

    /// Number of valid untyped regions
    pub num_untyped_regions: u32,

    /// Number of valid device regions
    pub num_device_regions: u32,

    /// Number of valid initial capability slots
    pub num_initial_caps: u32,

    /// Reserved for future use
    _reserved: [u32; 3],

    /// Root task's CSpace root capability slot
    pub cspace_root_slot: u64,

    /// Root task's VSpace root capability slot
    pub vspace_root_slot: u64,

    /// Root task's IPC buffer virtual address
    pub ipc_buffer_vaddr: u64,

    /// Total RAM size in bytes
    pub ram_size: u64,

    /// Kernel virtual base address
    pub kernel_virt_base: u64,

    /// User virtual address space start
    pub user_virt_start: u64,

    /// IRQControl capability physical address (for delegation to drivers)
    pub irq_control_paddr: u64,

    /// Untyped memory regions
    pub untyped_regions: [UntypedRegion; MAX_UNTYPED_REGIONS],

    /// Device regions
    pub device_regions: [DeviceRegion; MAX_DEVICE_REGIONS],

    /// Initial capability slots
    pub initial_caps: [CapabilitySlot; MAX_INITIAL_CAPS],
}

impl BootInfo {
    /// Create a new boot info structure
    pub const fn new() -> Self {
        Self {
            magic: BOOT_INFO_MAGIC,
            version: BOOT_INFO_VERSION,
            num_untyped_regions: 0,
            num_device_regions: 0,
            num_initial_caps: 0,
            _reserved: [0; 3],
            cspace_root_slot: 0,
            vspace_root_slot: 0,
            ipc_buffer_vaddr: 0,
            ram_size: 0,
            kernel_virt_base: 0,
            user_virt_start: 0,
            irq_control_paddr: 0,
            untyped_regions: [UntypedRegion {
                paddr: 0,
                size_bits: 0,
                is_device: false,
                _reserved: [0; 6],
            }; MAX_UNTYPED_REGIONS],
            device_regions: [DeviceRegion {
                paddr: 0,
                size: 0,
                device_type: 0,
                irq: 0xFFFFFFFF,
            }; MAX_DEVICE_REGIONS],
            initial_caps: [CapabilitySlot {
                slot: 0,
                cap_type: CapabilityType::Null,
                object_addr: 0,
                size_or_rights: 0,
            }; MAX_INITIAL_CAPS],
        }
    }

    /// Validate the boot info structure
    pub fn validate(&self) -> bool {
        self.magic == BOOT_INFO_MAGIC && self.version == BOOT_INFO_VERSION
    }

    /// Add an untyped region to the boot info
    pub fn add_untyped_region(&mut self, region: UntypedRegion) -> Result<(), &'static str> {
        let idx = self.num_untyped_regions as usize;
        if idx >= MAX_UNTYPED_REGIONS {
            return Err("Too many untyped regions");
        }
        self.untyped_regions[idx] = region;
        self.num_untyped_regions += 1;
        Ok(())
    }

    /// Add a device region to the boot info
    pub fn add_device_region(&mut self, region: DeviceRegion) -> Result<(), &'static str> {
        let idx = self.num_device_regions as usize;
        if idx >= MAX_DEVICE_REGIONS {
            return Err("Too many device regions");
        }
        self.device_regions[idx] = region;
        self.num_device_regions += 1;
        Ok(())
    }

    /// Add an initial capability to the boot info
    pub fn add_initial_cap(&mut self, cap: CapabilitySlot) -> Result<(), &'static str> {
        let idx = self.num_initial_caps as usize;
        if idx >= MAX_INITIAL_CAPS {
            return Err("Too many initial capabilities");
        }
        self.initial_caps[idx] = cap;
        self.num_initial_caps += 1;
        Ok(())
    }

    /// Get the size of the boot info structure in bytes
    pub const fn size() -> usize {
        size_of::<Self>()
    }
}

// Compile-time size check to ensure boot info fits in reasonable memory
const _: () = {
    assert!(size_of::<BootInfo>() < 64 * 1024, "BootInfo too large (>64KB)");
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_info_creation() {
        let boot_info = BootInfo::new();
        assert!(boot_info.validate());
        assert_eq!(boot_info.num_untyped_regions, 0);
        assert_eq!(boot_info.num_device_regions, 0);
        assert_eq!(boot_info.num_initial_caps, 0);
    }

    #[test]
    fn test_add_untyped_region() {
        let mut boot_info = BootInfo::new();
        let region = UntypedRegion::new(0x1000, 12, false); // 4KB region
        assert!(boot_info.add_untyped_region(region).is_ok());
        assert_eq!(boot_info.num_untyped_regions, 1);
        assert_eq!(boot_info.untyped_regions[0].paddr, 0x1000);
        assert_eq!(boot_info.untyped_regions[0].size(), 4096);
    }

    #[test]
    fn test_device_region_creation() {
        let region = DeviceRegion {
            paddr: 0x0900_0000,
            size: 0x1000,
            device_type: 1, // UART
            irq: 33,
        };
        assert_eq!(region.paddr, 0x0900_0000);
        assert_eq!(region.device_type, 1);
    }

    #[test]
    fn test_boot_info_size() {
        // Boot info should be reasonably sized (under 64KB)
        assert!(BootInfo::size() < 64 * 1024);
    }
}
