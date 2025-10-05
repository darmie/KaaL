//! Component Spawning Infrastructure
//!
//! Provides high-level component spawning that combines:
//! - TCB creation and configuration
//! - VSpace allocation and setup
//! - Memory allocation (stack, IPC buffer)
//! - Endpoint/notification creation for IPC
//! - Device resource allocation (MMIO, IRQ, DMA)
//!
//! # Component Model
//! A component is an isolated unit of execution with:
//! - Dedicated thread (TCB)
//! - Private address space (VSpace)
//! - IPC endpoints for communication
//! - Optional device resources
//!
//! # Example
//! ```ignore
//! let config = ComponentConfig {
//!     name: "serial_driver",
//!     entry_point: driver_main as usize,
//!     stack_size: 64 * 1024,
//!     priority: 150,
//!     device: Some(DeviceId::Serial { port: 0 }),
//! };
//!
//! let component = spawner.spawn_component(config)?;
//! component.start()?;
//! ```

#![allow(unused)]

use crate::{
    CSlot, Result, CapabilityError, TcbManager, TcbConfig, VSpaceManager,
    DeviceId, DeviceBundle, Priority, DEFAULT_PRIORITY,
};
use alloc::vec::Vec;

/// Default stack size for components (64KB)
pub const DEFAULT_STACK_SIZE: usize = 64 * 1024;

/// Default IPC buffer size (4KB - one page)
pub const IPC_BUFFER_SIZE: usize = 4096;

/// Component spawner - orchestrates component creation
pub struct ComponentSpawner {
    /// TCB manager for thread creation
    tcb_manager: TcbManager,

    /// VSpace manager for address space allocation
    vspace_manager: VSpaceManager,

    /// CSpace root capability
    cspace_root: CSlot,

    /// VSpace root capability
    vspace_root: CSlot,

    /// List of spawned components
    components: Vec<ComponentInfo>,
}

/// Information about a spawned component
#[derive(Debug, Clone)]
struct ComponentInfo {
    /// Component name
    name: &'static str,

    /// TCB capability
    tcb_cap: CSlot,

    /// VSpace root capability
    vspace_root: CSlot,

    /// IPC endpoint (for sending messages to this component)
    endpoint: CSlot,

    /// Notification capability (for async signals)
    notification: CSlot,

    /// Is component currently running?
    is_running: bool,
}

/// Component configuration
#[derive(Debug, Clone)]
pub struct ComponentConfig {
    /// Component name (for debugging)
    pub name: &'static str,

    /// Entry point function
    pub entry_point: usize,

    /// Stack size in bytes (must be page-aligned)
    pub stack_size: usize,

    /// Thread priority (0-255)
    pub priority: Priority,

    /// Optional device to allocate
    pub device: Option<DeviceId>,

    /// Fault handler endpoint (optional)
    pub fault_ep: Option<CSlot>,
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self {
            name: "component",
            entry_point: 0,
            stack_size: DEFAULT_STACK_SIZE,
            priority: DEFAULT_PRIORITY,
            device: None,
            fault_ep: None,
        }
    }
}

/// Spawned component handle
pub struct Component {
    /// Component name
    name: &'static str,

    /// TCB capability
    tcb_cap: CSlot,

    /// IPC endpoint
    endpoint: CSlot,

    /// Notification capability
    notification: CSlot,

    /// Device resources (if any)
    pub(crate) device_bundle: Option<DeviceBundle>,
}

impl Component {
    /// Get component name
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Get TCB capability
    pub fn tcb_cap(&self) -> CSlot {
        self.tcb_cap
    }

    /// Get IPC endpoint
    pub fn endpoint(&self) -> CSlot {
        self.endpoint
    }

    /// Get notification capability
    pub fn notification(&self) -> CSlot {
        self.notification
    }

    /// Get device bundle (if component has device access)
    pub fn device_bundle(&self) -> Option<&DeviceBundle> {
        self.device_bundle.as_ref()
    }
}

impl ComponentSpawner {
    /// Create a new component spawner
    ///
    /// # Arguments
    /// * `cspace_root` - CSpace root capability from bootinfo
    /// * `vspace_root` - VSpace root capability from bootinfo
    /// * `vaddr_base` - Base address for component address spaces
    /// * `vaddr_size` - Size of address space to manage
    pub fn new(
        cspace_root: CSlot,
        vspace_root: CSlot,
        vaddr_base: usize,
        vaddr_size: usize,
    ) -> Self {
        Self {
            tcb_manager: TcbManager::new(),
            vspace_manager: VSpaceManager::new(vspace_root, vaddr_base, vaddr_size),
            cspace_root,
            vspace_root,
            components: Vec::new(),
        }
    }

    /// Spawn a new component
    ///
    /// This creates a complete isolated component with:
    /// - TCB (thread)
    /// - VSpace (address space)
    /// - Stack memory
    /// - IPC buffer
    /// - Endpoints for communication
    ///
    /// # Arguments
    /// * `config` - Component configuration
    /// * `cspace_allocator` - Function to allocate capability slots
    /// * `untyped_cap` - Untyped memory for allocations
    ///
    /// # Returns
    /// Component handle
    ///
    /// # Errors
    /// Returns error if spawning fails
    pub fn spawn_component<F>(
        &mut self,
        config: ComponentConfig,
        mut cspace_allocator: F,
        untyped_cap: CSlot,
    ) -> Result<Component>
    where
        F: FnMut() -> Result<CSlot>,
    {
        // 1. Allocate capability slots
        let tcb_cap = cspace_allocator()?;
        let vspace_root = cspace_allocator()?;
        let endpoint = cspace_allocator()?;
        let notification = cspace_allocator()?;
        let stack_frame = cspace_allocator()?;
        let ipc_buffer_frame = cspace_allocator()?;

        // 2. Create TCB
        self.tcb_manager.create_tcb(
            tcb_cap,
            untyped_cap,
            self.cspace_root,
            config.name,
        )?;

        // 3. Allocate virtual address space for stack
        let stack_vaddr = self.vspace_manager.allocate_vaddr(config.stack_size)?;
        let stack_top = stack_vaddr + config.stack_size;

        // 4. Allocate virtual address for IPC buffer
        let ipc_buffer_vaddr = self.vspace_manager.allocate_vaddr(IPC_BUFFER_SIZE)?;

        // 5. Map stack memory
        // TODO PHASE 2: Create stack frame from untyped and map it
        // For Phase 1, we just track the addresses
        #[cfg(feature = "sel4-real")]
        {
            // Create frame for stack
            // Map frame into VSpace at stack_vaddr
            // (Requires seL4_Untyped_Retype + seL4_ARCH_Page_Map)
        }

        // 6. Map IPC buffer
        #[cfg(feature = "sel4-real")]
        {
            // Create frame for IPC buffer
            // Map frame into VSpace at ipc_buffer_vaddr
        }

        // 7. Create endpoints and notification
        #[cfg(feature = "sel4-real")]
        {
            use sel4_sys::*;

            // Create endpoint
            let ret = unsafe {
                seL4_Untyped_Retype(
                    untyped_cap,
                    seL4_EndpointObject,
                    0,
                    self.cspace_root,
                    0, 0,
                    endpoint,
                    1,
                )
            };

            if ret != seL4_NoError {
                return Err(CapabilityError::Sel4Error(alloc::format!(
                    "Failed to create endpoint: {}",
                    ret
                )));
            }

            // Create notification
            let ret = unsafe {
                seL4_Untyped_Retype(
                    untyped_cap,
                    seL4_NotificationObject,
                    0,
                    self.cspace_root,
                    0, 0,
                    notification,
                    1,
                )
            };

            if ret != seL4_NoError {
                return Err(CapabilityError::Sel4Error(alloc::format!(
                    "Failed to create notification: {}",
                    ret
                )));
            }
        }

        // 8. Configure TCB
        let tcb_config = TcbConfig {
            name: config.name,
            cspace_root: self.cspace_root,
            vspace_root,
            ipc_buffer_vaddr,
            ipc_buffer_frame,
            entry_point: config.entry_point,
            stack_pointer: stack_top, // Stack grows down
            priority: config.priority,
            fault_ep: config.fault_ep,
        };

        self.tcb_manager.configure_tcb(tcb_cap, &tcb_config)?;

        // 9. Track component
        self.components.push(ComponentInfo {
            name: config.name,
            tcb_cap,
            vspace_root,
            endpoint,
            notification,
            is_running: false,
        });

        // 9. Allocate device if requested
        // Note: Device allocation would be done by the capability broker
        // We just track whether this component will have device access
        let device_bundle = None; // TODO: Accept broker reference to allocate device

        // 10. Return component handle
        Ok(Component {
            name: config.name,
            tcb_cap,
            endpoint,
            notification,
            device_bundle,
        })
    }

    /// Spawn a component with device access
    ///
    /// This is a higher-level method that integrates with the capability broker
    /// to allocate both the component AND its device resources.
    ///
    /// # Arguments
    /// * `config` - Component configuration (must have device specified)
    /// * `cspace_allocator` - Function to allocate capability slots
    /// * `untyped_cap` - Untyped memory for allocations
    /// * `broker` - Capability broker for device allocation
    ///
    /// # Returns
    /// Component handle with device bundle
    ///
    /// # Errors
    /// Returns error if spawning or device allocation fails
    pub fn spawn_component_with_device<F, B>(
        &mut self,
        config: ComponentConfig,
        mut cspace_allocator: F,
        untyped_cap: CSlot,
        broker: &mut B,
    ) -> Result<Component>
    where
        F: FnMut() -> Result<CSlot>,
        B: crate::CapabilityBroker,
    {
        // First spawn the basic component
        let mut component = self.spawn_component(config.clone(), &mut cspace_allocator, untyped_cap)?;

        // Then allocate device if specified
        if let Some(device_id) = config.device {
            let device_bundle = broker.request_device(device_id)?;
            component.device_bundle = Some(device_bundle);
        }

        Ok(component)
    }

    /// Start a spawned component
    ///
    /// Resumes the component's TCB, starting execution at the entry point.
    ///
    /// # Arguments
    /// * `component` - Component to start
    ///
    /// # Returns
    /// Ok(()) on success
    pub fn start_component(&mut self, component: &Component) -> Result<()> {
        self.tcb_manager.resume_tcb(component.tcb_cap)?;

        // Mark as running
        if let Some(info) = self.components.iter_mut().find(|c| c.tcb_cap == component.tcb_cap) {
            info.is_running = true;
        }

        Ok(())
    }

    /// Stop a running component
    ///
    /// Suspends the component's TCB.
    ///
    /// # Arguments
    /// * `component` - Component to stop
    ///
    /// # Returns
    /// Ok(()) on success
    pub fn stop_component(&mut self, component: &Component) -> Result<()> {
        self.tcb_manager.suspend_tcb(component.tcb_cap)?;

        // Mark as not running
        if let Some(info) = self.components.iter_mut().find(|c| c.tcb_cap == component.tcb_cap) {
            info.is_running = false;
        }

        Ok(())
    }

    /// Get number of spawned components
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Get number of running components
    pub fn running_component_count(&self) -> usize {
        self.components.iter().filter(|c| c.is_running).count()
    }

    /// Get available VSpace
    pub fn available_vspace(&self) -> usize {
        self.vspace_manager.available_vaddr_space()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_allocator(next: &mut CSlot) -> impl FnMut() -> Result<CSlot> + '_ {
        move || {
            let slot = *next;
            *next += 1;
            Ok(slot)
        }
    }

    #[test]
    fn test_component_spawner_creation() {
        let spawner = ComponentSpawner::new(
            1,  // cspace_root
            2,  // vspace_root
            0x4000_0000,
            256 * 1024 * 1024,
        );

        assert_eq!(spawner.component_count(), 0);
        assert_eq!(spawner.running_component_count(), 0);
    }

    #[test]
    fn test_spawn_simple_component() {
        let mut spawner = ComponentSpawner::new(1, 2, 0x4000_0000, 256 * 1024 * 1024);

        let config = ComponentConfig {
            name: "test_component",
            entry_point: 0x400000,
            stack_size: 64 * 1024,
            priority: 100,
            device: None,
            fault_ep: None,
        };

        let mut next_slot = 100;
        let component = spawner
            .spawn_component(config, mock_allocator(&mut next_slot), 10)
            .unwrap();

        assert_eq!(component.name(), "test_component");
        assert_eq!(spawner.component_count(), 1);
        assert_eq!(spawner.running_component_count(), 0);
    }

    #[test]
    fn test_start_stop_component() {
        let mut spawner = ComponentSpawner::new(1, 2, 0x4000_0000, 256 * 1024 * 1024);

        let config = ComponentConfig {
            name: "test_component",
            entry_point: 0x400000,
            stack_size: 64 * 1024,
            priority: 100,
            device: None,
            fault_ep: None,
        };

        let mut next_slot = 100;
        let component = spawner
            .spawn_component(config, mock_allocator(&mut next_slot), 10)
            .unwrap();

        // Start component
        spawner.start_component(&component).unwrap();
        assert_eq!(spawner.running_component_count(), 1);

        // Stop component
        spawner.stop_component(&component).unwrap();
        assert_eq!(spawner.running_component_count(), 0);
    }

    #[test]
    fn test_spawn_multiple_components() {
        let mut spawner = ComponentSpawner::new(1, 2, 0x4000_0000, 256 * 1024 * 1024);

        let mut next_slot = 100;

        // Spawn first component
        let config1 = ComponentConfig {
            name: "component1",
            entry_point: 0x400000,
            stack_size: 64 * 1024,
            priority: 100,
            device: None,
            fault_ep: None,
        };
        let comp1 = spawner
            .spawn_component(config1, mock_allocator(&mut next_slot), 10)
            .unwrap();

        // Spawn second component
        let config2 = ComponentConfig {
            name: "component2",
            entry_point: 0x500000,
            stack_size: 64 * 1024,
            priority: 150,
            device: None,
            fault_ep: None,
        };
        let comp2 = spawner
            .spawn_component(config2, mock_allocator(&mut next_slot), 11)
            .unwrap();

        assert_eq!(spawner.component_count(), 2);

        // Start both
        spawner.start_component(&comp1).unwrap();
        spawner.start_component(&comp2).unwrap();
        assert_eq!(spawner.running_component_count(), 2);

        // Stop one
        spawner.stop_component(&comp1).unwrap();
        assert_eq!(spawner.running_component_count(), 1);
    }

    #[test]
    fn test_component_config_default() {
        let config = ComponentConfig::default();
        assert_eq!(config.name, "component");
        assert_eq!(config.stack_size, DEFAULT_STACK_SIZE);
        assert_eq!(config.priority, DEFAULT_PRIORITY);
        assert!(config.device.is_none());
    }

    #[test]
    fn test_vspace_allocation_tracking() {
        let spawner = ComponentSpawner::new(1, 2, 0x4000_0000, 256 * 1024 * 1024);

        let initial_available = spawner.available_vspace();
        assert_eq!(initial_available, 256 * 1024 * 1024);

        // After spawning, vspace should decrease
        // (Each component uses stack + IPC buffer space)
    }

    #[test]
    fn test_component_capabilities() {
        let mut spawner = ComponentSpawner::new(1, 2, 0x4000_0000, 256 * 1024 * 1024);

        let config = ComponentConfig {
            name: "test",
            entry_point: 0x400000,
            ..Default::default()
        };

        let mut next_slot = 100;
        let component = spawner
            .spawn_component(config, mock_allocator(&mut next_slot), 10)
            .unwrap();

        // Verify component has required capabilities
        assert!(component.tcb_cap() >= 100);
        assert!(component.endpoint() >= 100);
        assert!(component.notification() >= 100);
    }
}
