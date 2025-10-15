//! Endpoint Manager
//!
//! Manages IPC endpoint creation and tracking.

use crate::Result;

/// IPC Endpoint
///
/// Represents an IPC endpoint for communication between components.
#[derive(Debug, Clone, Copy)]
pub struct Endpoint {
    /// Capability slot for this endpoint
    pub cap_slot: usize,
    /// Endpoint ID (for debugging)
    pub id: usize,
}

impl Endpoint {
    /// Send a message through this endpoint
    ///
    /// # Arguments
    ///
    /// * `message` - Message buffer to send
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success, or an error.
    pub fn send(&self, message: &[u8]) -> Result<()> {
        let result = unsafe {
            let mut res: usize;
            core::arch::asm!(
                "mov x8, {syscall_num}",
                "mov x0, {cap_slot}",
                "mov x1, {msg_ptr}",
                "mov x2, {msg_len}",
                "svc #0",
                "mov {result}, x0",
                syscall_num = in(reg) 0x20u64, // SYS_IPC_SEND
                cap_slot = in(reg) self.cap_slot,
                msg_ptr = in(reg) message.as_ptr() as usize,
                msg_len = in(reg) message.len(),
                result = out(reg) res,
                out("x8") _,
                out("x0") _,
                out("x1") _,
                out("x2") _,
            );
            res
        };

        if result == 0 {
            Ok(())
        } else {
            Err(crate::BrokerError::SyscallFailed(result))
        }
    }

    /// Receive a message from this endpoint
    ///
    /// # Arguments
    ///
    /// * `buffer` - Buffer to receive message into
    ///
    /// # Returns
    ///
    /// Returns the number of bytes received, or an error.
    pub fn recv(&self, buffer: &mut [u8]) -> Result<usize> {
        let result = unsafe {
            let mut bytes_received: usize;
            core::arch::asm!(
                "mov x8, {syscall_num}",
                "mov x0, {cap_slot}",
                "mov x1, {buf_ptr}",
                "mov x2, {buf_len}",
                "svc #0",
                "mov {result}, x0",
                syscall_num = in(reg) 0x21u64, // SYS_IPC_RECV
                cap_slot = in(reg) self.cap_slot,
                buf_ptr = in(reg) buffer.as_mut_ptr() as usize,
                buf_len = in(reg) buffer.len(),
                result = out(reg) bytes_received,
                out("x8") _,
                out("x0") _,
                out("x1") _,
                out("x2") _,
            );
            bytes_received
        };

        if result == usize::MAX {
            Err(crate::BrokerError::SyscallFailed(result))
        } else {
            Ok(result)
        }
    }

    /// Call: Send message and wait for reply
    ///
    /// # Arguments
    ///
    /// * `request` - Request message buffer
    /// * `reply` - Buffer for reply message
    ///
    /// # Returns
    ///
    /// Returns the number of bytes received in reply, or an error.
    pub fn call(&self, request: &[u8], reply: &mut [u8]) -> Result<usize> {
        let result = unsafe {
            let mut bytes_received: usize;
            core::arch::asm!(
                "mov x8, {syscall_num}",
                "mov x0, {cap_slot}",
                "mov x1, {req_ptr}",
                "mov x2, {req_len}",
                "mov x3, {rep_ptr}",
                "mov x4, {rep_len}",
                "svc #0",
                "mov {result}, x0",
                syscall_num = in(reg) 0x22u64, // SYS_IPC_CALL
                cap_slot = in(reg) self.cap_slot,
                req_ptr = in(reg) request.as_ptr() as usize,
                req_len = in(reg) request.len(),
                rep_ptr = in(reg) reply.as_mut_ptr() as usize,
                rep_len = in(reg) reply.len(),
                result = out(reg) bytes_received,
                out("x8") _,
                out("x0") _,
                out("x1") _,
                out("x2") _,
                out("x3") _,
                out("x4") _,
            );
            bytes_received
        };

        if result == usize::MAX {
            Err(crate::BrokerError::SyscallFailed(result))
        } else {
            Ok(result)
        }
    }
}

/// Endpoint Manager
///
/// Tracks IPC endpoints and provides endpoint creation.
pub struct EndpointManager {
    /// Next endpoint ID
    next_endpoint_id: usize,
}

impl EndpointManager {
    /// Create a new Endpoint Manager
    pub(crate) fn new() -> Self {
        Self {
            next_endpoint_id: 0,
        }
    }

    /// Create a new IPC endpoint
    ///
    /// Allocates a capability slot and creates an endpoint in the kernel.
    pub(crate) fn create_endpoint(&mut self, cap_slot: usize) -> Result<Endpoint> {
        // Make syscall to kernel to create IPC endpoint
        let result_slot = unsafe {
            let mut slot: usize;
            core::arch::asm!(
                "mov x8, {syscall_num}",
                "svc #0",
                "mov {result}, x0",
                syscall_num = in(reg) 0x13u64, // SYS_ENDPOINT_CREATE
                result = out(reg) slot,
                out("x8") _,
                out("x0") _,
            );
            slot
        };

        // Check for error (u64::MAX = -1)
        if result_slot == usize::MAX {
            return Err(crate::BrokerError::SyscallFailed(result_slot));
        }

        let id = self.next_endpoint_id;
        self.next_endpoint_id += 1;

        Ok(Endpoint { cap_slot, id })
    }
}
