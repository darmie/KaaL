//! TCB (Thread Control Block) Manager
//!
//! Manages thread creation and configuration in seL4. TCBs are kernel objects
//! that represent threads of execution.
//!
//! # Architecture
//! - Creates TCBs from untyped memory via seL4_Untyped_Retype
//! - Configures TCB properties (priority, CSpace, VSpace, IPC buffer)
//! - Binds TCBs to scheduling contexts (for MCS kernel)
//! - Provides thread lifecycle management
//!
//! # seL4 TCB Configuration
//! Each TCB requires:
//! - CSpace root (capability space)
//! - VSpace root (virtual address space / page directory)
//! - IPC buffer (for message passing)
//! - Entry point and stack pointer
//! - Priority and affinity

#![allow(unused)]

use crate::{CSlot, Result, CapabilityError};
use alloc::vec::Vec;

/// Thread priority levels (0 = lowest, 255 = highest)
pub type Priority = u8;

/// Default priority for normal threads
pub const DEFAULT_PRIORITY: Priority = 100;

/// Maximum priority (seL4 allows 0-255)
pub const MAX_PRIORITY: Priority = 255;

/// TCB (Thread Control Block) manager
///
/// Manages thread creation and lifecycle for spawning components.
pub struct TcbManager {
    /// List of allocated TCBs
    allocated_tcbs: Vec<TcbInfo>,
}

/// Information about an allocated TCB
#[derive(Debug, Clone)]
struct TcbInfo {
    /// TCB capability slot
    tcb_cap: CSlot,

    /// Thread name (for debugging)
    name: &'static str,

    /// Thread priority
    priority: Priority,

    /// Is thread currently running?
    is_running: bool,
}

/// TCB configuration parameters
#[derive(Debug, Clone)]
pub struct TcbConfig {
    /// Thread name (for debugging)
    pub name: &'static str,

    /// CSpace root capability
    pub cspace_root: CSlot,

    /// VSpace root capability (page directory)
    pub vspace_root: CSlot,

    /// IPC buffer virtual address
    pub ipc_buffer_vaddr: usize,

    /// IPC buffer frame capability
    pub ipc_buffer_frame: CSlot,

    /// Entry point (function to execute)
    pub entry_point: usize,

    /// Stack pointer
    pub stack_pointer: usize,

    /// Thread priority (0-255)
    pub priority: Priority,

    /// Fault handler endpoint (optional)
    pub fault_ep: Option<CSlot>,
}

impl TcbManager {
    /// Create a new TCB manager
    pub fn new() -> Self {
        Self {
            allocated_tcbs: Vec::new(),
        }
    }

    /// Create a new TCB from untyped memory
    ///
    /// # Arguments
    /// * `tcb_cap` - Destination capability slot for new TCB
    /// * `untyped_cap` - Untyped memory to retype
    /// * `cspace_root` - CSpace root for Untyped_Retype
    /// * `name` - Thread name for debugging
    ///
    /// # Returns
    /// TCB capability slot
    ///
    /// # Errors
    /// Returns error if TCB creation fails
    pub fn create_tcb(
        &mut self,
        tcb_cap: CSlot,
        untyped_cap: CSlot,
        cspace_root: CSlot,
        name: &'static str,
    ) -> Result<CSlot> {
        #[cfg(feature = "runtime")]
        {
            use sel4_platform::adapter::*;

            // Create TCB object from untyped memory
            let ret = unsafe {
                seL4_Untyped_Retype(
                    untyped_cap as u64,
                    seL4_TCBObject as u64,
                    0, // size_bits (0 for fixed-size objects like TCB)
                    cspace_root as u64,
                    0, // node_index
                    0, // node_depth
                    tcb_cap as u64,
                    1, // num_objects
                )
            };

            if ret != seL4_NoError {
                return Err(CapabilityError::Sel4Error(alloc::format!(
                    "seL4_Untyped_Retype (TCB) failed: {}",
                    ret
                )));
            }
        }

        #[cfg(not(feature = "runtime"))]
        {
            // Phase 1: Mock - just track the allocation
            let _ = (untyped_cap, cspace_root);
        }

        // Track the TCB
        self.allocated_tcbs.push(TcbInfo {
            tcb_cap,
            name,
            priority: DEFAULT_PRIORITY,
            is_running: false,
        });

        Ok(tcb_cap)
    }

    /// Configure a TCB with all required settings
    ///
    /// # Arguments
    /// * `tcb_cap` - TCB capability to configure
    /// * `config` - Configuration parameters
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// Returns error if configuration fails
    pub fn configure_tcb(&mut self, tcb_cap: CSlot, config: &TcbConfig) -> Result<()> {
        #[cfg(feature = "runtime")]
        {
            use sel4_platform::adapter::*;

            // Set CSpace root
            let ret = unsafe {
                seL4_TCB_SetSpace(
                    tcb_cap as u64,
                    0, // fault_ep (0 = none, or use config.fault_ep)
                    config.cspace_root as u64,
                    0, // cspace_root_data (CSpace guard)
                    config.vspace_root as u64,
                    0, // vspace_root_data
                )
            };

            if ret != seL4_NoError {
                return Err(CapabilityError::Sel4Error(alloc::format!(
                    "seL4_TCB_SetSpace failed: {}",
                    ret
                )));
            }

            // Set IPC buffer
            let ret = unsafe {
                seL4_TCB_SetIPCBuffer(
                    tcb_cap as u64,
                    config.ipc_buffer_vaddr as u64,
                    config.ipc_buffer_frame as u64,
                )
            };

            if ret != seL4_NoError {
                return Err(CapabilityError::Sel4Error(alloc::format!(
                    "seL4_TCB_SetIPCBuffer failed: {}",
                    ret
                )));
            }

            // Set priority
            let ret = unsafe {
                seL4_TCB_SetPriority(
                    tcb_cap as u64,
                    tcb_cap as u64, // authority (use self for now)
                    config.priority as u64,
                )
            };

            if ret != seL4_NoError {
                return Err(CapabilityError::Sel4Error(alloc::format!(
                    "seL4_TCB_SetPriority failed: {}",
                    ret
                )));
            }

            // Set registers (entry point and stack)
            // Architecture-specific register setup
            #[cfg(target_arch = "x86_64")]
            {
                let mut regs: seL4_UserContext = core::mem::zeroed();

                // x86_64: Set instruction pointer (RIP) and stack pointer (RSP)
                regs.rip = config.entry_point as u64;
                regs.rsp = config.stack_pointer as u64;
                regs.rflags = 0x200; // IF (interrupt enable) flag

                let ret = unsafe {
                    seL4_TCB_WriteRegisters(
                        tcb_cap as u64,
                        0, // resume (0 = don't start yet)
                        0, // arch_flags
                        core::mem::size_of::<seL4_UserContext>() as u64,
                        &mut regs as *mut seL4_UserContext,
                    )
                };

                if ret != seL4_NoError {
                    return Err(CapabilityError::Sel4Error(alloc::format!(
                        "seL4_TCB_WriteRegisters failed: {}",
                        ret
                    )));
                }
            }

            #[cfg(target_arch = "aarch64")]
            unsafe {
                let mut regs: seL4_UserContext = core::mem::zeroed();

                // aarch64 (Mac Silicon, ARM64): Set program counter (PC) and stack pointer (SP)
                regs.pc = config.entry_point as u64;
                regs.sp = config.stack_pointer as u64;

                // Set processor state (PSTATE)
                // EL0t (user mode), interrupts enabled
                regs.spsr = 0x0; // Default user mode flags

                let ret = seL4_TCB_WriteRegisters(
                    tcb_cap as u64,
                    0, // resume (0 = don't start yet)
                    0, // arch_flags
                    core::mem::size_of::<seL4_UserContext>() as u64,
                    &mut regs as *mut seL4_UserContext,
                );

                if ret != seL4_NoError {
                    return Err(CapabilityError::Sel4Error(alloc::format!(
                        "seL4_TCB_WriteRegisters failed: {}",
                        ret
                    )));
                }
            }

            #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
            {
                // TODO: Add RISC-V register setup when needed
                compile_error!("TCB register setup not implemented for this architecture. Supported: x86_64, aarch64");
            }
        }

        #[cfg(not(feature = "runtime"))]
        {
            // Phase 1: Mock - just validate parameters
            let _ = config;
        }

        // Update TCB info
        if let Some(tcb_info) = self.allocated_tcbs.iter_mut().find(|t| t.tcb_cap == tcb_cap) {
            tcb_info.priority = config.priority;
        }

        Ok(())
    }

    /// Resume (start) a configured TCB
    ///
    /// # Arguments
    /// * `tcb_cap` - TCB capability to resume
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// Returns error if resume fails
    pub fn resume_tcb(&mut self, tcb_cap: CSlot) -> Result<()> {
        #[cfg(feature = "runtime")]
        {
            use sel4_platform::adapter::*;

            let ret = unsafe { seL4_TCB_Resume(tcb_cap as u64) };

            if ret != seL4_NoError {
                return Err(CapabilityError::Sel4Error(alloc::format!(
                    "seL4_TCB_Resume failed: {}",
                    ret
                )));
            }
        }

        #[cfg(not(feature = "runtime"))]
        {
            let _ = tcb_cap;
        }

        // Mark as running
        if let Some(tcb_info) = self.allocated_tcbs.iter_mut().find(|t| t.tcb_cap == tcb_cap) {
            tcb_info.is_running = true;
        }

        Ok(())
    }

    /// Suspend a running TCB
    ///
    /// # Arguments
    /// * `tcb_cap` - TCB capability to suspend
    ///
    /// # Returns
    /// Ok(()) on success
    pub fn suspend_tcb(&mut self, tcb_cap: CSlot) -> Result<()> {
        #[cfg(feature = "runtime")]
        {
            use sel4_platform::adapter::*;

            let ret = unsafe { seL4_TCB_Suspend(tcb_cap as u64) };

            if ret != seL4_NoError {
                return Err(CapabilityError::Sel4Error(alloc::format!(
                    "seL4_TCB_Suspend failed: {}",
                    ret
                )));
            }
        }

        #[cfg(not(feature = "runtime"))]
        {
            let _ = tcb_cap;
        }

        // Mark as not running
        if let Some(tcb_info) = self.allocated_tcbs.iter_mut().find(|t| t.tcb_cap == tcb_cap) {
            tcb_info.is_running = false;
        }

        Ok(())
    }

    /// Get number of allocated TCBs
    pub fn tcb_count(&self) -> usize {
        self.allocated_tcbs.len()
    }

    /// Get number of running TCBs
    pub fn running_tcb_count(&self) -> usize {
        self.allocated_tcbs.iter().filter(|t| t.is_running).count()
    }

    /// Check if a TCB is running
    pub fn is_running(&self, tcb_cap: CSlot) -> bool {
        self.allocated_tcbs
            .iter()
            .find(|t| t.tcb_cap == tcb_cap)
            .map(|t| t.is_running)
            .unwrap_or(false)
    }
}

impl Default for TcbManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcb_manager_creation() {
        let tcb_mgr = TcbManager::new();
        assert_eq!(tcb_mgr.tcb_count(), 0);
        assert_eq!(tcb_mgr.running_tcb_count(), 0);
    }

    #[test]
    fn test_create_tcb() {
        let mut tcb_mgr = TcbManager::new();

        // Create a TCB
        let tcb_cap = tcb_mgr
            .create_tcb(100, 10, 1, "test_thread")
            .unwrap();

        assert_eq!(tcb_cap, 100);
        assert_eq!(tcb_mgr.tcb_count(), 1);
        assert!(!tcb_mgr.is_running(tcb_cap));
    }

    #[test]
    fn test_configure_tcb() {
        let mut tcb_mgr = TcbManager::new();

        let tcb_cap = tcb_mgr.create_tcb(100, 10, 1, "test_thread").unwrap();

        let config = TcbConfig {
            name: "test_thread",
            cspace_root: 1,
            vspace_root: 2,
            ipc_buffer_vaddr: 0x8000_0000,
            ipc_buffer_frame: 50,
            entry_point: 0x400000,
            stack_pointer: 0x7FFF_F000,
            priority: 150,
            fault_ep: None,
        };

        let result = tcb_mgr.configure_tcb(tcb_cap, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resume_suspend_tcb() {
        let mut tcb_mgr = TcbManager::new();

        let tcb_cap = tcb_mgr.create_tcb(100, 10, 1, "test_thread").unwrap();

        // Initially not running
        assert!(!tcb_mgr.is_running(tcb_cap));
        assert_eq!(tcb_mgr.running_tcb_count(), 0);

        // Resume
        tcb_mgr.resume_tcb(tcb_cap).unwrap();
        assert!(tcb_mgr.is_running(tcb_cap));
        assert_eq!(tcb_mgr.running_tcb_count(), 1);

        // Suspend
        tcb_mgr.suspend_tcb(tcb_cap).unwrap();
        assert!(!tcb_mgr.is_running(tcb_cap));
        assert_eq!(tcb_mgr.running_tcb_count(), 0);
    }

    #[test]
    fn test_multiple_tcbs() {
        let mut tcb_mgr = TcbManager::new();

        // Create multiple TCBs
        let tcb1 = tcb_mgr.create_tcb(100, 10, 1, "thread1").unwrap();
        let tcb2 = tcb_mgr.create_tcb(101, 11, 1, "thread2").unwrap();
        let tcb3 = tcb_mgr.create_tcb(102, 12, 1, "thread3").unwrap();

        assert_eq!(tcb_mgr.tcb_count(), 3);

        // Resume two of them
        tcb_mgr.resume_tcb(tcb1).unwrap();
        tcb_mgr.resume_tcb(tcb3).unwrap();

        assert_eq!(tcb_mgr.running_tcb_count(), 2);
        assert!(tcb_mgr.is_running(tcb1));
        assert!(!tcb_mgr.is_running(tcb2));
        assert!(tcb_mgr.is_running(tcb3));
    }

    #[test]
    fn test_tcb_priority() {
        let mut tcb_mgr = TcbManager::new();
        let tcb_cap = tcb_mgr.create_tcb(100, 10, 1, "test_thread").unwrap();

        let config = TcbConfig {
            name: "test_thread",
            cspace_root: 1,
            vspace_root: 2,
            ipc_buffer_vaddr: 0x8000_0000,
            ipc_buffer_frame: 50,
            entry_point: 0x400000,
            stack_pointer: 0x7FFF_F000,
            priority: MAX_PRIORITY,
            fault_ep: None,
        };

        tcb_mgr.configure_tcb(tcb_cap, &config).unwrap();

        // Verify priority was set
        let tcb_info = tcb_mgr
            .allocated_tcbs
            .iter()
            .find(|t| t.tcb_cap == tcb_cap)
            .unwrap();
        assert_eq!(tcb_info.priority, MAX_PRIORITY);
    }
}
