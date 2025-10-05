//! Capability Broker - Centralized capability management for KaaL
//!
//! # Purpose
//! The Capability Broker hides seL4's capability complexity by providing
//! high-level abstractions for device access, memory allocation, and IPC
//! endpoint creation.
//!
//! # Integration Points
//! - Depends on: seL4 kernel
//! - Provides to: All system components
//! - IPC endpoints: Request/response pattern
//! - Capabilities required: Root task capabilities (untyped memory, device caps)
//!
//! # Architecture
//! The broker receives all initial capabilities from the root task and manages
//! them through a capability space (CSpace). It allocates capabilities on-demand
//! and tracks ownership for proper cleanup.
//!
//! # Testing Strategy
//! - Unit tests: Capability allocation/deallocation
//! - Integration tests: Device bundle requests
//! - Hardware sim tests: N/A (kernel integration)

#![no_std]

#[cfg(test)]
#[macro_use]
extern crate std;

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
#[macro_use]
extern crate alloc as alloc_crate;

mod bootinfo;
mod mmio;
mod irq;
mod vspace;
mod tcb;

pub use bootinfo::{BootInfo, DeviceInfo, UntypedDescriptor};
pub use mmio::{MmioMapper, PAGE_SIZE, align_up, align_down, pages_needed};
pub use irq::{IrqHandlerImpl, IrqAllocator, IrqInfo};
pub use vspace::VSpaceManager;
pub use tcb::{TcbManager, TcbConfig, Priority, DEFAULT_PRIORITY, MAX_PRIORITY};

use thiserror::Error;

/// Error types for capability operations
#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("Out of capability slots")]
    OutOfSlots,

    #[error("Out of untyped memory (requested: {requested} bytes)")]
    OutOfMemory { requested: usize },

    #[error("Device not found: {device_id:?}")]
    DeviceNotFound { device_id: DeviceId },

    #[error("IRQ {irq} already allocated")]
    IrqAlreadyAllocated { irq: u8 },

    #[error("Invalid capability")]
    InvalidCap,

    #[error("seL4 error: {0}")]
    Sel4Error(String),
}

pub type Result<T> = core::result::Result<T, CapabilityError>;

/// Device identifier for requesting device bundles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceId {
    /// PCI device by vendor and device ID
    Pci { vendor: u16, device: u16 },

    /// Platform device by name (from device tree)
    Platform { name: &'static str },

    /// Serial console (UART)
    Serial { port: u8 },
}

/// Complete device bundle with all required resources
pub struct DeviceBundle {
    /// Memory-mapped I/O regions
    pub mmio_regions: Vec<MappedRegion>,

    /// IRQ handler capability
    pub irq: IrqHandler,

    /// DMA memory pool
    pub dma_pool: DmaPool,

    /// I/O port access (x86 only)
    pub io_ports: Option<Vec<IoPort>>,
}

/// Memory-mapped I/O region
pub struct MappedRegion {
    /// Virtual address where region is mapped
    pub vaddr: usize,

    /// Physical address of the region
    pub paddr: usize,

    /// Size in bytes
    pub size: usize,
}

/// IRQ handler capability
pub struct IrqHandler {
    // Placeholder for seL4 IRQ capability
    _cap: u64,
    irq_num: u8,
}

impl IrqHandler {
    /// Get the IRQ number
    pub fn irq_num(&self) -> u8 {
        self.irq_num
    }

    /// Register a handler function for this IRQ
    ///
    /// # Safety
    /// Handler will be called in interrupt context. Must be fast and non-blocking.
    ///
    /// # Phase 1 Note
    /// This is a stub for Phase 1. Real IRQ handling will be implemented in Phase 2
    /// using the IrqHandlerImpl from the irq module.
    pub unsafe fn register<F>(&self, _handler: F) -> Result<()>
    where
        F: Fn() + Send + 'static,
    {
        // Phase 1: No-op stub
        // Phase 2: Use IrqHandlerImpl::wait() and call handler
        Ok(())
    }

    /// Acknowledge the IRQ
    ///
    /// # Phase 1 Note
    /// This is a stub for Phase 1. Real IRQ ack will be implemented in Phase 2
    /// using the IrqHandlerImpl from the irq module.
    pub fn acknowledge(&self) -> Result<()> {
        // Phase 1: No-op stub
        // Phase 2: Use IrqHandlerImpl::acknowledge()
        Ok(())
    }
}

/// DMA memory pool for device drivers
pub struct DmaPool {
    base_paddr: usize,
    base_vaddr: usize,
    size: usize,
    allocated: usize,
}

impl DmaPool {
    /// Get total pool size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get allocated bytes
    pub fn allocated(&self) -> usize {
        self.allocated
    }

    /// Get available bytes
    pub fn available(&self) -> usize {
        self.size - self.allocated
    }

    /// Allocate DMA memory from the pool
    ///
    /// # Arguments
    /// * `size` - Size in bytes
    /// * `alignment` - Required alignment (must be power of 2)
    ///
    /// # Returns
    /// DMA region with both virtual and physical addresses
    ///
    /// # Errors
    /// Returns error if pool is exhausted or alignment is invalid
    pub fn allocate(&mut self, size: usize, alignment: usize) -> Result<DmaRegion> {
        if !alignment.is_power_of_two() {
            return Err(CapabilityError::InvalidCap);
        }

        // Align allocation
        let aligned_addr = (self.base_vaddr + self.allocated + alignment - 1) & !(alignment - 1);
        let offset = aligned_addr - self.base_vaddr;

        if offset + size > self.size {
            return Err(CapabilityError::OutOfMemory { requested: size });
        }

        let region = DmaRegion {
            vaddr: aligned_addr,
            paddr: self.base_paddr + offset,
            size,
        };

        self.allocated = offset + size;

        Ok(region)
    }
}

/// DMA memory region with identity mapping
pub struct DmaRegion {
    /// Virtual address
    pub vaddr: usize,

    /// Physical address (identity mapped for DMA)
    pub paddr: usize,

    /// Size in bytes
    pub size: usize,
}

/// I/O port (x86 specific)
#[cfg(target_arch = "x86_64")]
pub struct IoPort {
    port: u16,
}

#[cfg(target_arch = "x86_64")]
impl IoPort {
    /// Create a new I/O port
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Read byte from I/O port
    ///
    /// # Safety
    /// Caller must ensure port access is safe
    pub unsafe fn inb(&self) -> u8 {
        let value: u8;
        core::arch::asm!("in al, dx", in("dx") self.port, out("al") value);
        value
    }

    /// Write byte to I/O port
    ///
    /// # Safety
    /// Caller must ensure port access is safe
    pub unsafe fn outb(&self, value: u8) {
        core::arch::asm!("out dx, al", in("dx") self.port, in("al") value);
    }
}

// Provide stub for non-x86 architectures
#[cfg(not(target_arch = "x86_64"))]
pub struct IoPort {
    _phantom: core::marker::PhantomData<()>,
}

#[cfg(not(target_arch = "x86_64"))]
impl IoPort {
    /// Create a new I/O port (no-op on non-x86)
    pub fn new(_port: u16) -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }
}

/// Main Capability Broker interface
pub trait CapabilityBroker {
    /// Request a complete device bundle
    ///
    /// # Arguments
    /// * `device` - Device identifier
    ///
    /// # Returns
    /// Complete bundle with MMIO, IRQ, and DMA resources
    ///
    /// # Errors
    /// Returns error if device not found or resources unavailable
    fn request_device(&mut self, device: DeviceId) -> Result<DeviceBundle>;

    /// Allocate memory region
    ///
    /// # Arguments
    /// * `size` - Size in bytes
    ///
    /// # Returns
    /// Memory region capability
    ///
    /// # Errors
    /// Returns error if out of memory
    fn allocate_memory(&mut self, size: usize) -> Result<MemoryRegion>;

    /// Request IRQ handler
    ///
    /// # Arguments
    /// * `irq` - IRQ number
    ///
    /// # Returns
    /// IRQ handler capability
    ///
    /// # Errors
    /// Returns error if IRQ already allocated
    fn request_irq(&mut self, irq: u8) -> Result<IrqHandler>;

    /// Create IPC endpoint pair
    ///
    /// # Returns
    /// Tuple of (client endpoint, server endpoint)
    ///
    /// # Errors
    /// Returns error if out of capability slots
    fn create_channel(&mut self) -> Result<(Endpoint, Endpoint)>;
}

/// Memory region capability
pub struct MemoryRegion {
    pub vaddr: usize,
    pub size: usize,
}

/// IPC endpoint
pub struct Endpoint {
    _cap: u64,
}

/// Capability slot in CSpace
type CSlot = usize;

/// CSpace allocator - manages capability slots
struct CSpaceAllocator {
    /// Next available slot
    next_slot: CSlot,
    /// Maximum slots available
    max_slots: CSlot,
    /// Free list of deallocated slots
    free_slots: Vec<CSlot>,
}

impl CSpaceAllocator {
    fn new(initial_slot: CSlot, max_slots: CSlot) -> Self {
        Self {
            next_slot: initial_slot,
            max_slots,
            free_slots: Vec::new(),
        }
    }

    fn allocate(&mut self) -> Result<CSlot> {
        // Try to reuse a freed slot first
        if let Some(slot) = self.free_slots.pop() {
            return Ok(slot);
        }

        // Allocate a new slot
        if self.next_slot >= self.max_slots {
            return Err(CapabilityError::OutOfSlots);
        }

        let slot = self.next_slot;
        self.next_slot += 1;
        Ok(slot)
    }

    fn free(&mut self, slot: CSlot) {
        self.free_slots.push(slot);
    }
}

/// Untyped memory region for capability derivation
struct UntypedRegion {
    cap: CSlot,
    base_paddr: usize,
    size_bits: usize,
    allocated: usize,
}

impl UntypedRegion {
    fn available(&self) -> usize {
        (1 << self.size_bits) - self.allocated
    }

    fn allocate(&mut self, size: usize) -> Result<usize> {
        if self.available() < size {
            return Err(CapabilityError::OutOfMemory { requested: size });
        }

        let offset = self.allocated;
        self.allocated += size;
        Ok(self.base_paddr + offset)
    }
}

/// Device registry entry
struct DeviceEntry {
    id: DeviceId,
    mmio_base: usize,
    mmio_size: usize,
    irq: u8,
    io_port_base: Option<u16>,
}

/// Default implementation of Capability Broker
pub struct DefaultCapBroker {
    /// CSpace allocator
    cspace: CSpaceAllocator,

    /// Untyped memory regions
    untyped_regions: Vec<UntypedRegion>,

    /// Device registry (hardcoded for Phase 1)
    devices: Vec<DeviceEntry>,

    /// MMIO mapper for device memory mapping
    mmio_mapper: mmio::MmioMapper,

    /// IRQ allocator for interrupt handling
    irq_allocator: irq::IrqAllocator,

    /// CSpace root capability (from bootinfo)
    cspace_root: CSlot,

    /// VSpace root capability (from bootinfo)
    vspace_root: CSlot,
}

impl DefaultCapBroker {
    /// Initialize the capability broker
    ///
    /// This should be called once during system initialization by the root task.
    ///
    /// # Safety
    /// Must be called exactly once, in the root task context
    pub unsafe fn init() -> Result<Self> {
        // PHASE 2: Get bootinfo from seL4 kernel
        let bootinfo = BootInfo::get()?;

        // Initialize CSpace allocator using bootinfo's empty slot region
        let cspace = CSpaceAllocator::new(
            bootinfo.empty.start,
            bootinfo.empty.end
        );

        // Parse untyped regions from bootinfo
        let untyped_regions: Vec<UntypedRegion> = bootinfo.all_untyped()
            .map(|ut| UntypedRegion {
                cap: ut.cap,
                base_paddr: ut.paddr,
                size_bits: ut.size_bits as usize,
                allocated: 0,
            })
            .collect();

        // TODO PHASE 2: Parse device tree or ACPI tables from bootinfo.extra
        // For Phase 1, hardcode common devices
        let devices = vec![
            // Serial port (COM1)
            DeviceEntry {
                id: DeviceId::Serial { port: 0 },
                mmio_base: 0x0,
                mmio_size: 0x0,
                irq: 4,
                io_port_base: Some(0x3F8),
            },
            // Intel E1000 NIC (common in QEMU)
            DeviceEntry {
                id: DeviceId::Pci {
                    vendor: 0x8086,
                    device: 0x100E,
                },
                mmio_base: 0xFEBC0000,
                mmio_size: 0x20000,
                irq: 11,
                io_port_base: None,
            },
        ];

        // Use IRQ control capability from bootinfo
        let irq_control_cap = bootinfo.irq_control;

        Ok(Self {
            cspace,
            untyped_regions,
            devices,
            mmio_mapper: mmio::MmioMapper::new(0x8000_0000, 256 * 1024 * 1024), // 256MB MMIO region
            irq_allocator: irq::IrqAllocator::new(irq_control_cap),
            cspace_root: bootinfo.cspace_root,
            vspace_root: bootinfo.vspace_root,
        })
    }

    /// Find device by ID
    fn find_device(&self, device_id: DeviceId) -> Result<&DeviceEntry> {
        self.devices
            .iter()
            .find(|d| d.id == device_id)
            .ok_or(CapabilityError::DeviceNotFound { device_id })
    }

    /// Allocate untyped memory
    fn allocate_untyped(&mut self, size: usize) -> Result<usize> {
        // Find first untyped region with enough space
        for region in &mut self.untyped_regions {
            if region.available() >= size {
                return region.allocate(size);
            }
        }

        Err(CapabilityError::OutOfMemory { requested: size })
    }

    /// Find untyped capability for a specific physical address region
    fn find_untyped_for_region(&self, paddr: usize, size: usize) -> Result<CSlot> {
        // Find untyped region that contains this physical address range
        for region in &self.untyped_regions {
            let region_end = region.base_paddr + (1 << region.size_bits);
            if region.base_paddr <= paddr && paddr + size <= region_end {
                return Ok(region.cap);
            }
        }

        // If not found, use first available untyped (for Phase 1)
        self.untyped_regions
            .first()
            .map(|r| r.cap)
            .ok_or(CapabilityError::OutOfMemory { requested: size })
    }
}

impl CapabilityBroker for DefaultCapBroker {
    fn request_device(&mut self, device_id: DeviceId) -> Result<DeviceBundle> {
        // Extract device information first to avoid borrow conflicts
        let device = self.find_device(device_id)?;
        let mmio_base = device.mmio_base;
        let mmio_size = device.mmio_size;
        let device_irq = device.irq;
        let io_port_base = device.io_port_base;

        // Allocate MMIO regions
        let mut mmio_regions = Vec::new();
        if mmio_size > 0 {
            // Get untyped cap for device memory region
            let untyped_cap = self.find_untyped_for_region(mmio_base, mmio_size)?;

            // Use MMIO mapper to map the device memory
            let region = self.mmio_mapper.map_region(
                mmio_base,
                mmio_size,
                &mut || self.cspace.allocate(),
                untyped_cap,
                self.vspace_root,
                self.cspace_root,
            )?;

            mmio_regions.push(region);
        }

        // Allocate IRQ handler
        let irq = self.request_irq(device_irq)?;

        // Allocate DMA pool (1MB per device)
        let dma_size = 1024 * 1024;
        let dma_paddr = self.allocate_untyped(dma_size)?;

        // TODO PHASE 2: Map DMA memory into VSpace
        let dma_vaddr = dma_paddr; // Identity mapping for now

        let dma_pool = DmaPool {
            base_paddr: dma_paddr,
            base_vaddr: dma_vaddr,
            size: dma_size,
            allocated: 0,
        };

        // Handle I/O ports (x86 only)
        let io_ports = io_port_base.map(|base| {
            vec![
                IoPort::new(base),
                IoPort::new(base + 1),
                IoPort::new(base + 2),
                IoPort::new(base + 3),
                IoPort::new(base + 4),
                IoPort::new(base + 5),
                IoPort::new(base + 6),
                IoPort::new(base + 7),
            ]
        });

        Ok(DeviceBundle {
            mmio_regions,
            irq,
            dma_pool,
            io_ports,
        })
    }

    fn allocate_memory(&mut self, size: usize) -> Result<MemoryRegion> {
        // Allocate from untyped pool
        let paddr = self.allocate_untyped(size)?;

        // TODO PHASE 2: Use seL4_Untyped_Retype to create frame capabilities
        // TODO PHASE 2: Use seL4_ARCH_Page_Map to map frames into VSpace

        // For Phase 1, use identity mapping
        Ok(MemoryRegion {
            vaddr: paddr,
            size,
        })
    }

    fn request_irq(&mut self, irq: u8) -> Result<IrqHandler> {
        // Allocate capability slots for IRQ handler and notification
        let handler_cap = self.cspace.allocate()?;
        let notification_cap = self.cspace.allocate()?;

        // Use IRQ allocator to create the IRQ handler
        let irq_impl = self.irq_allocator.allocate(irq, handler_cap, notification_cap, self.cspace_root)?;

        Ok(IrqHandler {
            _cap: irq_impl.irq_num() as u64,
            irq_num: irq_impl.irq_num(),
        })
    }

    fn create_channel(&mut self) -> Result<(Endpoint, Endpoint)> {
        // Allocate two capability slots
        let client_slot = self.cspace.allocate()?;
        let server_slot = self.cspace.allocate()?;

        // TODO PHASE 2: Use seL4_Untyped_Retype to create endpoint capabilities
        // For Phase 1, just track the slots

        Ok((
            Endpoint {
                _cap: client_slot as u64,
            },
            Endpoint {
                _cap: server_slot as u64,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dma_pool_allocation() {
        let mut pool = DmaPool {
            base_paddr: 0x1000,
            base_vaddr: 0x2000,
            size: 4096,
            allocated: 0,
        };

        // Test basic allocation
        let region1 = pool.allocate(64, 64).unwrap();
        assert_eq!(region1.size, 64);
        assert_eq!(region1.vaddr % 64, 0); // Check alignment

        // Test second allocation
        let region2 = pool.allocate(128, 128).unwrap();
        assert_eq!(region2.size, 128);
        assert_eq!(region2.vaddr % 128, 0);

        // Regions shouldn't overlap
        assert!(region1.vaddr + region1.size <= region2.vaddr);
    }

    #[test]
    fn test_dma_pool_out_of_memory() {
        let mut pool = DmaPool {
            base_paddr: 0x1000,
            base_vaddr: 0x2000,
            size: 100,
            allocated: 0,
        };

        // Try to allocate more than available
        let result = pool.allocate(200, 1);
        assert!(matches!(result, Err(CapabilityError::OutOfMemory { .. })));
    }

    #[test]
    fn test_dma_pool_alignment() {
        let mut pool = DmaPool {
            base_paddr: 0x1000,
            base_vaddr: 0x2000,
            size: 4096,
            allocated: 0,
        };

        // Invalid alignment (not power of 2)
        let result = pool.allocate(64, 3);
        assert!(matches!(result, Err(CapabilityError::InvalidCap)));

        // Valid power-of-2 alignments
        let region1 = pool.allocate(64, 4).unwrap();
        assert_eq!(region1.vaddr % 4, 0);

        let region2 = pool.allocate(64, 16).unwrap();
        assert_eq!(region2.vaddr % 16, 0);
    }

    #[test]
    fn test_device_id_equality() {
        let dev1 = DeviceId::Pci {
            vendor: 0x8086,
            device: 0x100E,
        };
        let dev2 = DeviceId::Pci {
            vendor: 0x8086,
            device: 0x100E,
        };
        let dev3 = DeviceId::Pci {
            vendor: 0x8086,
            device: 0x100F,
        };

        assert_eq!(dev1, dev2);
        assert_ne!(dev1, dev3);
    }

    #[test]
    fn test_cspace_allocator() {
        let mut cspace = CSpaceAllocator::new(100, 200);

        // Allocate some slots
        let slot1 = cspace.allocate().unwrap();
        assert_eq!(slot1, 100);

        let slot2 = cspace.allocate().unwrap();
        assert_eq!(slot2, 101);

        // Free a slot and reallocate
        cspace.free(slot1);
        let slot3 = cspace.allocate().unwrap();
        assert_eq!(slot3, 100); // Should reuse freed slot
    }

    #[test]
    fn test_cspace_out_of_slots() {
        let mut cspace = CSpaceAllocator::new(100, 102);

        // Allocate all available slots
        cspace.allocate().unwrap();
        cspace.allocate().unwrap();

        // Next allocation should fail
        let result = cspace.allocate();
        assert!(matches!(result, Err(CapabilityError::OutOfSlots)));
    }

    #[test]
    fn test_untyped_region_allocation() {
        let mut region = UntypedRegion {
            cap: 1,
            base_paddr: 0x1000_0000,
            size_bits: 12, // 4KB
            allocated: 0,
        };

        // Allocate 1KB
        let paddr1 = region.allocate(1024).unwrap();
        assert_eq!(paddr1, 0x1000_0000);
        assert_eq!(region.allocated, 1024);

        // Allocate another 2KB
        let paddr2 = region.allocate(2048).unwrap();
        assert_eq!(paddr2, 0x1000_0000 + 1024);
        assert_eq!(region.allocated, 3072);

        // Try to allocate more than available (only 1KB left)
        let result = region.allocate(2048);
        assert!(matches!(result, Err(CapabilityError::OutOfMemory { .. })));
    }

    #[test]
    fn test_broker_initialization() {
        unsafe {
            let broker = DefaultCapBroker::init().unwrap();
            assert_eq!(broker.devices.len(), 2); // Serial + E1000
            // IRQs are now tracked by irq_allocator internally
        }
    }

    #[test]
    fn test_broker_request_device() {
        unsafe {
            let mut broker = DefaultCapBroker::init().unwrap();

            // Request serial device
            let serial_id = DeviceId::Serial { port: 0 };
            let bundle = broker.request_device(serial_id).unwrap();

            // Serial device should have IRQ 4
            assert_eq!(bundle.irq.irq_num, 4);

            // Should have DMA pool
            assert_eq!(bundle.dma_pool.size, 1024 * 1024);

            // x86: Should have I/O ports
            #[cfg(target_arch = "x86_64")]
            assert!(bundle.io_ports.is_some());
        }
    }

    #[test]
    fn test_broker_irq_allocation() {
        unsafe {
            let mut broker = DefaultCapBroker::init().unwrap();

            // Allocate IRQ 5
            let irq1 = broker.request_irq(5).unwrap();
            assert_eq!(irq1.irq_num, 5);

            // Try to allocate same IRQ again - should fail
            let result = broker.request_irq(5);
            assert!(matches!(
                result,
                Err(CapabilityError::IrqAlreadyAllocated { irq: 5 })
            ));

            // Allocate different IRQ - should succeed
            let irq2 = broker.request_irq(6).unwrap();
            assert_eq!(irq2.irq_num, 6);
        }
    }

    #[test]
    fn test_broker_memory_allocation() {
        unsafe {
            let mut broker = DefaultCapBroker::init().unwrap();

            // Allocate 4KB
            let mem1 = broker.allocate_memory(4096).unwrap();
            assert_eq!(mem1.size, 4096);

            // Allocate 16KB
            let mem2 = broker.allocate_memory(16384).unwrap();
            assert_eq!(mem2.size, 16384);

            // Regions should not overlap
            assert!(mem1.vaddr + mem1.size <= mem2.vaddr || mem2.vaddr + mem2.size <= mem1.vaddr);
        }
    }

    #[test]
    fn test_broker_channel_creation() {
        unsafe {
            let mut broker = DefaultCapBroker::init().unwrap();

            // Create IPC channel
            let (client, server) = broker.create_channel().unwrap();

            // Endpoints should have different capabilities
            assert_ne!(client._cap, server._cap);
        }
    }

    #[test]
    fn test_broker_device_not_found() {
        unsafe {
            let mut broker = DefaultCapBroker::init().unwrap();

            // Try to request non-existent device
            let unknown_device = DeviceId::Pci {
                vendor: 0xFFFF,
                device: 0xFFFF,
            };

            let result = broker.request_device(unknown_device);
            assert!(matches!(
                result,
                Err(CapabilityError::DeviceNotFound { .. })
            ));
        }
    }
}
