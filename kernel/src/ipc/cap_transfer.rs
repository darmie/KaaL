//! Capability Transfer Protocol
//!
//! This module implements the capability transfer protocol for IPC.
//! When capabilities are sent via IPC, they can be transferred in three modes:
//!
//! ## Transfer Modes
//!
//! 1. **Grant**: Transfer full capability rights
//!    - Sender loses access (capability moved)
//!    - Receiver gains full rights
//!    - Original capability deleted from sender's CSpace
//!
//! 2. **Mint**: Create badged copy
//!    - Sender retains original capability
//!    - Receiver gets badged endpoint capability
//!    - Used to identify senders
//!
//! 3. **Derive**: Create restricted copy
//!    - Sender retains original capability
//!    - Receiver gets capability with reduced rights
//!    - Cannot escalate privileges
//!
//! ## Security Model
//!
//! - Capabilities cannot be forged
//! - Rights can only be reduced, never increased
//! - Badge values are preserved during transfer
//! - Invalid transfers fail safely
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Grant capability (move)
//! grant_capability(sender_cspace, receiver_cspace, src_slot, dest_slot)?;
//!
//! // Mint badged endpoint
//! mint_capability(sender_cspace, receiver_cspace, src_slot, dest_slot, badge)?;
//!
//! // Derive with reduced rights
//! derive_capability(sender_cspace, receiver_cspace, src_slot, dest_slot, new_rights)?;
//! ```

use super::message::IpcError;
use crate::objects::{Capability, CapRights, CNode};

/// Capability transfer mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    /// Grant: Move capability (sender loses access)
    Grant,

    /// Mint: Create badged copy (for endpoints)
    Mint { badge: u64 },

    /// Derive: Create restricted copy
    Derive { rights: CapRights },
}

/// Grant a capability (move from sender to receiver)
///
/// This is the strongest form of capability transfer:
/// - Sender loses access to the capability
/// - Receiver gains full capability rights
/// - Capability is deleted from sender's CSpace
///
/// # Arguments
///
/// * `sender_cspace` - Sender's capability space root
/// * `receiver_cspace` - Receiver's capability space root
/// * `src_slot` - Slot in sender's CSpace
/// * `dest_slot` - Slot in receiver's CSpace
///
/// # Returns
///
/// * `Ok(())` - Capability granted successfully
/// * `Err(IpcError)` - Grant failed
///
/// # Safety
///
/// CSpace pointers must be valid.
pub unsafe fn grant_capability(
    sender_cspace: *mut CNode,
    receiver_cspace: *mut CNode,
    src_slot: usize,
    dest_slot: usize,
) -> Result<(), IpcError> {
    if sender_cspace.is_null() || receiver_cspace.is_null() {
        return Err(IpcError::NullPointer);
    }

    // Lookup source capability
    let cap = (*sender_cspace).lookup(src_slot)
        .ok_or(IpcError::InvalidCapability)?;

    // Check if sender has GRANT right
    if !cap.rights().contains(CapRights::GRANT) {
        return Err(IpcError::InsufficientRights);
    }

    // Copy capability to receiver
    let cap_copy = *cap;
    (*receiver_cspace).insert(dest_slot, cap_copy)
        .map_err(|e| IpcError::CapError(e))?;

    // Delete from sender (grant = move)
    (*sender_cspace).delete(src_slot)
        .map_err(|e| IpcError::CapError(e))?;

    Ok(())
}

/// Mint a badged capability (create badged copy)
///
/// This creates a copy of an endpoint capability with a badge value.
/// The badge allows the receiver to identify which capability was used
/// when receiving messages.
///
/// # Arguments
///
/// * `sender_cspace` - Sender's capability space root
/// * `receiver_cspace` - Receiver's capability space root
/// * `src_slot` - Slot in sender's CSpace
/// * `dest_slot` - Slot in receiver's CSpace
/// * `badge` - Badge value to apply
///
/// # Returns
///
/// * `Ok(())` - Capability minted successfully
/// * `Err(IpcError)` - Minting failed
///
/// # Safety
///
/// CSpace pointers must be valid.
pub unsafe fn mint_capability(
    sender_cspace: *mut CNode,
    receiver_cspace: *mut CNode,
    src_slot: usize,
    dest_slot: usize,
    badge: u64,
) -> Result<(), IpcError> {
    if sender_cspace.is_null() || receiver_cspace.is_null() {
        return Err(IpcError::NullPointer);
    }

    // Lookup source capability
    let cap = (*sender_cspace).lookup(src_slot)
        .ok_or(IpcError::InvalidCapability)?;

    // Check if sender has GRANT right (needed for minting)
    if !cap.rights().contains(CapRights::GRANT) {
        return Err(IpcError::InsufficientRights);
    }

    // Mint badged capability
    let badged_cap = cap.mint(badge)
        .map_err(|e| IpcError::CapError(e))?;

    // Insert into receiver's CSpace
    (*receiver_cspace).insert(dest_slot, badged_cap)
        .map_err(|e| IpcError::CapError(e))?;

    Ok(())
}

/// Derive a capability (create restricted copy)
///
/// This creates a copy of a capability with reduced rights.
/// The new capability cannot have more rights than the original.
///
/// # Arguments
///
/// * `sender_cspace` - Sender's capability space root
/// * `receiver_cspace` - Receiver's capability space root
/// * `src_slot` - Slot in sender's CSpace
/// * `dest_slot` - Slot in receiver's CSpace
/// * `new_rights` - Rights for derived capability (must be subset)
///
/// # Returns
///
/// * `Ok(())` - Capability derived successfully
/// * `Err(IpcError)` - Derivation failed
///
/// # Safety
///
/// CSpace pointers must be valid.
pub unsafe fn derive_capability(
    sender_cspace: *mut CNode,
    receiver_cspace: *mut CNode,
    src_slot: usize,
    dest_slot: usize,
    new_rights: CapRights,
) -> Result<(), IpcError> {
    if sender_cspace.is_null() || receiver_cspace.is_null() {
        return Err(IpcError::NullPointer);
    }

    // Lookup source capability
    let cap = (*sender_cspace).lookup(src_slot)
        .ok_or(IpcError::InvalidCapability)?;

    // Check if sender has GRANT right (needed for derivation)
    if !cap.rights().contains(CapRights::GRANT) {
        return Err(IpcError::InsufficientRights);
    }

    // Derive capability with reduced rights
    let derived_cap = cap.derive(new_rights)
        .map_err(|e| IpcError::CapError(e))?;

    // Insert into receiver's CSpace
    (*receiver_cspace).insert(dest_slot, derived_cap)
        .map_err(|e| IpcError::CapError(e))?;

    Ok(())
}

/// Transfer capabilities from sender to receiver according to message
///
/// This is the high-level capability transfer function used during IPC.
/// It processes all capabilities in a message and transfers them according
/// to their transfer mode.
///
/// # Arguments
///
/// * `sender_cspace` - Sender's capability space root
/// * `receiver_cspace` - Receiver's capability space root
/// * `caps` - Array of (capability, mode, dest_slot) tuples
///
/// # Returns
///
/// * `Ok(())` - All capabilities transferred successfully
/// * `Err(IpcError)` - Transfer failed (transaction may be partial)
///
/// # Safety
///
/// CSpace pointers must be valid.
pub unsafe fn transfer_capabilities(
    sender_cspace: *mut CNode,
    receiver_cspace: *mut CNode,
    caps: &[(Capability, TransferMode, usize)],
) -> Result<(), IpcError> {
    for (cap, mode, dest_slot) in caps {
        // Find source slot (linear scan - could be optimized)
        let src_slot = find_capability_slot(sender_cspace, cap)?;

        match mode {
            TransferMode::Grant => {
                grant_capability(sender_cspace, receiver_cspace, src_slot, *dest_slot)?;
            }
            TransferMode::Mint { badge } => {
                mint_capability(sender_cspace, receiver_cspace, src_slot, *dest_slot, *badge)?;
            }
            TransferMode::Derive { rights } => {
                derive_capability(sender_cspace, receiver_cspace, src_slot, *dest_slot, *rights)?;
            }
        }
    }

    Ok(())
}

/// Find the slot containing a specific capability
///
/// Linear search through CSpace to find capability.
/// This is not optimal but works for small CSpaces.
///
/// # TODO
///
/// - Implement capability address resolution
/// - Use CNode guard bits for efficient lookup
/// - Cache frequently used capability slots
unsafe fn find_capability_slot(cspace: *mut CNode, target_cap: &Capability) -> Result<usize, IpcError> {
    if cspace.is_null() {
        return Err(IpcError::NullPointer);
    }

    // Linear search (inefficient but correct)
    let num_slots = (*cspace).num_slots();
    for slot in 0..num_slots {
        if let Some(cap) = (*cspace).lookup(slot) {
            // Compare capability object pointers
            if cap.object_ptr() == target_cap.object_ptr() {
                return Ok(slot);
            }
        }
    }

    Err(IpcError::InvalidCapability)
}

/// Encode transfer mode into IPC buffer
///
/// Transfer modes are encoded as bit patterns:
/// - Bits 0-1: Mode (00=Grant, 01=Mint, 10=Derive)
/// - Bits 2-63: Mode-specific data (badge or rights)
pub fn encode_transfer_mode(mode: TransferMode) -> u64 {
    match mode {
        TransferMode::Grant => 0b00,
        TransferMode::Mint { badge } => {
            0b01 | (badge << 2)
        }
        TransferMode::Derive { rights } => {
            0b10 | ((rights.bits() as u64) << 2)
        }
    }
}

/// Decode transfer mode from IPC buffer
pub fn decode_transfer_mode(encoded: u64) -> Result<TransferMode, IpcError> {
    let mode_bits = encoded & 0b11;
    let data = encoded >> 2;

    match mode_bits {
        0b00 => Ok(TransferMode::Grant),
        0b01 => Ok(TransferMode::Mint { badge: data }),
        0b10 => {
            let rights = CapRights::from_bits(data as u8);
            Ok(TransferMode::Derive { rights })
        }
        _ => Err(IpcError::InvalidCapability),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_mode_encoding() {
        let grant = TransferMode::Grant;
        let encoded = encode_transfer_mode(grant);
        let decoded = decode_transfer_mode(encoded).unwrap();
        assert_eq!(decoded, grant);
    }

    #[test]
    fn test_mint_encoding() {
        let mint = TransferMode::Mint { badge: 0x1234 };
        let encoded = encode_transfer_mode(mint);
        let decoded = decode_transfer_mode(encoded).unwrap();
        assert_eq!(decoded, mint);
    }

    #[test]
    fn test_derive_encoding() {
        let derive = TransferMode::Derive {
            rights: CapRights::READ | CapRights::WRITE
        };
        let encoded = encode_transfer_mode(derive);
        let decoded = decode_transfer_mode(encoded).unwrap();
        assert_eq!(decoded, derive);
    }

    #[test]
    fn test_invalid_mode() {
        let invalid = 0b11; // Invalid mode bits
        assert!(decode_transfer_mode(invalid).is_err());
    }
}
