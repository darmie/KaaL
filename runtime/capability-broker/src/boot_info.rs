//! Boot Information Types
//!
//! These types match the kernel's BootInfo structure and allow userspace
//! to read system configuration passed by the kernel.
//!
//! The boot info is mapped at a fixed virtual address (0x7FFF_F000) by the kernel.

/// Magic number to identify valid boot info (ASCII: "KAAL")
pub const BOOT_INFO_MAGIC: u32 = 0x4B41414C;

/// Boot info structure version
pub const BOOT_INFO_VERSION: u32 = 1;

/// Fixed virtual address where kernel maps boot info
pub const BOOT_INFO_VADDR: usize = 0x7FFF_F000;

/// Untyped memory region descriptor
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UntypedRegion {
    /// Physical address of the region
    pub paddr: u64,
    /// Size in bits (e.g., 26 = 64MB)
    pub size_bits: u8,
    /// Whether this region is device memory
    pub is_device: bool,
    /// Reserved for alignment
    _reserved: [u8; 6],
}

impl UntypedRegion {
    /// Get the size in bytes
    pub fn size(&self) -> usize {
        1 << self.size_bits
    }
}

/// Device region descriptor
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DeviceRegion {
    /// Physical address of the device MMIO region
    pub paddr: u64,
    /// Size in bytes
    pub size: u64,
    /// Device type identifier
    pub device_type: u32,
    /// IRQ number (0xFFFFFFFF if no IRQ)
    pub irq: u32,
}

/// Capability type identifiers
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityType {
    /// Null capability
    Null = 0,
    /// Untyped memory
    Untyped = 1,
    /// TCB
    Tcb = 2,
    /// CNode
    CNode = 3,
    /// Endpoint
    Endpoint = 4,
    /// VSpace
    VSpace = 5,
    /// Page
    Page = 6,
    /// Device frame
    DeviceFrame = 7,
    /// IRQ handler
    IrqHandler = 8,
}

/// Initial capability slot descriptor
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CapabilitySlot {
    /// CSpace slot index
    pub slot: u64,
    /// Capability type
    pub cap_type: CapabilityType,
    /// Object address
    pub object_addr: u64,
    /// Size or rights
    pub size_or_rights: u64,
}

/// Boot information structure
///
/// This structure is created by the kernel and mapped at BOOT_INFO_VADDR
/// in the root task's address space.
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
    /// Reserved
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
    /// Untyped memory regions (max 128)
    pub untyped_regions: [UntypedRegion; 128],
    /// Device regions (max 32)
    pub device_regions: [DeviceRegion; 32],
    /// Initial capability slots (max 256)
    pub initial_caps: [CapabilitySlot; 256],
}

impl BootInfo {
    /// Read boot info from the fixed virtual address
    ///
    /// # Safety
    ///
    /// Assumes the kernel has properly mapped the boot info at BOOT_INFO_VADDR.
    /// This should only be called after kernel has completed initialization.
    pub unsafe fn read() -> Option<&'static Self> {
        let boot_info_ptr = BOOT_INFO_VADDR as *const BootInfo;
        let boot_info = &*boot_info_ptr;

        // Validate magic and version
        if boot_info.magic != BOOT_INFO_MAGIC {
            return None;
        }

        if boot_info.version != BOOT_INFO_VERSION {
            return None;
        }

        Some(boot_info)
    }

    /// Iterate over untyped memory regions
    pub fn untyped_regions(&self) -> impl Iterator<Item = &UntypedRegion> {
        self.untyped_regions[..self.num_untyped_regions as usize].iter()
    }

    /// Iterate over device regions
    pub fn device_regions(&self) -> impl Iterator<Item = &DeviceRegion> {
        self.device_regions[..self.num_device_regions as usize].iter()
    }

    /// Iterate over initial capability slots
    pub fn initial_caps(&self) -> impl Iterator<Item = &CapabilitySlot> {
        self.initial_caps[..self.num_initial_caps as usize].iter()
    }

    /// Find a device region by device type
    pub fn find_device(&self, device_type: u32) -> Option<&DeviceRegion> {
        self.device_regions().find(|d| d.device_type == device_type)
    }
}
