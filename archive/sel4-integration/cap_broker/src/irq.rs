//! IRQ Handling - Interrupt request management
//!
//! This module handles IRQ allocation and binding to seL4 notification objects.
//! Each IRQ is associated with a notification that can be waited on by drivers.

use alloc::vec::Vec;
use crate::{CSlot, CapabilityError, Result};

// TODO PHASE 2: Import real seL4 types
// use sel4_platform::adapter::{seL4_IRQControl_Get, seL4_IRQHandler_SetNotification, seL4_IRQHandler_Ack};

/// IRQ handler with notification binding
pub struct IrqHandlerImpl {
    /// IRQ handler capability
    handler_cap: CSlot,

    /// Notification object capability
    notification_cap: CSlot,

    /// IRQ number
    irq_num: u8,
}

impl IrqHandlerImpl {
    /// Create a new IRQ handler
    ///
    /// # Phase 1
    /// Just stores the capability slots
    ///
    /// # Phase 2
    /// Will use seL4_IRQControl_Get to obtain handler capability
    pub fn new(irq_num: u8, handler_cap: CSlot, notification_cap: CSlot) -> Self {
        Self {
            handler_cap,
            notification_cap,
            irq_num,
        }
    }

    /// Get IRQ number
    pub fn irq_num(&self) -> u8 {
        self.irq_num
    }

    /// Get notification capability
    pub fn notification_cap(&self) -> CSlot {
        self.notification_cap
    }

    /// Wait for interrupt (blocking)
    ///
    /// # Phase 2
    /// Uses seL4_Wait on notification object
    pub fn wait(&self) -> Result<()> {
        #[cfg(not(feature = "runtime"))]
        {
            // Phase 1: Would block forever, so just return error
            Err(CapabilityError::InvalidCap)
        }

        #[cfg(feature = "runtime")]
        {
            let mut badge: u64 = 0;
            unsafe {
                sel4_platform::adapter::seL4_Wait(self.notification_cap as u64, &mut badge as *mut u64);
            }
            Ok(())
        }
    }

    /// Acknowledge interrupt
    ///
    /// Must be called after handling interrupt to re-enable it
    ///
    /// # Phase 2
    /// Uses seL4_IRQHandler_Ack
    pub fn acknowledge(&self) -> Result<()> {
        #[cfg(not(feature = "runtime"))]
        {
            // Phase 1: No-op
            Ok(())
        }

        #[cfg(feature = "runtime")]
        {
            unsafe {
                let ret = sel4_platform::adapter::seL4_IRQHandler_Ack(self.handler_cap as u64);
                if ret != sel4_platform::adapter::seL4_NoError {
                    return Err(CapabilityError::Sel4Error(format!(
                        "seL4_IRQHandler_Ack failed: {}",
                        ret
                    )));
                }
            }
            Ok(())
        }
    }
}

/// IRQ allocator - manages IRQ handler creation
pub struct IrqAllocator {
    /// IRQs that have been allocated
    allocated_irqs: Vec<u8>,

    /// IRQ control capability (global)
    irq_control: CSlot,
}

impl IrqAllocator {
    /// Create a new IRQ allocator
    ///
    /// # Arguments
    /// * `irq_control` - seL4_CapIRQControl from bootinfo
    pub fn new(irq_control: CSlot) -> Self {
        Self {
            allocated_irqs: Vec::new(),
            irq_control,
        }
    }

    /// Allocate an IRQ handler
    ///
    /// # Arguments
    /// * `irq` - IRQ number to allocate
    /// * `handler_cap` - Capability slot for IRQ handler
    /// * `notification_cap` - Capability slot for notification
    /// * `cspace_root` - Root CSpace capability
    ///
    /// # Returns
    /// IrqHandlerImpl bound to notification
    ///
    /// # Errors
    /// Returns error if IRQ already allocated or seL4 operations fail
    ///
    /// # Phase 2
    /// Uses seL4_IRQControl_Get and seL4_IRQHandler_SetNotification
    pub fn allocate(
        &mut self,
        irq: u8,
        handler_cap: CSlot,
        notification_cap: CSlot,
        cspace_root: CSlot,
    ) -> Result<IrqHandlerImpl> {
        // Check if already allocated
        if self.allocated_irqs.contains(&irq) {
            return Err(CapabilityError::IrqAlreadyAllocated { irq });
        }

        #[cfg(not(feature = "runtime"))]
        {
            // Phase 1: Just track allocation
            self.allocated_irqs.push(irq);
            Ok(IrqHandlerImpl::new(irq, handler_cap, notification_cap))
        }

        #[cfg(feature = "runtime")]
        {
            // Get IRQ handler capability
            unsafe {
                let ret = sel4_platform::adapter::seL4_IRQControl_Get(
                    self.irq_control as u64,
                    irq as u64,
                    cspace_root as u64,
                    handler_cap as u64,
                    32, // depth (seL4_WordBits)
                );

                if ret != sel4_platform::adapter::seL4_NoError {
                    return Err(CapabilityError::Sel4Error(format!(
                        "seL4_IRQControl_Get failed for IRQ {}: {}",
                        irq, ret
                    )));
                }
            }

            // Bind notification to IRQ handler
            unsafe {
                let ret = sel4_platform::adapter::seL4_IRQHandler_SetNotification(handler_cap as u64, notification_cap as u64);

                if ret != sel4_platform::adapter::seL4_NoError {
                    return Err(CapabilityError::Sel4Error(format!(
                        "seL4_IRQHandler_SetNotification failed: {}",
                        ret
                    )));
                }
            }

            self.allocated_irqs.push(irq);
            Ok(IrqHandlerImpl::new(irq, handler_cap, notification_cap))
        }
    }

    /// Check if an IRQ is already allocated
    pub fn is_allocated(&self, irq: u8) -> bool {
        self.allocated_irqs.contains(&irq)
    }

    /// Get list of allocated IRQs
    pub fn allocated_irqs(&self) -> &[u8] {
        &self.allocated_irqs
    }
}

/// Platform-specific IRQ information
pub struct IrqInfo {
    /// IRQ number
    pub irq: u8,

    /// Is this a level-triggered interrupt?
    pub level_triggered: bool,

    /// Is this a shared interrupt?
    pub shared: bool,
}

impl IrqInfo {
    /// Create IRQ info for edge-triggered interrupt
    pub fn edge_triggered(irq: u8) -> Self {
        Self {
            irq,
            level_triggered: false,
            shared: false,
        }
    }

    /// Create IRQ info for level-triggered interrupt
    pub fn level_triggered(irq: u8) -> Self {
        Self {
            irq,
            level_triggered: true,
            shared: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_irq_handler_creation() {
        let handler = IrqHandlerImpl::new(4, 100, 101);
        assert_eq!(handler.irq_num(), 4);
        assert_eq!(handler.notification_cap(), 101);
    }

    #[test]
    fn test_irq_allocator() {
        let mut allocator = IrqAllocator::new(1); // IRQ control cap

        // Allocate IRQ 4
        let handler = allocator.allocate(4, 100, 101, 10).unwrap();
        assert_eq!(handler.irq_num(), 4);
        assert!(allocator.is_allocated(4));

        // Try to allocate same IRQ again
        let result = allocator.allocate(4, 102, 103, 10);
        assert!(matches!(
            result,
            Err(CapabilityError::IrqAlreadyAllocated { irq: 4 })
        ));
    }

    #[test]
    fn test_multiple_irq_allocation() {
        let mut allocator = IrqAllocator::new(1);

        // Allocate multiple IRQs
        allocator.allocate(4, 100, 101, 10).unwrap();
        allocator.allocate(11, 102, 103, 10).unwrap();
        allocator.allocate(15, 104, 105, 10).unwrap();

        // Check all are allocated
        assert!(allocator.is_allocated(4));
        assert!(allocator.is_allocated(11));
        assert!(allocator.is_allocated(15));

        // Check non-allocated
        assert!(!allocator.is_allocated(5));

        // Check list
        let irqs = allocator.allocated_irqs();
        assert_eq!(irqs.len(), 3);
        assert!(irqs.contains(&4));
        assert!(irqs.contains(&11));
        assert!(irqs.contains(&15));
    }

    #[test]
    fn test_irq_info() {
        let edge = IrqInfo::edge_triggered(4);
        assert_eq!(edge.irq, 4);
        assert!(!edge.level_triggered);
        assert!(!edge.shared);

        let level = IrqInfo::level_triggered(11);
        assert_eq!(level.irq, 11);
        assert!(level.level_triggered);
        assert!(!level.shared);
    }
}
