//! IPC Operations - Send and Receive
//!
//! This module implements the core IPC operations:
//! - `send()`: Send message to endpoint (blocking)
//! - `recv()`: Receive message from endpoint (blocking)
//! - `call()`: Send and block for reply (RPC)
//! - `reply()`: Reply to caller
//!
//! ## Synchronous IPC Model
//!
//! IPC follows a synchronous rendezvous model:
//! 1. If sender arrives first, sender blocks on endpoint send queue
//! 2. If receiver arrives first, receiver blocks on endpoint receive queue
//! 3. When both present, message transfer happens immediately
//! 4. Both threads unblock and continue execution
//!
//! ## Fast Path Optimization
//!
//! - Fast path: ≤8 words in registers, no caps → direct register transfer
//! - Slow path: >8 words or caps present → use IPC buffer
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Sender thread
//! let msg = Message::with_regs(0x42, &[1, 2, 3, 4]);
//! ipc::send(endpoint_cap, sender_tcb, msg)?;
//!
//! // Receiver thread
//! let msg = ipc::recv(endpoint_cap, receiver_tcb)?;
//! ```

use super::message::{Message, IpcBuffer, IpcError, FAST_PATH_REGS, MAX_CAPS};
use crate::objects::{Capability, CapType, CapRights, TCB, Endpoint, ThreadState};

/// Send a message to an endpoint (blocking)
///
/// This is the fundamental IPC send operation. The sender thread will:
/// 1. Check capability rights (must have Write permission)
/// 2. Try to match with a waiting receiver
/// 3. If receiver present, transfer message and unblock both
/// 4. If no receiver, block on endpoint send queue
///
/// # Arguments
///
/// * `endpoint_cap` - Capability to endpoint (must have Write right)
/// * `sender` - Sending thread's TCB
/// * `msg` - Message to send
///
/// # Returns
///
/// * `Ok(())` - Message sent successfully
/// * `Err(IpcError)` - Send failed (invalid cap, insufficient rights, etc.)
///
/// # Fast Path
///
/// If message fits in registers (≤8 words) and has no capabilities,
/// the fast path directly transfers registers without touching IPC buffer.
pub unsafe fn send(
    endpoint_cap: &Capability,
    sender: *mut TCB,
    msg: Message,
) -> Result<(), IpcError> {
    // Validate capability
    if endpoint_cap.cap_type() != CapType::Endpoint {
        return Err(IpcError::InvalidCapability);
    }

    if !endpoint_cap.rights().contains(CapRights::WRITE) {
        return Err(IpcError::InsufficientRights);
    }

    // Get endpoint object
    let endpoint = endpoint_cap.object_ptr() as *mut Endpoint;
    if endpoint.is_null() {
        return Err(IpcError::NullPointer);
    }

    // Try to match with waiting receiver
    if (*endpoint).has_receivers() {
        // Fast path: receiver already waiting
        let receiver = (*endpoint).dequeue_receiver()
            .ok_or(IpcError::QueueCorrupted)?;

        // Transfer message
        transfer_message(sender, receiver, &msg)?;

        // Unblock receiver
        (*receiver).unblock();

        Ok(())
    } else {
        // No receiver - block on send queue
        // Store message in sender's IPC buffer
        write_message_to_buffer(sender, &msg)?;

        // Block sender on endpoint
        (*sender).set_state(ThreadState::BlockedOnSend { endpoint: endpoint as usize });
        (*endpoint).queue_send(sender);

        // Yield to scheduler - block until receiver arrives
        crate::scheduler::block_current();

        // When we resume here, message has been transferred
        Ok(())
    }
}

/// Receive a message from an endpoint (blocking)
///
/// This is the fundamental IPC receive operation. The receiver thread will:
/// 1. Check capability rights (must have Read permission)
/// 2. Try to match with a waiting sender
/// 3. If sender present, transfer message and unblock both
/// 4. If no sender, block on endpoint receive queue
///
/// # Arguments
///
/// * `endpoint_cap` - Capability to endpoint (must have Read right)
/// * `receiver` - Receiving thread's TCB
///
/// # Returns
///
/// * `Ok(Message)` - Message received successfully
/// * `Err(IpcError)` - Receive failed
///
/// # Blocking Behavior
///
/// If no sender is waiting, receiver blocks on the endpoint receive queue
/// until a sender arrives. The message is then transferred and the receiver
/// is unblocked by the sender.
pub unsafe fn recv(
    endpoint_cap: &Capability,
    receiver: *mut TCB,
) -> Result<Message, IpcError> {
    // Validate capability
    if endpoint_cap.cap_type() != CapType::Endpoint {
        return Err(IpcError::InvalidCapability);
    }

    if !endpoint_cap.rights().contains(CapRights::READ) {
        return Err(IpcError::InsufficientRights);
    }

    // Get endpoint object
    let endpoint = endpoint_cap.object_ptr() as *mut Endpoint;
    if endpoint.is_null() {
        return Err(IpcError::NullPointer);
    }

    // Try to match with waiting sender
    if (*endpoint).has_senders() {
        // Fast path: sender already waiting
        let sender = (*endpoint).dequeue_sender()
            .ok_or(IpcError::QueueCorrupted)?;

        // Read message from sender's IPC buffer
        let msg = read_message_from_buffer(sender)?;

        // Unblock sender
        (*sender).unblock();

        Ok(msg)
    } else {
        // No sender - block on receive queue
        (*receiver).set_state(ThreadState::BlockedOnReceive { endpoint: endpoint as usize });
        (*endpoint).queue_receive(receiver);

        // Yield to scheduler - block until sender arrives
        crate::scheduler::block_current();

        // When we resume here, message has been transferred to our IPC buffer
        read_message_from_buffer(receiver)
    }
}

/// Transfer message from sender to receiver
///
/// This performs the actual message transfer between threads.
/// Uses fast path (registers only) or slow path (IPC buffer) depending on message size.
///
/// # Fast Path (≤8 words, no caps)
///
/// Message is transferred directly via trap frame registers:
/// - x0-x7 contain message data
/// - No IPC buffer access needed
///
/// # Slow Path (>8 words or caps present)
///
/// Message is transferred via IPC buffers:
/// 1. Read from sender's IPC buffer
/// 2. Write to receiver's IPC buffer
/// 3. Transfer capabilities if present
pub(crate) unsafe fn transfer_message(
    sender: *mut TCB,
    receiver: *mut TCB,
    msg: &Message,
) -> Result<(), IpcError> {
    if msg.is_fast_path() {
        // Fast path: transfer via registers
        transfer_fast_path(sender, receiver, msg)
    } else {
        // Slow path: transfer via IPC buffer
        transfer_slow_path(sender, receiver, msg)
    }
}

/// Fast path message transfer (registers only)
///
/// Directly copies message registers into receiver's trap frame.
/// This is the most efficient path for small messages without capabilities.
unsafe fn transfer_fast_path(
    _sender: *mut TCB,
    receiver: *mut TCB,
    msg: &Message,
) -> Result<(), IpcError> {
    // Get receiver's context (trap frame)
    let recv_context = (*receiver).context_mut();

    // Transfer message label and registers
    // x0 = label
    recv_context.x0 = msg.label();

    // x1-x7 = message registers (up to 8 words)
    let regs = msg.regs();
    let len = msg.len().min(FAST_PATH_REGS - 1); // -1 because x0 has label

    for i in 0..len {
        match i {
            0 => recv_context.x1 = regs[i],
            1 => recv_context.x2 = regs[i],
            2 => recv_context.x3 = regs[i],
            3 => recv_context.x4 = regs[i],
            4 => recv_context.x5 = regs[i],
            5 => recv_context.x6 = regs[i],
            6 => recv_context.x7 = regs[i],
            _ => break,
        }
    }

    Ok(())
}

/// Slow path message transfer (IPC buffer)
///
/// Copies message from sender's IPC buffer to receiver's IPC buffer.
/// Also handles capability transfer.
unsafe fn transfer_slow_path(
    sender: *mut TCB,
    receiver: *mut TCB,
    msg: &Message,
) -> Result<(), IpcError> {
    // Write to sender's buffer first (if not already there)
    write_message_to_buffer(sender, msg)?;

    // Read from sender's buffer
    let msg = read_message_from_buffer(sender)?;

    // Write to receiver's buffer
    write_message_to_buffer(receiver, &msg)?;

    // Also transfer via registers for fast access
    transfer_fast_path(sender, receiver, &msg)?;

    Ok(())
}

/// Write message to a thread's IPC buffer
///
/// # Safety
///
/// TCB must have a valid IPC buffer pointer configured.
unsafe fn write_message_to_buffer(tcb: *mut TCB, msg: &Message) -> Result<(), IpcError> {
    let ipc_buffer_addr = (*tcb).ipc_buffer();
    if ipc_buffer_addr.as_u64() == 0 {
        return Err(IpcError::NoIpcBuffer);
    }

    // Get IPC buffer
    let ipc_buffer = ipc_buffer_addr.as_u64() as *mut IpcBuffer;
    if ipc_buffer.is_null() {
        return Err(IpcError::NoIpcBuffer);
    }

    // Write message data (words beyond first 8)
    let regs = msg.regs();
    let len = msg.len();

    if len > FAST_PATH_REGS {
        // Copy extended message words
        let extended_len = len - FAST_PATH_REGS;
        for i in 0..extended_len {
            (*ipc_buffer).msg[i] = regs[FAST_PATH_REGS + i];
        }
    }

    // Write capabilities if present
    if msg.num_caps() > 0 {
        (*ipc_buffer).caps_unwrapped = msg.num_caps() as u64;

        for i in 0..msg.num_caps() {
            if let Some(cap) = msg.cap(i) {
                // Store capability object pointer
                // Transfer mode will be handled separately during actual transfer
                (*ipc_buffer).caps[i] = cap.object_ptr() as u64;
            }
        }
    } else {
        (*ipc_buffer).caps_unwrapped = 0;
    }

    Ok(())
}

/// Read message from a thread's IPC buffer
///
/// # Safety
///
/// TCB must have a valid IPC buffer pointer configured.
unsafe fn read_message_from_buffer(tcb: *mut TCB) -> Result<Message, IpcError> {
    let ipc_buffer_addr = (*tcb).ipc_buffer();
    if ipc_buffer_addr.as_u64() == 0 {
        return Err(IpcError::NoIpcBuffer);
    }

    // Get IPC buffer
    let ipc_buffer = ipc_buffer_addr.as_u64() as *const IpcBuffer;
    if ipc_buffer.is_null() {
        return Err(IpcError::NoIpcBuffer);
    }

    // Read message from trap frame (fast path registers)
    let context = (*tcb).context();

    // x0 = label
    let label = context.x0;

    // x1-x7 = first 7 message words
    let mut msg = Message::with_label(label);

    msg.push(context.x1).ok();
    msg.push(context.x2).ok();
    msg.push(context.x3).ok();
    msg.push(context.x4).ok();
    msg.push(context.x5).ok();
    msg.push(context.x6).ok();
    msg.push(context.x7).ok();

    // Read extended message words from IPC buffer if needed
    // TODO: Need to know actual message length - for now assume fast path only

    // Read capabilities if present
    let num_caps = (*ipc_buffer).caps_unwrapped as usize;
    if num_caps > 0 && num_caps <= MAX_CAPS {
        // Note: Capabilities are not reconstructed here - they were already transferred
        // during the IPC operation by transfer_capabilities(). The IPC buffer just
        // stores metadata about transferred capabilities.
        // Full reconstruction would require:
        // 1. Lookup capabilities in receiver's CSpace
        // 2. Rebuild Capability objects
        // 3. Add to message
        // This is deferred as it requires CSpace integration in the message path.
    }

    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::VirtAddr;

    #[test]
    fn test_fast_path_detection() {
        let msg = Message::with_regs(0x42, &[1, 2, 3, 4]);
        assert!(msg.is_fast_path());

        let mut large_msg = Message::with_label(0x42);
        for i in 0..10 {
            large_msg.push(i).unwrap();
        }
        assert!(!large_msg.is_fast_path());
    }

    #[test]
    fn test_message_with_cap() {
        let mut msg = Message::with_regs(0x42, &[1, 2, 3]);
        let cap = Capability::new(CapType::Endpoint, 0x1000);
        msg.add_cap(cap).unwrap();

        // Message with caps cannot use fast path
        assert!(!msg.is_fast_path());
    }
}
