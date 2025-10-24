//! IRQ Handler and IRQ Control Objects
//!
//! This module implements the IRQ capability system for delegating hardware interrupts
//! to userspace device drivers.
//!
//! ## Architecture
//!
//! KaaL follows seL4's IRQ model:
//!
//! ### IRQControl
//! - Global singleton capability (only root-task has it)
//! - Allows allocation of IRQHandler capabilities
//! - One IRQControl per system
//!
//! ### IRQHandler
//! - Per-IRQ capability
//! - Binds a specific IRQ to a notification object
//! - When IRQ fires, kernel signals the notification
//! - Userspace driver waits on notification to receive IRQ
//!
//! ## IRQ Handling Flow
//!
//! 1. Root-task has IRQControl capability
//! 2. Driver requests IRQ: `IRQControl_Get(irq_control, irq_num, notif, irq_handler_slot)`
//! 3. Kernel creates IRQHandler, binds IRQ → notification
//! 4. Driver calls `sys_wait(notification)` to wait for IRQs
//! 5. When IRQ fires:
//!    - Kernel ACKs interrupt at GIC
//!    - Kernel signals notification
//!    - Userspace driver wakes up
//!    - Driver services the device
//!    - Driver calls `IRQHandler_Ack(irq_handler)` to re-enable IRQ
//!
//! ## Differences from seL4
//!
//! seL4 requires explicit IRQ acknowledgment from userspace before the IRQ
//! can fire again. KaaL follows the same model for safety:
//! - Prevents IRQ storms
//! - Forces driver to handle interrupt before enabling next one
//! - Provides backpressure if driver is slow

use crate::arch::aarch64::gic;
use crate::objects::Notification;

/// IRQ Handler - capability for receiving hardware interrupts
///
/// An IRQHandler binds a specific hardware IRQ to a notification object.
/// When the IRQ fires, the kernel signals the notification, waking up
/// the waiting userspace driver.
///
/// ## Lifecycle
///
/// 1. Created by IRQControl_Get syscall
/// 2. Binds IRQ number to notification
/// 3. IRQ is disabled until first Ack
/// 4. On IRQ: kernel signals notification
/// 5. Driver services device
/// 6. Driver calls IRQHandler_Ack to re-enable
///
/// ## Safety
///
/// - Only one IRQHandler per IRQ number (enforced by IRQ_HANDLERS table)
/// - IRQ is masked until driver acknowledges
/// - Prevents IRQ storms and race conditions
#[repr(C)]
pub struct IRQHandler {
    /// IRQ number this handler manages (e.g., 27 for timer, 33 for UART0)
    irq_num: u32,

    /// Notification to signal when IRQ fires
    ///
    /// When the IRQ fires, the kernel:
    /// 1. Acknowledges the IRQ at the GIC
    /// 2. Signals this notification
    /// 3. Masks the IRQ (prevents further interrupts)
    ///
    /// The IRQ remains masked until the driver calls IRQHandler_Ack.
    notification: *mut Notification,

    /// Whether this IRQ is currently enabled
    ///
    /// IRQs start disabled and must be explicitly enabled by the first Ack.
    /// After each interrupt, the IRQ is masked until the next Ack.
    enabled: bool,
}

impl IRQHandler {
    /// Create a new IRQ handler
    ///
    /// # Arguments
    /// * `irq_num` - Hardware IRQ number (from GIC)
    /// * `notification` - Notification to signal on IRQ
    ///
    /// # Safety
    /// - `notification` must be a valid pointer to a Notification object
    /// - Caller must ensure only one IRQHandler per IRQ number exists
    pub unsafe fn new(irq_num: u32, notification: *mut Notification) -> Self {
        Self {
            irq_num,
            notification,
            enabled: false,
        }
    }

    /// Get the IRQ number
    pub fn irq_num(&self) -> u32 {
        self.irq_num
    }

    /// Get the notification pointer
    pub fn notification(&self) -> *mut Notification {
        self.notification
    }

    /// Check if IRQ is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Acknowledge IRQ and re-enable it
    ///
    /// This is called by the userspace driver after it has serviced the interrupt.
    /// It performs EOI (End Of Interrupt) and unmasks the IRQ at the GIC.
    ///
    /// # Safety
    /// Must be called from the owning driver thread
    pub unsafe fn ack(&mut self) {
        // Signal End Of Interrupt to the GIC
        // This is deferred from the IRQ handler to ensure the device has cleared
        // its interrupt line before we tell the GIC the interrupt is complete.
        // For level-sensitive interrupts, EOI before device clear causes spurious re-trigger.
        gic::end_of_interrupt(self.irq_num);

        // Enable/unmask the IRQ at the GIC for the next interrupt
        gic::enable_irq(self.irq_num);
        self.enabled = true;
    }

    /// Signal the notification (called by kernel IRQ handler)
    ///
    /// # Safety
    /// Must be called from IRQ context with valid notification pointer
    pub unsafe fn signal_irq(&self) {
        if !self.notification.is_null() {
            (*self.notification).signal(1 << self.irq_num);
        }
    }
}

/// IRQ Control - global capability for allocating IRQ handlers
///
/// Only the root-task has the IRQControl capability. It can use it to
/// create IRQHandler capabilities for specific IRQs and delegate them
/// to device drivers.
///
/// ## Design
///
/// IRQControl is a singleton - there's only one per system. It doesn't
/// store any state; it's just a capability type that gates access to
/// the IRQControl_Get operation.
#[repr(C)]
pub struct IRQControl {
    // Empty - IRQControl is just a capability type with no state
    _marker: core::marker::PhantomData<()>,
}

impl IRQControl {
    /// Create the IRQControl capability
    ///
    /// This should only be called once during system initialization
    /// to create the root-task's IRQControl capability.
    pub const fn new() -> Self {
        Self {
            _marker: core::marker::PhantomData,
        }
    }
}

/// Global IRQ handler table
///
/// Maps IRQ numbers to IRQHandler objects. Only one handler per IRQ is allowed.
///
/// # Safety
/// - Protected by kernel context (single-threaded kernel)
/// - Only accessed from syscall context
/// - Prevents multiple drivers from claiming the same IRQ
static mut IRQ_HANDLERS: [Option<*mut IRQHandler>; gic::MAX_IRQS] = [None; gic::MAX_IRQS];

/// Register an IRQ handler
///
/// # Arguments
/// * `irq_num` - IRQ number to register
/// * `handler` - Pointer to IRQHandler object
///
/// # Returns
/// - `Ok(())` if registration succeeded
/// - `Err(())` if IRQ is already registered
///
/// # Safety
/// - Must be called from syscall context
/// - `handler` must be a valid IRQHandler pointer
/// - IRQ number must be valid (<MAX_IRQS)
pub unsafe fn register_irq_handler(irq_num: u32, handler: *mut IRQHandler) -> Result<(), ()> {
    if irq_num >= gic::MAX_IRQS as u32 {
        return Err(());
    }

    let slot = &mut IRQ_HANDLERS[irq_num as usize];
    if slot.is_some() {
        // IRQ already claimed
        return Err(());
    }

    *slot = Some(handler);
    Ok(())
}

/// Unregister an IRQ handler
///
/// # Arguments
/// * `irq_num` - IRQ number to unregister
///
/// # Safety
/// - Must be called from syscall context
/// - Caller must own the IRQHandler for this IRQ
pub unsafe fn unregister_irq_handler(irq_num: u32) {
    if irq_num < gic::MAX_IRQS as u32 {
        IRQ_HANDLERS[irq_num as usize] = None;
        gic::disable_irq(irq_num);
    }
}

/// Handle an IRQ from the GIC
///
/// This is called by the kernel IRQ exception handler when an interrupt fires.
/// It looks up the registered handler and signals its notification.
///
/// # Arguments
/// * `irq_num` - IRQ number from GIC IAR
///
/// # Safety
/// - Must be called from IRQ exception context
/// - GIC interrupt must already be acknowledged (IAR read)
pub unsafe fn handle_irq(irq_num: u32) {
    if irq_num >= gic::MAX_IRQS as u32 {
        crate::kprintln!("[IRQ] Invalid IRQ number: {}", irq_num);
        return;
    }

    if let Some(handler_ptr) = IRQ_HANDLERS[irq_num as usize] {
        if !handler_ptr.is_null() {
            let handler = &*handler_ptr;
            handler.signal_irq();
        }
    }
    // If no handler registered, just ignore (already ACKed at GIC)
}

/// Get IRQ handler for an IRQ number
///
/// Used by syscalls to access IRQ handlers.
///
/// # Safety
/// Must be called from syscall context
pub unsafe fn get_irq_handler(irq_num: u32) -> Option<*mut IRQHandler> {
    if irq_num < gic::MAX_IRQS as u32 {
        IRQ_HANDLERS[irq_num as usize]
    } else {
        None
    }
}
