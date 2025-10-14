//! IPC Test Runner
//!
//! Comprehensive tests for Inter-Process Communication (IPC) operations.
//! Each test returns bool (true = pass, false = fail).

use super::*;
use crate::objects::{Capability, CapType, CapRights, TCB, Endpoint, CNode, ThreadState};
use crate::memory::{PhysAddr, VirtAddr};

// ========================================================================
// Message Tests
// ========================================================================

pub fn test_message_creation() -> bool {
    let msg = Message::with_label(0x1234);
    msg.label() == 0x1234 && msg.len() == 0
}

pub fn test_message_set_reg() -> bool {
    let mut msg = Message::with_label(0x1234);
    if msg.set_reg(0, 0xDEADBEEF).is_err() { return false; }
    if msg.set_reg(1, 0xCAFEBABE).is_err() { return false; }

    msg.get_reg(0) == Some(0xDEADBEEF) && msg.get_reg(1) == Some(0xCAFEBABE) && msg.len() == 2
}

pub fn test_message_fast_path() -> bool {
    let mut msg = Message::new();
    // Fast path: up to 4 registers
    for i in 0..4 {
        if msg.set_reg(i, i as u64 * 100).is_err() { return false; }
    }
    msg.is_fast_path()
}

pub fn test_message_slow_path() -> bool {
    let mut msg = Message::new();
    // Slow path: more than FAST_PATH_REGS (4) registers
    for i in 0..9 {
        if msg.set_reg(i, i as u64 * 100).is_err() { return false; }
    }
    !msg.is_fast_path()
}

// ========================================================================
// Basic Send/Receive Tests
// ========================================================================

pub fn test_send_blocks_no_receiver() -> bool {
    unsafe {
        // Create endpoint
        let ep_mem = alloc::boxed::Box::leak(alloc::boxed::Box::new(Endpoint::new()));
        let endpoint = ep_mem as *mut Endpoint;

        // Create sender TCB
        let sender_tcb = create_test_tcb(1);

        // Create capability
        let ep_cap = Capability::new(CapType::Endpoint, endpoint as usize);

        // Create message
        let msg = Message::with_label(0x1234);

        // Sender sends (no receiver waiting)
        if operations::send(&ep_cap, sender_tcb, msg).is_err() {
            return false;
        }

        // Sender should be blocked
        matches!((*sender_tcb).state(), ThreadState::BlockedOnSend { .. })
    }
}

pub fn test_recv_blocks_no_sender() -> bool {
    unsafe {
        // Create endpoint
        let ep_mem = alloc::boxed::Box::leak(alloc::boxed::Box::new(Endpoint::new()));
        let endpoint = ep_mem as *mut Endpoint;

        // Create receiver TCB
        let receiver_tcb = create_test_tcb(1);

        // Create capability
        let ep_cap = Capability::new(CapType::Endpoint, endpoint as usize);

        // Receiver receives (no sender waiting)
        if operations::recv(&ep_cap, receiver_tcb).is_ok() {
            // Receiver should be blocked
            matches!((*receiver_tcb).state(), ThreadState::BlockedOnReceive { .. })
        } else {
            false
        }
    }
}

pub fn test_message_data_transfer() -> bool {
    unsafe {
        // Create endpoint
        let ep_mem = alloc::boxed::Box::leak(alloc::boxed::Box::new(Endpoint::new()));
        let endpoint = ep_mem as *mut Endpoint;

        // Create sender and receiver TCBs
        let sender_tcb = create_test_tcb(1);
        let receiver_tcb = create_test_tcb(2);

        // Create capabilities
        let ep_cap = Capability::new(CapType::Endpoint, endpoint as usize);

        // Create message with multiple registers
        let mut msg = Message::with_label(0xABCD);
        let _ = msg.set_reg(0, 0x1111_1111);
        let _ = msg.set_reg(1, 0x2222_2222);
        let _ = msg.set_reg(2, 0x3333_3333);

        // Receiver waits first
        let _ = operations::recv(&ep_cap, receiver_tcb);

        // Sender sends
        if operations::send(&ep_cap, sender_tcb, msg).is_err() {
            return false;
        }

        // Verify data in receiver's context (trap frame x0-x2)
        let context = (*receiver_tcb).context();
        context.x0 == 0x1111_1111
            && context.x1 == 0x2222_2222
            && context.x2 == 0x3333_3333
    }
}

// ========================================================================
// Capability Transfer Tests
// ========================================================================

pub fn test_cap_grant_simple() -> bool {
    unsafe {
        // Create sender and receiver CSpaces (must be mutable and remain in scope)
        let mut sender_caps = [Capability::null(); 16];
        let mut receiver_caps = [Capability::null(); 16];

        let mut sender_cspace = CNode::new(4, PhysAddr::new(&sender_caps[0] as *const _ as usize)).unwrap();
        let mut receiver_cspace = CNode::new(4, PhysAddr::new(&receiver_caps[0] as *const _ as usize)).unwrap();

        // Create a capability to transfer
        let test_cap = Capability::new(CapType::Endpoint, 0x5000);
        let _ = sender_cspace.insert(0, test_cap);

        // Grant capability (move from sender to receiver)
        let sender_ptr = &mut sender_cspace as *mut CNode;
        let receiver_ptr = &mut receiver_cspace as *mut CNode;

        let result = cap_transfer::grant_capability(
            sender_ptr,
            receiver_ptr,
            0,  // src_slot
            5,  // dst_slot
        );

        if result.is_err() { return false; }

        // Verify: sender slot should be empty, receiver should have cap
        let sender_cap = sender_cspace.lookup(0);
        let receiver_cap = receiver_cspace.lookup(5);

        sender_cap.is_none()
            && receiver_cap.is_some()
            && receiver_cap.unwrap().object_ptr() == 0x5000
    }
}

pub fn test_cap_mint_with_badge() -> bool {
    unsafe {
        // Create sender and receiver CSpaces
        let mut sender_caps = [Capability::null(); 16];
        let mut receiver_caps = [Capability::null(); 16];

        let mut sender_cspace = CNode::new(4, PhysAddr::new(&sender_caps[0] as *const _ as usize)).unwrap();
        let mut receiver_cspace = CNode::new(4, PhysAddr::new(&receiver_caps[0] as *const _ as usize)).unwrap();

        // Create endpoint capability
        let ep_cap = Capability::new(CapType::Endpoint, 0x6000);
        let _ = sender_cspace.insert(0, ep_cap);

        // Mint capability with badge (copy with badge)
        let sender_ptr = &mut sender_cspace as *mut CNode;
        let receiver_ptr = &mut receiver_cspace as *mut CNode;
        let badge = 0xBEEF;

        let result = cap_transfer::mint_capability(
            sender_ptr,
            receiver_ptr,
            0,  // src_slot
            3,  // dst_slot
            badge,
        );

        if result.is_err() { return false; }

        // Verify: both sender and receiver should have cap, receiver's is badged
        let sender_cap = sender_cspace.lookup(0);
        let receiver_cap = receiver_cspace.lookup(3);

        sender_cap.is_some()
            && receiver_cap.is_some()
            && receiver_cap.unwrap().guard() == badge
            && receiver_cap.unwrap().object_ptr() == 0x6000
    }
}

pub fn test_cap_derive_reduced_rights() -> bool {
    unsafe {
        // Create sender and receiver CSpaces
        let mut sender_caps = [Capability::null(); 16];
        let mut receiver_caps = [Capability::null(); 16];

        let mut sender_cspace = CNode::new(4, PhysAddr::new(&sender_caps[0] as *const _ as usize)).unwrap();
        let mut receiver_cspace = CNode::new(4, PhysAddr::new(&receiver_caps[0] as *const _ as usize)).unwrap();

        // Create capability with ALL rights
        let cap = Capability::new(CapType::Endpoint, 0x7000);
        let _ = sender_cspace.insert(0, cap);

        // Derive capability with only READ rights (copy with reduced rights)
        let sender_ptr = &mut sender_cspace as *mut CNode;
        let receiver_ptr = &mut receiver_cspace as *mut CNode;

        let result = cap_transfer::derive_capability(
            sender_ptr,
            receiver_ptr,
            0,  // src_slot
            2,  // dst_slot
            CapRights::READ,
        );

        if result.is_err() { return false; }

        // Verify: receiver has reduced rights
        let receiver_cap = receiver_cspace.lookup(2);

        receiver_cap.is_some()
            && receiver_cap.unwrap().rights() == CapRights::READ
            && receiver_cap.unwrap().object_ptr() == 0x7000
    }
}

// ========================================================================
// Call/Reply Tests
// ========================================================================

pub fn test_call_creates_reply_cap() -> bool {
    unsafe {
        // Create endpoint
        let ep_mem = alloc::boxed::Box::leak(alloc::boxed::Box::new(Endpoint::new()));
        let endpoint = ep_mem as *mut Endpoint;

        // Create caller TCB
        let caller_tcb = create_test_tcb(1);

        // Create endpoint capability
        let ep_cap = Capability::new(CapType::Endpoint, endpoint as usize);

        // Create message
        let msg = Message::with_label(0x9999);

        // Call (will block waiting for server)
        if call::call(&ep_cap, caller_tcb, msg).is_err() {
            return false;
        }

        // Verify caller is in BlockedOnReply state
        matches!((*caller_tcb).state(), ThreadState::BlockedOnReply)
    }
}

pub fn test_reply_wakes_caller() -> bool {
    unsafe {
        // Create endpoint
        let ep_mem = alloc::boxed::Box::leak(alloc::boxed::Box::new(Endpoint::new()));
        let endpoint = ep_mem as *mut Endpoint;

        // Create caller and server TCBs
        let caller_tcb = create_test_tcb(1);
        let server_tcb = create_test_tcb(2);

        // Create capabilities
        let ep_cap = Capability::new(CapType::Endpoint, endpoint as usize);

        // Caller calls (blocks)
        let call_msg = Message::with_label(0xAAAA);
        let _ = call::call(&ep_cap, caller_tcb, call_msg);

        // Verify caller blocked
        if !matches!((*caller_tcb).state(), ThreadState::BlockedOnReply) {
            return false;
        }

        // Create reply capability pointing to caller
        let reply_cap = call::create_reply_capability(caller_tcb);

        // Server replies
        let reply_msg = Message::with_label(0xBBBB);
        if call::reply(&reply_cap, server_tcb, &reply_msg).is_err() {
            return false;
        }

        // Verify caller is now runnable
        (*caller_tcb).state() == ThreadState::Runnable
    }
}

// ========================================================================
// FIFO Ordering Tests
// ========================================================================

pub fn test_multiple_senders_fifo() -> bool {
    unsafe {
        // Create endpoint
        let ep_mem = alloc::boxed::Box::leak(alloc::boxed::Box::new(Endpoint::new()));
        let endpoint = ep_mem as *mut Endpoint;

        // Create 3 sender TCBs
        let sender1 = create_test_tcb(1);
        let sender2 = create_test_tcb(2);
        let sender3 = create_test_tcb(3);

        // Create endpoint capability
        let ep_cap = Capability::new(CapType::Endpoint, endpoint as usize);

        // All senders send (will block and queue up)
        let _ = operations::send(&ep_cap, sender1, Message::with_label(0x1111));
        let _ = operations::send(&ep_cap, sender2, Message::with_label(0x2222));
        let _ = operations::send(&ep_cap, sender3, Message::with_label(0x3333));

        // Verify they're queued in FIFO order
        let first = (*endpoint).dequeue_sender();
        let second = (*endpoint).dequeue_sender();
        let third = (*endpoint).dequeue_sender();

        first == Some(sender1) && second == Some(sender2) && third == Some(sender3)
    }
}

// ========================================================================
// Helper Functions
// ========================================================================

/// Create a minimal test TCB with default values
unsafe fn create_test_tcb(tid: u64) -> *mut TCB {
    // Create a minimal CSpace for the TCB (MIN_SIZE_BITS = 4, so 16 slots)
    let mut caps = alloc::boxed::Box::leak(alloc::boxed::Box::new([Capability::null(); 16]));
    let cspace = alloc::boxed::Box::leak(alloc::boxed::Box::new(
        CNode::new(4, PhysAddr::new(&caps[0] as *const _ as usize)).unwrap()
    ));

    // Create TCB with minimal configuration
    let tcb_mem = alloc::boxed::Box::leak(alloc::boxed::Box::new(
        TCB::new(
            tid as usize,                  // tid
            cspace as *mut CNode,          // cspace_root
            0x0,                           // vspace_root (not used in tests)
            VirtAddr::new(0x0),            // ipc_buffer (not used in tests)
            0x0,                           // entry_point (not used)
            0x1000,                        // stack_pointer (dummy)
        )
    ));

    let tcb = tcb_mem as *mut TCB;
    (*tcb).set_state(ThreadState::Runnable);
    tcb
}

// ========================================================================
// Test Registration
// ========================================================================

pub fn run_all_ipc_tests() -> (usize, usize) {
    let tests: &[(&str, fn() -> bool)] = &[
        // Message tests
        ("message_creation", test_message_creation),
        ("message_set_reg", test_message_set_reg),
        ("message_fast_path", test_message_fast_path),
        ("message_slow_path", test_message_slow_path),

        // Send/Receive tests
        ("send_blocks_no_receiver", test_send_blocks_no_receiver),
        ("recv_blocks_no_sender", test_recv_blocks_no_sender),
        ("message_data_transfer", test_message_data_transfer),

        // Capability transfer tests
        ("cap_grant_simple", test_cap_grant_simple),
        ("cap_mint_with_badge", test_cap_mint_with_badge),
        ("cap_derive_reduced_rights", test_cap_derive_reduced_rights),

        // Call/Reply tests
        ("call_creates_reply_cap", test_call_creates_reply_cap),
        ("reply_wakes_caller", test_reply_wakes_caller),

        // FIFO tests
        ("multiple_senders_fifo", test_multiple_senders_fifo),
    ];

    let mut passed = 0;
    let mut failed = 0;

    crate::kprintln!("\n=== Running IPC Tests ===");

    for (name, test_fn) in tests {
        let result = test_fn();
        if result {
            crate::kprintln!("  [PASS] {}", name);
            passed += 1;
        } else {
            crate::kprintln!("  [FAIL] {}", name);
            failed += 1;
        }
    }

    crate::kprintln!("=== IPC Tests Complete: {}/{} passed ===\n", passed, passed + failed);

    (passed, failed)
}
