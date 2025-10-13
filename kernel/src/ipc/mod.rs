//! Inter-Process Communication (IPC)
//!
//! This module implements synchronous IPC following the seL4 model.
//! IPC is the fundamental mechanism for threads to communicate and
//! transfer capabilities between protection domains.
//!
//! ## IPC Model
//!
//! **Synchronous Rendezvous**:
//! - Sender and receiver must both be ready
//! - No buffering - direct transfer
//! - Thread blocks until partner arrives
//!
//! **Message Passing**:
//! - Fast path: Up to 8 words in CPU registers
//! - Extended: Up to 64 words via IPC buffer
//! - Capability transfer: Up to 3 capabilities per message
//!
//! **Operations**:
//! - `send()`: Send message to endpoint
//! - `recv()`: Receive message from endpoint
//! - `call()`: Send and block for reply (RPC)
//! - `reply()`: Reply to caller
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Sender
//! let msg = Message::with_regs(0x42, &[1, 2, 3, 4]);
//! ipc::send(endpoint, sender_tcb, msg)?;
//!
//! // Receiver
//! let msg = ipc::recv(endpoint, receiver_tcb)?;
//! ```

pub mod message;
pub mod operations;
pub mod cap_transfer;

// Re-export main types
pub use message::{Message, IpcBuffer, IpcError, MAX_MSG_REGS, FAST_PATH_REGS, MAX_CAPS};
pub use operations::{send, recv};
pub use cap_transfer::{TransferMode, grant_capability, mint_capability, derive_capability};
