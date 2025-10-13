//! Call/Reply IPC Operations
//!
//! This module implements RPC-style call/reply semantics on top of send/receive.
//! Call/reply provides a convenient abstraction for request-response patterns.
//!
//! ## Call/Reply Model
//!
//! ```text
//! Client                   Server
//!   |                         |
//!   | call(ep, request)       |
//!   |------------------------>|
//!   |  (blocks on reply)      | recv(ep) -> gets request + reply_cap
//!   |                         |
//!   |                         | ... process request ...
//!   |                         |
//!   |      reply(reply_cap, response)
//!   |<------------------------|
//!   | (unblocked)             |
//!   v                         v
//! ```
//!
//! ## Key Features
//!
//! - **Implicit reply capability**: Call automatically creates a reply cap
//! - **One-time use**: Reply capability consumed after use
//! - **Synchronous RPC**: Caller blocks until reply received
//! - **Type-safe**: Reply cap can only be used once
//!
//! ## Implementation Notes
//!
//! For Phase 6, we'll use a simplified approach:
//! - Reply capability stored in caller's TCB
//! - Server gets reply cap in message
//! - Reply operation finds caller via cap and unblocks
//!
//! Future optimization: Use dedicated reply object type for better isolation.

use crate::objects::{Capability, CapType, CapRights, TCB, Endpoint, ThreadState};
use super::message::{Message, IpcError};
use super::operations::transfer_message;

/// Call: Send message and block for reply (RPC-style)
///
/// This combines send() with an implicit reply capability grant.
/// The caller blocks in `BlockedOnReply` state until the server replies.
///
/// # Arguments
///
/// * `endpoint_cap` - Capability to the endpoint (must have WRITE right for send)
/// * `caller` - TCB of the calling thread (will be blocked on reply)
/// * `msg` - Message to send (includes data and capabilities)
///
/// # Returns
///
/// On success, returns the reply message from the server.
/// On error, returns IpcError (capability errors, null pointer, etc.).
///
/// # Safety
///
/// - `endpoint_cap` must be a valid Endpoint capability
/// - `caller` must be a valid TCB pointer
/// - Caller must not already be blocked
///
/// # Notes
///
/// - Reply capability is implicitly generated and stored in caller TCB
/// - Server receives reply cap as part of the message context
/// - This operation requires scheduler integration (Chapter 6) to properly yield
pub unsafe fn call(
    endpoint_cap: &Capability,
    caller: *mut TCB,
    mut msg: Message,
) -> Result<Message, IpcError> {
    // Validate inputs
    if caller.is_null() {
        return Err(IpcError::NullPointer);
    }

    if endpoint_cap.cap_type() != CapType::Endpoint {
        return Err(IpcError::InvalidCapability);
    }

    // Check capability rights - need WRITE to send
    if !endpoint_cap.rights().contains(CapRights::WRITE) {
        return Err(IpcError::InsufficientRights);
    }

    let endpoint_ptr = endpoint_cap.object_ptr() as *mut Endpoint;
    if endpoint_ptr.is_null() {
        return Err(IpcError::NullPointer);
    }

    let endpoint = &mut *endpoint_ptr;
    let caller_ref = &mut *caller;

    // Create implicit reply capability
    // In simplified model: store caller pointer in a global reply table
    // Server will receive the reply capability via the message badge/label
    let reply_cap = create_reply_capability(caller);

    // Add reply cap to message (if there's room)
    // For now, we'll use the badge field to encode the reply cap
    // Real implementation would add to msg.caps array
    msg.set_label(msg.label() | REPLY_CAP_FLAG);

    // Store reply cap in caller's context so reply() can find it
    store_reply_capability(caller, reply_cap);

    // Update caller state to BlockedOnReply
    caller_ref.set_state(ThreadState::BlockedOnReply);

    // Check if there's a receiver waiting
    if endpoint.has_receivers() {
        // Fast path: receiver is waiting, transfer immediately
        if let Some(receiver) = endpoint.dequeue_receiver() {
            // Transfer message to receiver
            transfer_message(caller, receiver, &msg)?;

            // Store caller in receiver's context so reply() can find the caller
            (*receiver).store_reply_target(caller);

            // Unblock receiver (it's now ready to process the message)
            (*receiver).unblock();

            // TODO (Chapter 6): Yield to scheduler
            // For now, caller remains blocked until reply() is called

            // The reply will come later via reply() call
            // For now, return a placeholder - in real impl, this would
            // yield and return only when reply() unblocks us

            // Placeholder: In testing, we'll manually call reply()
            // In production with scheduler, this would yield and resume
            // when reply() is called

            return Ok(Message::empty());
        }
    }

    // Slow path: no receiver, queue the caller
    endpoint.queue_send(caller);

    // TODO (Chapter 6): Yield to scheduler
    // Thread should yield here and resume when:
    // 1. A receiver arrives and processes the message
    // 2. The receiver calls reply()

    // For now, return placeholder
    // Real implementation would yield and return reply message on resume
    Ok(Message::empty())
}

/// Reply: Send response back to caller
///
/// This completes an RPC by sending a response message back to the
/// thread that called call(). The reply capability is consumed (one-time use).
///
/// # Arguments
///
/// * `reply_cap` - Reply capability (obtained from call())
/// * `replier` - TCB of the replying thread (typically a server)
/// * `msg` - Reply message to send back
///
/// # Returns
///
/// On success, returns Ok(()).
/// On error, returns IpcError (invalid cap, caller not found, etc.).
///
/// # Safety
///
/// - `reply_cap` must be a valid Reply capability
/// - `replier` must be a valid TCB pointer
/// - Reply cap can only be used once
///
/// # Notes
///
/// - Unblocks the original caller (changes state from BlockedOnReply to Runnable)
/// - Transfers message to caller's context
/// - Destroys the reply capability (one-time use)
/// - Requires scheduler integration (Chapter 6) to wake the caller
pub unsafe fn reply(
    reply_cap: &Capability,
    replier: *mut TCB,
    msg: &Message,
) -> Result<(), IpcError> {
    // Validate inputs
    if replier.is_null() {
        return Err(IpcError::NullPointer);
    }

    // For Phase 6 simplified model: reply_cap encodes the caller pointer
    // Real implementation would have dedicated Reply capability type
    let caller = extract_reply_target(reply_cap)?;

    if caller.is_null() {
        return Err(IpcError::InvalidCapability);
    }

    let caller_ref = &mut *caller;

    // Verify caller is actually blocked on reply
    if caller_ref.state() != ThreadState::BlockedOnReply {
        return Err(IpcError::ThreadNotBlocked);
    }

    // Transfer message to caller
    transfer_message(replier, caller, msg)?;

    // Unblock caller - make it runnable
    caller_ref.set_state(ThreadState::Runnable);

    // Destroy reply capability (one-time use)
    destroy_reply_capability(reply_cap);

    // TODO (Chapter 6): Wake the caller in scheduler
    // The scheduler should mark the caller as ready to run

    Ok(())
}

// ============================================================================
// Reply Capability Management (Simplified for Phase 6)
// ============================================================================

/// Flag in message label indicating reply cap is included
const REPLY_CAP_FLAG: u64 = 1 << 31;

/// Maximum number of concurrent reply capabilities
/// In real implementation, this would be dynamic or use a proper allocator
const MAX_REPLY_CAPS: usize = 256;

/// Global reply capability table (simplified for Phase 6)
/// Maps reply cap ID -> caller TCB pointer
///
/// TODO: Replace with proper capability object in future phases
static mut REPLY_CAP_TABLE: [*mut TCB; MAX_REPLY_CAPS] = [core::ptr::null_mut(); MAX_REPLY_CAPS];
static mut NEXT_REPLY_ID: usize = 1; // 0 reserved for null

/// Create a reply capability for the given caller
///
/// Returns a capability that encodes the caller's pointer.
/// In real implementation, would allocate a proper Reply object.
unsafe fn create_reply_capability(caller: *mut TCB) -> Capability {
    // Find free slot in reply table
    let reply_id = NEXT_REPLY_ID;
    NEXT_REPLY_ID = (NEXT_REPLY_ID + 1) % MAX_REPLY_CAPS;
    if NEXT_REPLY_ID == 0 {
        NEXT_REPLY_ID = 1; // Skip 0 (reserved for null)
    }

    // Store caller in table
    REPLY_CAP_TABLE[reply_id] = caller;

    // Create capability that encodes the reply ID
    // Using badge field to store reply ID
    let mut cap = Capability::new(CapType::Reply, reply_id);
    cap.set_rights(CapRights::WRITE); // Reply requires WRITE
    cap.set_badge(reply_id as u64);
    cap
}

/// Store reply capability in caller's TCB context
///
/// The caller needs to remember its reply cap so it can be
/// retrieved when reply() is called.
unsafe fn store_reply_capability(caller: *mut TCB, _reply_cap: Capability) {
    // In simplified model, the reply cap is stored in global table
    // indexed by reply ID, so we don't need to store in TCB
    // Real implementation might store in TCB's IPC buffer or dedicated field
    let _ = caller; // Unused in simplified model
}

/// Extract the caller TCB pointer from a reply capability
unsafe fn extract_reply_target(reply_cap: &Capability) -> Result<*mut TCB, IpcError> {
    if reply_cap.cap_type() != CapType::Reply {
        return Err(IpcError::InvalidCapability);
    }

    let reply_id = reply_cap.badge() as usize;
    if reply_id == 0 || reply_id >= MAX_REPLY_CAPS {
        return Err(IpcError::InvalidCapability);
    }

    let caller = REPLY_CAP_TABLE[reply_id];
    if caller.is_null() {
        return Err(IpcError::InvalidCapability);
    }

    Ok(caller)
}

/// Destroy a reply capability (one-time use)
unsafe fn destroy_reply_capability(reply_cap: &Capability) {
    if reply_cap.cap_type() != CapType::Reply {
        return;
    }

    let reply_id = reply_cap.badge() as usize;
    if reply_id > 0 && reply_id < MAX_REPLY_CAPS {
        // Clear the entry
        REPLY_CAP_TABLE[reply_id] = core::ptr::null_mut();
    }
}

// ============================================================================
// TCB Extension for Reply Capability Support
// ============================================================================

/// Extension trait for TCB to support reply capability storage
///
/// This adds methods needed for call/reply semantics.
/// In real implementation, these would be in TCB itself.
trait TcbReplyExt {
    unsafe fn store_reply_target(&mut self, target: *mut TCB);
    unsafe fn get_reply_target(&self) -> *mut TCB;
}

impl TcbReplyExt for TCB {
    unsafe fn store_reply_target(&mut self, _target: *mut TCB) {
        // In simplified model, stored in global table
        // Real implementation would store in TCB field
    }

    unsafe fn get_reply_target(&self) -> *mut TCB {
        // In simplified model, retrieved from global table via reply cap
        // Real implementation would read from TCB field
        core::ptr::null_mut()
    }
}
