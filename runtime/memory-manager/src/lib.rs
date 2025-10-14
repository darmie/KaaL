//! KaaL Memory Manager
//!
//! Extended memory management service that builds on top of the capability broker.
//! Provides higher-level memory allocation and management APIs.

#![no_std]

pub use capability_broker::memory_manager::*;

// Re-export for convenience
pub use capability_broker::{BrokerError, Result};
