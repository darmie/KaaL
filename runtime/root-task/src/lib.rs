//! # KaaL Root Task
//!
//! The root task is the first userspace program that runs on seL4. It is responsible for:
//! - Parsing bootinfo from the kernel
//! - Initializing the capability broker
//! - Setting up initial system services
//! - Spawning component tasks
//!
//! ## Architecture
//!
//! ```text
//! seL4 Kernel Boot
//!      ↓
//! Root Task (this crate)
//!      ↓
//! ┌────┴────────────────────────┐
//! │  1. Parse Bootinfo          │
//! │  2. Initialize Cap Broker   │
//! │  3. Setup VSpace/CSpace     │
//! │  4. Spawn Components        │
//! └─────────────────────────────┘
//!      ↓
//! System Services + Drivers
//! ```

#![no_std]
#![feature(never_type)]

#[cfg(test)]
#[macro_use]
extern crate std;

use cap_broker::{BootInfo, DefaultCapBroker};

// Use seL4 platform adapter (supports mock/microkit/runtime modes)
use sel4_platform::adapter as sel4_sys;

/// Root task configuration
#[derive(Debug, Clone)]
pub struct RootTaskConfig {
    /// Initial heap size in bytes
    pub heap_size: usize,
    /// Number of capability slots to reserve
    pub cspace_size: usize,
    /// Virtual address space size
    pub vspace_size: usize,
}

impl Default for RootTaskConfig {
    fn default() -> Self {
        Self {
            heap_size: 4 * 1024 * 1024,      // 4MB heap
            cspace_size: 4096,               // 4K capability slots
            vspace_size: 1024 * 1024 * 1024, // 1GB virtual address space
        }
    }
}

/// Root task initialization error
#[derive(Debug)]
pub enum RootTaskError {
    /// Failed to get bootinfo from kernel
    BootinfoFailed,
    /// Failed to initialize capability broker
    BrokerInitFailed,
    /// Invalid bootinfo structure
    InvalidBootinfo,
    /// Out of memory during initialization
    OutOfMemory,
}

/// Root task context
///
/// This structure holds all the state needed by the root task to manage
/// the system initialization and component spawning.
pub struct RootTask {
    /// Capability broker instance
    broker: DefaultCapBroker,
    /// Bootinfo from seL4 kernel
    bootinfo: BootInfo,
    /// Configuration
    config: RootTaskConfig,
}

impl RootTask {
    /// Initialize the root task
    ///
    /// This is the entry point called by the seL4 kernel after boot.
    /// It performs system initialization and sets up the capability broker.
    ///
    /// # Safety
    ///
    /// Must be called exactly once, as the very first function in the root task.
    /// The caller must ensure that:
    /// - We are running in the root task context
    /// - The kernel has provided valid bootinfo
    /// - No other code has modified the initial state
    pub unsafe fn init(config: RootTaskConfig) -> Result<Self, RootTaskError> {
        // Step 1: Get bootinfo from seL4 kernel
        let bootinfo = BootInfo::get().map_err(|_| RootTaskError::BootinfoFailed)?;

        // Step 2: Validate bootinfo
        if bootinfo.empty.start >= bootinfo.empty.end {
            return Err(RootTaskError::InvalidBootinfo);
        }

        // Step 3: Initialize capability broker
        let broker = DefaultCapBroker::init().map_err(|_| RootTaskError::BrokerInitFailed)?;

        Ok(Self {
            broker,
            bootinfo,
            config,
        })
    }

    /// Get a reference to the capability broker
    pub fn broker(&self) -> &DefaultCapBroker {
        &self.broker
    }

    /// Get a mutable reference to the capability broker
    pub fn broker_mut(&mut self) -> &mut DefaultCapBroker {
        &mut self.broker
    }

    /// Get bootinfo
    pub fn bootinfo(&self) -> &BootInfo {
        &self.bootinfo
    }

    /// Get configuration
    pub fn config(&self) -> &RootTaskConfig {
        &self.config
    }

    /// Run the root task with custom component initialization
    ///
    /// This is the **composable** way to build your KaaL system.
    /// Pass a closure that spawns your components, and the system does the rest!
    ///
    /// # Getting Started
    ///
    /// 1. Define what components you want
    /// 2. Pass a closure to spawn them
    /// 3. KaaL handles the rest!
    ///
    /// # Example: Minimal System
    /// ```no_run
    /// let root = unsafe { RootTask::init(RootTaskConfig::default())? };
    /// root.run_with(|broker| {
    ///     // Spawn your components here
    ///     spawn_hello_component(broker);
    ///     spawn_my_driver(broker);
    /// });
    /// ```
    ///
    /// # Arguments
    /// * `init_fn` - Closure that receives the capability broker and spawns components
    pub fn run_with<F>(mut self, init_fn: F) -> !
    where
        F: FnOnce(&mut DefaultCapBroker),
    {
        // Call user's initialization function
        init_fn(&mut self.broker);

        // Enter idle loop (wait for component events)
        loop {
            #[cfg(feature = "sel4-real")]
            unsafe {
                // Wait for any notification from components
                sel4_sys::seL4_Yield();
            }

            #[cfg(not(feature = "sel4-real"))]
            {
                // Mock mode: idle loop
                core::hint::spin_loop();
            }
        }
    }

    /// Run the root task with default (empty) initialization
    ///
    /// For systems that don't need any components initially.
    pub fn run(self) -> ! {
        self.run_with(|_| {
            // No components - just idle
        })
    }
}

/// VSpace (Virtual Address Space) Manager
///
/// Manages virtual memory mappings for the root task and spawned components.
pub struct VSpaceManager {
    /// Root VSpace capability
    vspace_root: sel4_sys::seL4_CPtr,
    /// Next available virtual address
    next_vaddr: usize,
    /// Virtual address space base
    vaddr_base: usize,
    /// Virtual address space size
    vaddr_size: usize,
}

impl VSpaceManager {
    /// Create a new VSpace manager
    ///
    /// # Arguments
    /// * `vspace_root` - The root page directory capability
    /// * `base` - Starting virtual address
    /// * `size` - Size of virtual address space
    pub fn new(vspace_root: sel4_sys::seL4_CPtr, base: usize, size: usize) -> Self {
        Self {
            vspace_root,
            next_vaddr: base,
            vaddr_base: base,
            vaddr_size: size,
        }
    }

    /// Allocate a virtual address range
    ///
    /// Returns the starting virtual address of the allocated range.
    pub fn allocate(&mut self, size: usize) -> Result<usize, RootTaskError> {
        // Align to page boundary
        let aligned_size = (size + 0xFFF) & !0xFFF;

        if self.next_vaddr + aligned_size > self.vaddr_base + self.vaddr_size {
            return Err(RootTaskError::OutOfMemory);
        }

        let vaddr = self.next_vaddr;
        self.next_vaddr += aligned_size;
        Ok(vaddr)
    }

    /// Get the VSpace root capability
    pub fn root_cap(&self) -> sel4_sys::seL4_CPtr {
        self.vspace_root
    }
}

/// CNode (Capability Space) Manager
///
/// Manages capability slot allocation for the root task.
pub struct CNodeManager {
    /// Root CNode capability
    cnode_root: sel4_sys::seL4_CPtr,
    /// Next available capability slot
    next_slot: usize,
    /// Total number of slots
    total_slots: usize,
}

impl CNodeManager {
    /// Create a new CNode manager
    pub fn new(cnode_root: sel4_sys::seL4_CPtr, start_slot: usize, total_slots: usize) -> Self {
        Self {
            cnode_root,
            next_slot: start_slot,
            total_slots,
        }
    }

    /// Allocate a capability slot
    pub fn allocate(&mut self) -> Result<sel4_sys::seL4_CPtr, RootTaskError> {
        if self.next_slot >= self.total_slots {
            return Err(RootTaskError::OutOfMemory);
        }

        let slot = self.next_slot;
        self.next_slot += 1;
        Ok(slot as sel4_sys::seL4_CPtr)
    }

    /// Get the CNode root capability
    pub fn root_cap(&self) -> sel4_sys::seL4_CPtr {
        self.cnode_root
    }
}

/// Component spawning information
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    /// Component name
    pub name: &'static str,
    /// Entry point function
    pub entry_point: usize,
    /// Stack size in bytes
    pub stack_size: usize,
    /// Priority (0-255)
    pub priority: u8,
}

/// Component spawner
///
/// Handles creating and starting new seL4 threads for components.
pub struct ComponentSpawner {
    /// VSpace manager for allocating virtual memory
    vspace: VSpaceManager,
    /// CNode manager for allocating capabilities
    cnode: CNodeManager,
}

impl ComponentSpawner {
    /// Create a new component spawner
    pub fn new(vspace: VSpaceManager, cnode: CNodeManager) -> Self {
        Self { vspace, cnode }
    }

    /// Spawn a new component
    ///
    /// Creates a new thread with its own stack and starts executing
    /// at the specified entry point.
    ///
    /// # Safety
    ///
    /// The entry point must be a valid function pointer and the component
    /// must not violate memory safety.
    pub unsafe fn spawn(
        &mut self,
        info: ComponentInfo,
    ) -> Result<sel4_sys::seL4_CPtr, RootTaskError> {
        // TODO PHASE 2: Allocate TCB capability
        let tcb_cap = self.cnode.allocate()?;

        // TODO PHASE 2: Allocate stack memory
        let stack_vaddr = self.vspace.allocate(info.stack_size)?;

        // TODO PHASE 2: Configure TCB
        // - Set instruction pointer to entry_point
        // - Set stack pointer to stack_vaddr + stack_size
        // - Set priority
        // - Set CSpace and VSpace

        // TODO PHASE 2: Start the thread
        #[cfg(feature = "sel4-real")]
        {
            // Real seL4 TCB configuration would go here
            // sel4_sys::seL4_TCB_Configure(...)
            // sel4_sys::seL4_TCB_WriteRegisters(...)
            // sel4_sys::seL4_TCB_Resume(...)
        }

        Ok(tcb_cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_task_config_default() {
        let config = RootTaskConfig::default();
        assert_eq!(config.heap_size, 4 * 1024 * 1024);
        assert_eq!(config.cspace_size, 4096);
        assert_eq!(config.vspace_size, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_vspace_manager_allocation() {
        let mut vspace = VSpaceManager::new(1, 0x1000_0000, 256 * 1024 * 1024);

        // Allocate 4KB
        let vaddr1 = vspace.allocate(4096).unwrap();
        assert_eq!(vaddr1, 0x1000_0000);

        // Allocate another 8KB
        let vaddr2 = vspace.allocate(8192).unwrap();
        assert_eq!(vaddr2, 0x1000_1000); // 4KB after first allocation
    }

    #[test]
    fn test_vspace_manager_alignment() {
        let mut vspace = VSpaceManager::new(1, 0x1000_0000, 256 * 1024 * 1024);

        // Allocate 100 bytes (should round up to 4KB)
        let vaddr1 = vspace.allocate(100).unwrap();
        let vaddr2 = vspace.allocate(100).unwrap();

        assert_eq!(vaddr2 - vaddr1, 4096); // Should be page-aligned
    }

    #[test]
    fn test_cnode_manager_allocation() {
        let mut cnode = CNodeManager::new(1, 100, 1000);

        let slot1 = cnode.allocate().unwrap();
        assert_eq!(slot1, 100);

        let slot2 = cnode.allocate().unwrap();
        assert_eq!(slot2, 101);
    }

    #[test]
    fn test_cnode_manager_out_of_slots() {
        let mut cnode = CNodeManager::new(1, 10, 12); // Only 2 slots available

        cnode.allocate().unwrap(); // Slot 10
        cnode.allocate().unwrap(); // Slot 11

        let result = cnode.allocate(); // Should fail
        assert!(matches!(result, Err(RootTaskError::OutOfMemory)));
    }
}
