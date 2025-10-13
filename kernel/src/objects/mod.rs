//! Kernel Object Model
//!
//! This module implements the capability-based object system for KaaL.
//! All kernel resources are represented as objects, and access is controlled
//! through capabilities.
//!
//! ## Object Types
//!
//! -  **Untyped Memory**: Raw memory that can be retyped into other objects
//! - **CNode**: Capability node - container for capabilities
//! - **TCB**: Thread Control Block - represents a thread
//! - **Endpoint**: Synchronous IPC rendezvous point
//! - **Notification**: Asynchronous signaling
//! - **VSpace**: Virtual address space root
//! - **Page**: Physical memory page
//! - **IRQ Handler/Control**: Interrupt handling
//!
//! ## Capability-Based Security
//!
//! - All access to objects is through capabilities
//! - Capabilities grant specific rights (Read, Write, Grant)
//! - Capabilities can be derived with reduced rights
//! - User space cannot forge capabilities
//! - Capabilities stored in CNodes

pub mod capability;
pub mod cnode;
pub mod endpoint;
pub mod tcb;
pub mod untyped;

// Re-export main types
pub use capability::{Capability, CapType, CapRights, CapError};
pub use cnode::CNode;
pub use endpoint::Endpoint;
pub use tcb::{TCB, ThreadState};
pub use untyped::{UntypedMemory, ObjectType};
