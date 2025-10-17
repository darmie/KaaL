//! Component argument passing
//!
//! When a component is spawned, the kernel sets up initial register values
//! to pass arguments. This module provides safe access to these arguments.
//!
//! # Register Convention (ARM64)
//! - x0: First argument (shared memory virtual address)
//! - x1: Second argument (receiver notification cap slot)
//! - x2: Third argument (sender notification cap slot)
//!
//! For IPC components, these arguments form a ChannelConfig structure.

/// Component startup arguments
///
/// These are passed via registers when the component is spawned.
/// The kernel sets x0, x1, x2 in the initial TCB context.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ComponentArgs {
    /// First argument (typically shared memory virtual address)
    pub arg0: usize,
    /// Second argument (typically receiver notification cap slot)
    pub arg1: usize,
    /// Third argument (typically sender notification cap slot)
    pub arg2: usize,
}

impl ComponentArgs {
    /// Read arguments from initial registers
    ///
    /// This reads the arguments that were passed when the component was spawned.
    /// It uses inline assembly to access the register values that were set up
    /// by the kernel in the TCB's initial context.
    ///
    /// # Safety
    /// This should only be called once at component startup, before any
    /// other code modifies the argument registers.
    #[inline(always)]
    pub unsafe fn read() -> Self {
        let arg0: usize;
        let arg1: usize;
        let arg2: usize;

        core::arch::asm!(
            // Arguments are already in x0, x1, x2 from kernel
            // Just move them to our output variables
            "mov {arg0}, x0",
            "mov {arg1}, x1",
            "mov {arg2}, x2",
            arg0 = out(reg) arg0,
            arg1 = out(reg) arg1,
            arg2 = out(reg) arg2,
            options(pure, nomem, nostack)
        );

        Self { arg0, arg1, arg2 }
    }

    /// Check if arguments are initialized (non-zero)
    pub fn is_initialized(&self) -> bool {
        self.arg0 != 0 || self.arg1 != 0 || self.arg2 != 0
    }
}

/// Channel configuration passed to IPC components
///
/// This is the semantic interpretation of ComponentArgs for components
/// that communicate via shared memory channels.
#[derive(Debug, Clone, Copy)]
pub struct ChannelConfig {
    /// Virtual address of shared memory for the ring buffer
    pub shared_memory: usize,
    /// Capability slot for receiver's notification object
    pub receiver_notify: usize,
    /// Capability slot for sender's notification object
    pub sender_notify: usize,
}

impl ChannelConfig {
    /// Create ChannelConfig from raw component arguments
    pub fn from_args(args: ComponentArgs) -> Self {
        Self {
            shared_memory: args.arg0,
            receiver_notify: args.arg1,
            sender_notify: args.arg2,
        }
    }

    /// Read channel configuration from startup arguments
    ///
    /// # Safety
    /// Should only be called once at component startup
    pub unsafe fn read() -> Self {
        Self::from_args(ComponentArgs::read())
    }
}
