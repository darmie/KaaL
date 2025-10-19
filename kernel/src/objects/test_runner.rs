//! Object Model Test Runner
//!
//! This module provides test functions that can be called from the kernel-test binary.
//! Each test returns bool (true = pass, false = fail) for integration with the test harness.

use super::*;
use crate::memory::{PhysAddr, VirtAddr};

// ========================================================================
// Capability Tests
// ========================================================================

pub fn test_capability_creation() -> bool {
    let cap = Capability::new(CapType::Endpoint, 0x1000);
    cap.cap_type() == CapType::Endpoint
        && cap.object_ptr() == 0x1000
        && cap.rights() == CapRights::ALL
}

pub fn test_capability_derivation() -> bool {
    let cap = Capability::new(CapType::Endpoint, 0x1000);
    let derived = match cap.derive(CapRights::READ) {
        Ok(d) => d,
        Err(_) => return false,
    };

    if derived.cap_type() != CapType::Endpoint { return false; }
    if derived.object_ptr() != 0x1000 { return false; }
    if derived.rights() != CapRights::READ { return false; }

    // Cannot derive with more rights
    derived.derive(CapRights::ALL).is_err()
}

pub fn test_capability_minting() -> bool {
    let cap = Capability::new(CapType::Endpoint, 0x1000);
    let minted = match cap.mint(0x1234) {
        Ok(m) => m,
        Err(_) => return false,
    };

    minted.cap_type() == CapType::Endpoint && minted.guard() == 0x1234
}

pub fn test_capability_rights() -> bool {
    let rw = CapRights::from_bits(0b0011); // READ | WRITE

    rw.contains(CapRights::READ)
        && rw.contains(CapRights::WRITE)
        && !rw.contains(CapRights::GRANT)
}

// ========================================================================
// CNode Tests
// ========================================================================

pub fn test_cnode_creation() -> bool {
    let mut memory = [Capability::null(); 16];
    let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

    unsafe {
        match CNode::new(4, paddr) {
            Ok(cnode) => cnode.num_slots() == 16 && cnode.size_bits() == 4,
            Err(_) => false,
        }
    }
}

pub fn test_cnode_insert_lookup() -> bool {
    let mut memory = [Capability::null(); 16];
    let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

    unsafe {
        let mut cnode = match CNode::new(4, paddr) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let cap = Capability::new(CapType::Endpoint, 0x1000);
        if cnode.insert(5, cap).is_err() { return false; }

        match cnode.lookup(5) {
            Some(retrieved) => {
                retrieved.cap_type() == CapType::Endpoint
                    && retrieved.object_ptr() == 0x1000
            }
            None => false,
        }
    }
}

pub fn test_cnode_copy_move() -> bool {
    let mut memory = [Capability::null(); 16];
    let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

    unsafe {
        let mut cnode = match CNode::new(4, paddr) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let cap = Capability::new(CapType::Endpoint, 0x1000);
        if cnode.insert(0, cap).is_err() { return false; }

        // Copy
        if cnode.copy_cap(0, 1).is_err() { return false; }
        if cnode.lookup(0).is_none() { return false; }
        if cnode.lookup(1).is_none() { return false; }

        // Move
        if cnode.move_cap(1, 2).is_err() { return false; }
        if cnode.lookup(1).is_some() { return false; }
        cnode.lookup(2).is_some()
    }
}

// ========================================================================
// TCB Tests
// ========================================================================

pub fn test_tcb_creation() -> bool {
    unsafe {
        let mut cnode_memory = [Capability::null(); 16];
        let cnode_ptr = &mut cnode_memory[0] as *mut _ as *mut CNode;

        let tcb = TCB::new(
            1,
            cnode_ptr,
            0x40000000,
            VirtAddr::new(0x10000000),
            0x200000,
            0x300000,
            TCB::CAP_ALL,
        );

        tcb.tid() == 1
            && tcb.state() == ThreadState::Inactive
            && tcb.priority() == TCB::DEFAULT_PRIORITY
    }
}

pub fn test_tcb_state_transitions() -> bool {
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
            TCB::CAP_ALL,
        );

        // Inactive -> Runnable
        tcb.activate();
        if tcb.state() != ThreadState::Runnable { return false; }

        // Runnable -> Blocked
        tcb.block_on_receive(0x5000);
        match tcb.state() {
            ThreadState::BlockedOnReceive { endpoint } if endpoint == 0x5000 => {},
            _ => return false,
        }

        // Blocked -> Runnable
        tcb.unblock();
        tcb.state() == ThreadState::Runnable
    }
}

pub fn test_tcb_priority() -> bool {
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
            TCB::CAP_ALL,
        );

        if tcb.priority() != 128 { return false; }

        tcb.set_priority(200);
        tcb.priority() == 200
    }
}

// ========================================================================
// Endpoint Tests
// ========================================================================

pub fn test_endpoint_creation() -> bool {
    let ep = Endpoint::new();
    ep.badge() == 0
        && !ep.has_senders()
        && !ep.has_receivers()
        && ep.is_idle()
}

pub fn test_endpoint_queue_operations() -> bool {
    unsafe {
        let mut ep = Endpoint::new();
        let mut cnode_memory = [Capability::null(); 16];
        let cnode_ptr = &mut cnode_memory[0] as *mut _ as *mut CNode;

        let mut sender = TCB::new(
            1,
            cnode_ptr,
            0x40000000,
            VirtAddr::new(0x10000000),
            0x200000,
            0x300000,
            TCB::CAP_ALL,
        );

        let mut receiver = TCB::new(
            2,
            cnode_ptr,
            0x40000000,
            VirtAddr::new(0x10000000),
            0x200000,
            0x300000,
            TCB::CAP_ALL,
        );

        let sender_ptr = &mut sender as *mut TCB;
        let receiver_ptr = &mut receiver as *mut TCB;

        ep.queue_send(sender_ptr);
        if !ep.has_senders() { return false; }
        if ep.send_queue_len() != 1 { return false; }

        ep.queue_receive(receiver_ptr);
        if !ep.has_receivers() { return false; }
        if ep.recv_queue_len() != 1 { return false; }

        // Try match
        match ep.try_match() {
            Some((s, r)) => s == sender_ptr && r == receiver_ptr,
            None => false,
        }
    }
}

// ========================================================================
// Untyped Tests
// ========================================================================

pub fn test_untyped_creation() -> bool {
    match UntypedMemory::new(PhysAddr::new(0x50000000), 20) {
        Ok(untyped) => {
            untyped.size_bits() == 20
                && untyped.size() == 1024 * 1024
                && untyped.free_bytes() == 1024 * 1024
        }
        Err(_) => false,
    }
}

pub fn test_untyped_retype() -> bool {
    let mut untyped = match UntypedMemory::new(PhysAddr::new(0x50000000), 20) {
        Ok(u) => u,
        Err(_) => return false,
    };

    // Retype to TCB
    let tcb_addr = match untyped.retype(CapType::Tcb, 12) {
        Ok(addr) => addr,
        Err(_) => return false,
    };

    if tcb_addr != PhysAddr::new(0x50000000) { return false; }
    if untyped.num_children() != 1 { return false; }

    // Retype to Endpoint
    let ep_addr = match untyped.retype(CapType::Endpoint, 6) {
        Ok(addr) => addr,
        Err(_) => return false,
    };

    ep_addr.as_u64() > tcb_addr.as_u64() && untyped.num_children() == 2
}

pub fn test_untyped_revoke() -> bool {
    let mut untyped = match UntypedMemory::new(PhysAddr::new(0x50000000), 20) {
        Ok(u) => u,
        Err(_) => return false,
    };

    if untyped.retype(CapType::Tcb, 12).is_err() { return false; }
    if untyped.retype(CapType::Endpoint, 6).is_err() { return false; }
    if untyped.num_children() != 2 { return false; }

    unsafe {
        if untyped.revoke().is_err() { return false; }
    }

    untyped.num_children() == 0 && untyped.free_bytes() == 1024 * 1024
}

// ========================================================================
// Invocation Tests
// ========================================================================

pub fn test_tcb_invocation_priority() -> bool {
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
            TCB::CAP_ALL,
        );

        let cap = Capability::new(CapType::Tcb, &mut tcb as *mut _ as usize);
        let args = InvocationArgs {
            label: 3, // SetPriority
            args: &[150],
            cap_args: &[],
        };

        match invoke_capability(&cap, args) {
            Ok(_) => tcb.priority() == 150,
            Err(_) => false,
        }
    }
}

pub fn test_invocation_rights_enforcement() -> bool {
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
            TCB::CAP_ALL,
        );

        let cap = Capability::new(CapType::Tcb, &mut tcb as *mut _ as usize);
        let read_only = match cap.derive(CapRights::READ) {
            Ok(r) => r,
            Err(_) => return false,
        };

        let args = InvocationArgs {
            label: 3, // SetPriority (requires WRITE)
            args: &[150],
            cap_args: &[],
        };

        match invoke_capability(&read_only, args) {
            Err(InvocationError::InsufficientRights) => true,
            _ => false,
        }
    }
}

// ========================================================================
// Integration Tests
// ========================================================================

pub fn test_capability_delegation_chain() -> bool {
    let root = Capability::new(CapType::Endpoint, 0x1000);

    let rw_rights = CapRights::from_bits(0b0011); // READ | WRITE
    let child1 = match root.derive(rw_rights) {
        Ok(c) => c,
        Err(_) => return false,
    };

    if !child1.rights().contains(CapRights::READ) { return false; }
    if !child1.rights().contains(CapRights::WRITE) { return false; }
    if child1.rights().contains(CapRights::GRANT) { return false; }

    let child2 = match child1.derive(CapRights::READ) {
        Ok(c) => c,
        Err(_) => return false,
    };

    if !child2.rights().contains(CapRights::READ) { return false; }
    if child2.rights().contains(CapRights::WRITE) { return false; }

    // Cannot escalate privileges
    child2.derive(CapRights::WRITE).is_err()
}

pub fn test_endpoint_badged_capabilities() -> bool {
    let endpoint = Endpoint::new();
    let base_cap = Capability::new(CapType::Endpoint, &endpoint as *const _ as usize);

    let client1 = match base_cap.mint(0x1111) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let client2 = match base_cap.mint(0x2222) {
        Ok(c) => c,
        Err(_) => return false,
    };

    client1.guard() == 0x1111
        && client2.guard() == 0x2222
        && client1.object_ptr() == client2.object_ptr()
}
