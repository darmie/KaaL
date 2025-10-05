//! Bootinfo Parsing - Extract initial capabilities from seL4 bootinfo
//!
//! The seL4 kernel passes a bootinfo structure to the root task containing:
//! - Untyped memory regions
//! - Device memory regions
//! - Initial capability slots
//! - Platform-specific information
//!
//! This module parses bootinfo and makes it available to the Capability Broker.

use alloc::vec::Vec;
use crate::Result;

// TODO PHASE 2: Import real seL4 bootinfo types
// use sel4::BootInfo;

/// Bootinfo structure (Phase 1 stub)
///
/// TODO PHASE 2: Replace with sel4::BootInfo
pub struct BootInfo {
    /// Range of empty CSlots available for allocation
    pub empty: SlotRegion,

    /// List of untyped memory regions
    pub untyped: Vec<UntypedDescriptor>,

    /// List of device untyped regions
    pub device_untyped: Vec<UntypedDescriptor>,

    /// User image frames
    pub user_image_frames: SlotRegion,

    /// Extra bootinfo (device tree, ACPI tables)
    pub extra_len: usize,
}

/// CSlot region [start, end)
#[derive(Debug, Clone, Copy)]
pub struct SlotRegion {
    pub start: usize,
    pub end: usize,
}

impl SlotRegion {
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

/// Untyped memory descriptor
#[derive(Debug, Clone, Copy)]
pub struct UntypedDescriptor {
    /// Capability slot containing this untyped
    pub cap: usize,

    /// Physical address
    pub paddr: usize,

    /// Size as power of 2 (size = 1 << size_bits)
    pub size_bits: u8,

    /// Is this a device untyped?
    pub is_device: bool,
}

impl UntypedDescriptor {
    pub fn size(&self) -> usize {
        1 << self.size_bits
    }

    pub fn end_paddr(&self) -> usize {
        self.paddr + self.size()
    }
}

impl BootInfo {
    /// Get bootinfo from seL4 kernel
    ///
    /// # Safety
    /// Must be called exactly once from the root task
    ///
    /// TODO PHASE 2: Call sel4::get_bootinfo()
    pub unsafe fn get() -> Result<Self> {
        // Phase 1: Return mock bootinfo
        // Phase 2: Replace with real seL4 call

        Ok(Self {
            empty: SlotRegion {
                start: 100,
                end: 4096,
            },
            untyped: vec![
                UntypedDescriptor {
                    cap: 10,
                    paddr: 0x1000_0000,
                    size_bits: 20, // 1MB
                    is_device: false,
                },
                UntypedDescriptor {
                    cap: 11,
                    paddr: 0x2000_0000,
                    size_bits: 24, // 16MB
                    is_device: false,
                },
            ],
            device_untyped: vec![],
            user_image_frames: SlotRegion { start: 50, end: 60 },
            extra_len: 0,
        })
    }

    /// Get all untyped regions (both regular and device)
    pub fn all_untyped(&self) -> impl Iterator<Item = &UntypedDescriptor> {
        self.untyped.iter().chain(self.device_untyped.iter())
    }

    /// Find untyped region containing physical address
    pub fn find_untyped_for_paddr(&self, paddr: usize) -> Option<&UntypedDescriptor> {
        self.all_untyped()
            .find(|ut| paddr >= ut.paddr && paddr < ut.end_paddr())
    }

    /// Get total amount of untyped memory
    pub fn total_untyped(&self) -> usize {
        self.untyped.iter().map(|ut| ut.size()).sum()
    }

    /// Get total amount of device untyped memory
    pub fn total_device_untyped(&self) -> usize {
        self.device_untyped.iter().map(|ut| ut.size()).sum()
    }
}

/// Parse device tree or ACPI tables from bootinfo extra
///
/// TODO PHASE 2: Implement device tree/ACPI parsing
pub fn parse_device_info(bootinfo: &BootInfo) -> Result<Vec<DeviceInfo>> {
    // Phase 1: Return hardcoded device list
    // Phase 2: Parse actual device tree or ACPI tables

    Ok(vec![
        DeviceInfo {
            name: "serial0",
            compatible: "16550",
            mmio_base: 0x0,
            mmio_size: 0x0,
            irq: 4,
            pci_vendor: None,
            pci_device: None,
        },
        DeviceInfo {
            name: "eth0",
            compatible: "e1000",
            mmio_base: 0xFEBC0000,
            mmio_size: 0x20000,
            irq: 11,
            pci_vendor: Some(0x8086),
            pci_device: Some(0x100E),
        },
    ])
}

/// Device information from device tree or ACPI
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device name
    pub name: &'static str,

    /// Compatible string
    pub compatible: &'static str,

    /// MMIO base address
    pub mmio_base: usize,

    /// MMIO region size
    pub mmio_size: usize,

    /// IRQ number
    pub irq: u8,

    /// PCI vendor ID (if PCI device)
    pub pci_vendor: Option<u16>,

    /// PCI device ID (if PCI device)
    pub pci_device: Option<u16>,
}

impl DeviceInfo {
    /// Check if this is a PCI device
    pub fn is_pci(&self) -> bool {
        self.pci_vendor.is_some() && self.pci_device.is_some()
    }

    /// Check if this matches a device identifier
    pub fn matches(&self, id: &crate::DeviceId) -> bool {
        match id {
            crate::DeviceId::Pci { vendor, device } => {
                self.pci_vendor == Some(*vendor) && self.pci_device == Some(*device)
            }
            crate::DeviceId::Platform { name } => self.name == *name,
            crate::DeviceId::Serial { port } => {
                self.compatible == "16550" && self.irq == 4 + port
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootinfo_parsing() {
        unsafe {
            let bootinfo = BootInfo::get().unwrap();

            // Check empty slot region
            assert!(bootinfo.empty.len() > 0);
            assert!(bootinfo.empty.start < bootinfo.empty.end);

            // Check untyped regions
            assert!(bootinfo.untyped.len() > 0);
            for ut in &bootinfo.untyped {
                assert!(ut.size() > 0);
                assert!(!ut.is_device);
            }

            // Check total memory
            let total = bootinfo.total_untyped();
            assert!(total > 0);
        }
    }

    #[test]
    fn test_untyped_descriptor() {
        let ut = UntypedDescriptor {
            cap: 10,
            paddr: 0x1000,
            size_bits: 12, // 4KB
            is_device: false,
        };

        assert_eq!(ut.size(), 4096);
        assert_eq!(ut.end_paddr(), 0x1000 + 4096);
    }

    #[test]
    fn test_find_untyped_for_paddr() {
        unsafe {
            let bootinfo = BootInfo::get().unwrap();

            // Should find untyped containing this address
            let ut = bootinfo.find_untyped_for_paddr(0x1000_0000);
            assert!(ut.is_some());
            assert_eq!(ut.unwrap().paddr, 0x1000_0000);

            // Should not find untyped for invalid address
            let ut = bootinfo.find_untyped_for_paddr(0xFFFF_FFFF);
            assert!(ut.is_none());
        }
    }

    #[test]
    fn test_device_info_parsing() {
        unsafe {
            let bootinfo = BootInfo::get().unwrap();
            let devices = parse_device_info(&bootinfo).unwrap();

            assert!(devices.len() > 0);

            // Check serial device
            let serial = devices.iter().find(|d| d.compatible == "16550");
            assert!(serial.is_some());
            assert_eq!(serial.unwrap().irq, 4);

            // Check network device
            let eth = devices.iter().find(|d| d.compatible == "e1000");
            assert!(eth.is_some());
            assert!(eth.unwrap().is_pci());
        }
    }

    #[test]
    fn test_device_matching() {
        let device = DeviceInfo {
            name: "eth0",
            compatible: "e1000",
            mmio_base: 0xFEBC0000,
            mmio_size: 0x20000,
            irq: 11,
            pci_vendor: Some(0x8086),
            pci_device: Some(0x100E),
        };

        // Should match PCI device ID
        let id = crate::DeviceId::Pci {
            vendor: 0x8086,
            device: 0x100E,
        };
        assert!(device.matches(&id));

        // Should not match wrong vendor
        let id = crate::DeviceId::Pci {
            vendor: 0x1234,
            device: 0x100E,
        };
        assert!(!device.matches(&id));
    }
}
