//! Object Invocation System
//!
//! This module implements the syscall dispatch mechanism for object operations.
//! Every operation on a kernel object goes through the invocation system, which:
//! - Validates capability rights
//! - Dispatches to appropriate handler
//! - Provides uniform error handling
//!
//! ## Design
//!
//! The invocation system follows seL4's design where all kernel operations
//! are expressed as invocations on capabilities:
//!
//! ```
//! User Space                Kernel Space
//! ----------                ------------
//! syscall(cap, op, args) -> invoke_capability()
//!                              ├─> validate_rights()
//!                              ├─> match cap_type
//!                              └─> invoke_<object>()
//! ```
//!
//! ## Object Operations
//!
//! Each object type has a set of invocations:
//!
//! ### TCB Invocations
//! - Read/Write registers
//! - Set CSpace/VSpace
//! - Set priority
//! - Suspend/Resume
//!
//! ### CNode Invocations
//! - Copy capability
//! - Move capability
//! - Mint capability
//! - Revoke capability
//!
//! ### Endpoint Invocations
//! - Send message
//! - Receive message
//! - Call (send + block for reply)
//! - Reply
//!
//! ### Untyped Invocations
//! - Retype to object
//! - Revoke children
//!
//! ## Usage
//!
//! ```rust,ignore
//! // From syscall handler
//! let result = invoke_capability(
//!     &cap,
//!     InvocationArgs {
//!         label: syscall_num,
//!         args: &[arg0, arg1, arg2, ...],
//!     }
//! )?;
//! ```

use super::{Capability, CapType, CapRights, CapError, TCB, CNode, Endpoint, UntypedMemory};

/// Invocation arguments passed from syscall
#[derive(Debug, Clone)]
pub struct InvocationArgs<'a> {
    /// Syscall/invocation label
    pub label: u64,

    /// Invocation arguments (up to 6 from x0-x5)
    pub args: &'a [u64],

    /// Optional capability arguments
    pub cap_args: &'a [Option<Capability>],
}

/// Invocation result
pub type InvocationResult = Result<u64, InvocationError>;

/// Invocation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationError {
    /// Invalid capability
    InvalidCapability,

    /// Insufficient rights
    InsufficientRights,

    /// Invalid invocation label
    InvalidInvocation,

    /// Invalid arguments
    InvalidArguments,

    /// Operation failed
    OperationFailed,

    /// Capability error
    CapError(CapError),
}

impl From<CapError> for InvocationError {
    fn from(err: CapError) -> Self {
        InvocationError::CapError(err)
    }
}

/// Main invocation dispatcher
///
/// This is the entry point for all object invocations from syscalls.
/// It validates the capability and dispatches to the appropriate handler.
///
/// # Arguments
///
/// * `cap` - Capability to invoke
/// * `args` - Invocation arguments from syscall
///
/// # Returns
///
/// * `Ok(u64)` - Invocation result (returned in x0)
/// * `Err(InvocationError)` - Invocation failed
///
/// # Safety
///
/// This function assumes:
/// - Capability is valid and points to correct object
/// - Object pointer in capability is valid
/// - Caller has verified syscall came from user space
pub unsafe fn invoke_capability(
    cap: &Capability,
    args: InvocationArgs,
) -> InvocationResult {
    // Dispatch based on capability type
    match cap.cap_type() {
        CapType::Null => Err(InvocationError::InvalidCapability),
        CapType::UntypedMemory => invoke_untyped(cap, args),
        CapType::Tcb => invoke_tcb(cap, args),
        CapType::Endpoint => invoke_endpoint(cap, args),
        CapType::Notification => invoke_notification(cap, args),
        CapType::CNode => invoke_cnode(cap, args),
        CapType::VSpace => invoke_vspace(cap, args),
        CapType::Page => invoke_page(cap, args),
        CapType::PageTable => invoke_page_table(cap, args),
        CapType::IrqHandler => invoke_irq_handler(cap, args),
        CapType::IrqControl => invoke_irq_control(cap, args),
        CapType::Reply => Err(InvocationError::InvalidCapability), // Reply caps are used directly by IPC, not invoked
    }
}

/// TCB invocation labels
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcbInvocation {
    ReadRegisters = 0,
    WriteRegisters = 1,
    CopyRegisters = 2,
    SetPriority = 3,
    SetMCPriority = 4,
    SetSchedParams = 5,
    SetTimeoutEndpoint = 6,
    SetIPCBuffer = 7,
    SetSpace = 8,
    Suspend = 9,
    Resume = 10,
    BindNotification = 11,
    UnbindNotification = 12,
}

/// Invoke TCB operation
unsafe fn invoke_tcb(cap: &Capability, args: InvocationArgs) -> InvocationResult {
    // Check rights
    if !cap.rights().contains(CapRights::WRITE) {
        return Err(InvocationError::InsufficientRights);
    }

    let tcb = cap.object_ptr() as *mut TCB;
    if tcb.is_null() {
        return Err(InvocationError::InvalidCapability);
    }

    // Parse invocation label
    let invocation = match args.label {
        0 => TcbInvocation::ReadRegisters,
        1 => TcbInvocation::WriteRegisters,
        3 => TcbInvocation::SetPriority,
        9 => TcbInvocation::Suspend,
        10 => TcbInvocation::Resume,
        _ => return Err(InvocationError::InvalidInvocation),
    };

    match invocation {
        TcbInvocation::SetPriority => {
            if args.args.is_empty() {
                return Err(InvocationError::InvalidArguments);
            }
            let priority = args.args[0] as u8;
            (*tcb).set_priority(priority);
            Ok(0)
        }
        TcbInvocation::Suspend => {
            // Mark as inactive
            (*tcb).set_state(super::ThreadState::Inactive);
            Ok(0)
        }
        TcbInvocation::Resume => {
            // Activate thread
            (*tcb).activate();
            Ok(0)
        }
        _ => {
            // TODO: Implement other TCB operations
            Err(InvocationError::InvalidInvocation)
        }
    }
}

/// CNode invocation labels
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CNodeInvocation {
    Copy = 0,
    Mint = 1,
    Move = 2,
    Mutate = 3,
    Rotate = 4,
    Delete = 5,
    Revoke = 6,
    SaveCaller = 7,
}

/// Invoke CNode operation
unsafe fn invoke_cnode(cap: &Capability, args: InvocationArgs) -> InvocationResult {
    let cnode = cap.object_ptr() as *mut CNode;
    if cnode.is_null() {
        return Err(InvocationError::InvalidCapability);
    }

    // Parse invocation label
    let invocation = match args.label {
        0 => CNodeInvocation::Copy,
        1 => CNodeInvocation::Mint,
        2 => CNodeInvocation::Move,
        5 => CNodeInvocation::Delete,
        _ => return Err(InvocationError::InvalidInvocation),
    };

    match invocation {
        CNodeInvocation::Delete => {
            if args.args.is_empty() {
                return Err(InvocationError::InvalidArguments);
            }
            let index = args.args[0] as usize;

            // Check WRITE right for deletion
            if !cap.rights().contains(CapRights::WRITE) {
                return Err(InvocationError::InsufficientRights);
            }

            (*cnode).delete(index)?;
            Ok(0)
        }
        CNodeInvocation::Copy | CNodeInvocation::Move => {
            if args.args.len() < 2 {
                return Err(InvocationError::InvalidArguments);
            }
            let src_index = args.args[0] as usize;
            let dest_index = args.args[1] as usize;

            if invocation == CNodeInvocation::Copy {
                (*cnode).copy_cap(src_index, dest_index)?;
            } else {
                (*cnode).move_cap(src_index, dest_index)?;
            }
            Ok(0)
        }
        _ => {
            // TODO: Implement other CNode operations
            Err(InvocationError::InvalidInvocation)
        }
    }
}

/// Endpoint invocation labels
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointInvocation {
    Send = 0,
    Receive = 1,
    Call = 2,
    Reply = 3,
    ReplyRecv = 4,
}

/// Invoke Endpoint operation
unsafe fn invoke_endpoint(cap: &Capability, args: InvocationArgs) -> InvocationResult {
    let endpoint = cap.object_ptr() as *mut Endpoint;
    if endpoint.is_null() {
        return Err(InvocationError::InvalidCapability);
    }

    // Endpoint invocations are handled by IPC layer
    // This is a placeholder that validates the invocation
    let invocation = match args.label {
        0 => EndpointInvocation::Send,
        1 => EndpointInvocation::Receive,
        2 => EndpointInvocation::Call,
        3 => EndpointInvocation::Reply,
        4 => EndpointInvocation::ReplyRecv,
        _ => return Err(InvocationError::InvalidInvocation),
    };

    match invocation {
        EndpointInvocation::Send => {
            if !cap.rights().contains(CapRights::WRITE) {
                return Err(InvocationError::InsufficientRights);
            }
            // Actual send handled by IPC layer
            Ok(0)
        }
        EndpointInvocation::Receive => {
            if !cap.rights().contains(CapRights::READ) {
                return Err(InvocationError::InsufficientRights);
            }
            // Actual receive handled by IPC layer
            Ok(0)
        }
        _ => {
            // TODO: Implement Call/Reply/ReplyRecv
            Err(InvocationError::InvalidInvocation)
        }
    }
}

/// Untyped invocation labels
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UntypedInvocation {
    Retype = 0,
    Revoke = 1,
}

/// Invoke Untyped operation
unsafe fn invoke_untyped(cap: &Capability, args: InvocationArgs) -> InvocationResult {
    // Check rights
    if !cap.rights().contains(CapRights::WRITE) {
        return Err(InvocationError::InsufficientRights);
    }

    let untyped = cap.object_ptr() as *mut UntypedMemory;
    if untyped.is_null() {
        return Err(InvocationError::InvalidCapability);
    }

    let invocation = match args.label {
        0 => UntypedInvocation::Retype,
        1 => UntypedInvocation::Revoke,
        _ => return Err(InvocationError::InvalidInvocation),
    };

    match invocation {
        UntypedInvocation::Retype => {
            if args.args.len() < 2 {
                return Err(InvocationError::InvalidArguments);
            }

            let obj_type_num = args.args[0];
            let size_bits = args.args[1] as u8;

            // Convert number to CapType
            let obj_type = match obj_type_num {
                1 => CapType::UntypedMemory,
                2 => CapType::Endpoint,
                3 => CapType::Notification,
                4 => CapType::Tcb,
                5 => CapType::CNode,
                6 => CapType::VSpace,
                7 => CapType::PageTable,
                8 => CapType::Page,
                _ => return Err(InvocationError::InvalidArguments),
            };

            let paddr = (*untyped).retype(obj_type, size_bits)?;
            Ok(paddr.as_u64())
        }
        UntypedInvocation::Revoke => {
            (*untyped).revoke()?;
            Ok(0)
        }
    }
}

/// Notification invocation (placeholder)
unsafe fn invoke_notification(_cap: &Capability, _args: InvocationArgs) -> InvocationResult {
    // TODO: Implement notification invocations
    Err(InvocationError::InvalidInvocation)
}

/// VSpace invocation (placeholder)
unsafe fn invoke_vspace(_cap: &Capability, _args: InvocationArgs) -> InvocationResult {
    // TODO: Implement VSpace invocations
    Err(InvocationError::InvalidInvocation)
}

/// Page invocation (placeholder)
unsafe fn invoke_page(_cap: &Capability, _args: InvocationArgs) -> InvocationResult {
    // TODO: Implement Page invocations
    Err(InvocationError::InvalidInvocation)
}

/// Page table invocation (placeholder)
unsafe fn invoke_page_table(_cap: &Capability, _args: InvocationArgs) -> InvocationResult {
    // TODO: Implement PageTable invocations
    Err(InvocationError::InvalidInvocation)
}

/// IRQ handler invocation (placeholder)
unsafe fn invoke_irq_handler(_cap: &Capability, _args: InvocationArgs) -> InvocationResult {
    // TODO: Implement IRQ handler invocations
    Err(InvocationError::InvalidInvocation)
}

/// IRQ control invocation (placeholder)
unsafe fn invoke_irq_control(_cap: &Capability, _args: InvocationArgs) -> InvocationResult {
    // TODO: Implement IRQ control invocations
    Err(InvocationError::InvalidInvocation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::VirtAddr;

    #[test]
    fn test_invocation_args_creation() {
        let args = [1u64, 2, 3, 4];
        let inv_args = InvocationArgs {
            label: 42,
            args: &args,
            cap_args: &[],
        };

        assert_eq!(inv_args.label, 42);
        assert_eq!(inv_args.args.len(), 4);
    }

    #[test]
    fn test_tcb_invocation_priority() {
        unsafe {
            let mut cnode_memory = [Capability::null(); 16];
            let cnode_ptr = &mut cnode_memory[0] as *mut _ as *mut CNode;

            let mut tcb = TCB::new(
                1,
                cnode_ptr,
                0x40000000,
                VirtAddr::new(0x10000000),
                0x200000,
                0x300000,
            );

            let cap = Capability::new(CapType::Tcb, &mut tcb as *mut _ as usize);
            let args = InvocationArgs {
                label: 3, // SetPriority
                args: &[100],
                cap_args: &[],
            };

            let result = invoke_capability(&cap, args);
            assert!(result.is_ok());
            assert_eq!(tcb.priority(), 100);
        }
    }

    #[test]
    fn test_insufficient_rights() {
        unsafe {
            let mut cnode_memory = [Capability::null(); 16];
            let cnode_ptr = &mut cnode_memory[0] as *mut _ as *mut CNode;

            let mut tcb = TCB::new(
                1,
                cnode_ptr,
                0x40000000,
                VirtAddr::new(0x10000000),
                0x200000,
                0x300000,
            );

            // Create capability with READ only (no WRITE)
            let mut cap = Capability::new(CapType::Tcb, &mut tcb as *mut _ as usize);
            cap = cap.derive(CapRights::READ).unwrap();

            let args = InvocationArgs {
                label: 3, // SetPriority (requires WRITE)
                args: &[100],
                cap_args: &[],
            };

            let result = invoke_capability(&cap, args);
            assert_eq!(result, Err(InvocationError::InsufficientRights));
        }
    }
}
