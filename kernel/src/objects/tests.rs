//! Object Model Integration Tests
//!
//! This module contains comprehensive tests for the entire object model,
//! testing interactions between different object types and ensuring the
//! capability-based security model works correctly.
//!
//! ## Test Categories
//!
//! 1. **Capability Tests**: Creation, derivation, minting
//! 2. **CNode Tests**: Capability storage and lookup
//! 3. **TCB Tests**: Thread state management
//! 4. **Endpoint Tests**: IPC queue management
//! 5. **Untyped Tests**: Memory retyping and revocation
//! 6. **Invocation Tests**: Syscall dispatch and rights enforcement
//! 7. **Integration Tests**: Multi-object scenarios

#[cfg(test)]
mod tests {
    use crate::objects::*;
    use crate::memory::{PhysAddr, VirtAddr};
    use alloc::vec::Vec;

    // ========================================================================
    // Capability Tests
    // ========================================================================

    #[test]
    fn test_capability_creation() {
        let cap = Capability::new(CapType::Endpoint, 0x1000);
        assert_eq!(cap.cap_type(), CapType::Endpoint);
        assert_eq!(cap.object_ptr(), 0x1000);
        assert_eq!(cap.rights(), CapRights::ALL);
    }

    #[test]
    fn test_capability_null() {
        let cap = Capability::null();
        assert_eq!(cap.cap_type(), CapType::Null);
        assert_eq!(cap.object_ptr(), 0);
    }

    #[test]
    fn test_capability_derivation() {
        let cap = Capability::new(CapType::Endpoint, 0x1000);
        let derived = cap.derive(CapRights::READ).unwrap();

        assert_eq!(derived.cap_type(), CapType::Endpoint);
        assert_eq!(derived.object_ptr(), 0x1000);
        assert_eq!(derived.rights(), CapRights::READ);

        // Cannot derive with more rights
        assert!(derived.derive(CapRights::ALL).is_err());
    }

    #[test]
    fn test_capability_minting() {
        let cap = Capability::new(CapType::Endpoint, 0x1000);
        let minted = cap.mint(0x1234).unwrap();

        assert_eq!(minted.cap_type(), CapType::Endpoint);
        assert_eq!(minted.guard(), 0x1234);
    }

    #[test]
    fn test_capability_rights_operations() {
        let r = CapRights::READ;
        let w = CapRights::WRITE;
        let rw = r | w;

        assert!(rw.contains(CapRights::READ));
        assert!(rw.contains(CapRights::WRITE));
        assert!(!rw.contains(CapRights::GRANT));

        assert_eq!((rw & CapRights::READ).bits(), CapRights::READ.bits());
    }

    // ========================================================================
    // CNode Tests
    // ========================================================================

    #[test]
    fn test_cnode_creation() {
        let mut memory = [Capability::null(); 16];
        let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

        unsafe {
            let cnode = CNode::new(4, paddr).unwrap(); // 2^4 = 16 slots
            assert_eq!(cnode.num_slots(), 16);
            assert_eq!(cnode.size_bits(), 4);
        }
    }

    #[test]
    fn test_cnode_insert_lookup() {
        let mut memory = [Capability::null(); 16];
        let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

        unsafe {
            let mut cnode = CNode::new(4, paddr).unwrap();
            let cap = Capability::new(CapType::Endpoint, 0x1000);

            cnode.insert(5, cap).unwrap();

            let retrieved = cnode.lookup(5).unwrap();
            assert_eq!(retrieved.cap_type(), CapType::Endpoint);
            assert_eq!(retrieved.object_ptr(), 0x1000);
        }
    }

    #[test]
    fn test_cnode_copy_move() {
        let mut memory = [Capability::null(); 16];
        let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

        unsafe {
            let mut cnode = CNode::new(4, paddr).unwrap();
            let cap = Capability::new(CapType::Endpoint, 0x1000);

            cnode.insert(0, cap).unwrap();

            // Copy
            cnode.copy_cap(0, 1).unwrap();
            assert!(cnode.lookup(0).is_some());
            assert!(cnode.lookup(1).is_some());

            // Move
            cnode.move_cap(1, 2).unwrap();
            assert!(cnode.lookup(1).is_none());
            assert!(cnode.lookup(2).is_some());
        }
    }

    #[test]
    fn test_cnode_delete() {
        let mut memory = [Capability::null(); 16];
        let paddr = PhysAddr::new(&memory[0] as *const _ as usize);

        unsafe {
            let mut cnode = CNode::new(4, paddr).unwrap();
            let cap = Capability::new(CapType::Endpoint, 0x1000);

            cnode.insert(0, cap).unwrap();
            assert!(cnode.lookup(0).is_some());

            cnode.delete(0).unwrap();
            assert!(cnode.lookup(0).is_none());
        }
    }

    // ========================================================================
    // TCB Tests
    // ========================================================================

    #[test]
    fn test_tcb_creation() {
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
            );

            assert_eq!(tcb.tid(), 1);
            assert_eq!(tcb.state(), ThreadState::Inactive);
            assert_eq!(tcb.priority(), TCB::DEFAULT_PRIORITY);
        }
    }

    #[test]
    fn test_tcb_state_transitions() {
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

            // Inactive -> Runnable
            tcb.activate();
            assert_eq!(tcb.state(), ThreadState::Runnable);

            // Runnable -> Blocked
            tcb.block_on_receive(0x5000);
            assert_eq!(tcb.state(), ThreadState::BlockedOnReceive { endpoint: 0x5000 });

            // Blocked -> Runnable
            tcb.unblock();
            assert_eq!(tcb.state(), ThreadState::Runnable);
        }
    }

    #[test]
    fn test_tcb_priority() {
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

            assert_eq!(tcb.priority(), 127);

            tcb.set_priority(200);
            assert_eq!(tcb.priority(), 200);
        }
    }

    // ========================================================================
    // Endpoint Tests
    // ========================================================================

    #[test]
    fn test_endpoint_creation() {
        let ep = Endpoint::new();
        assert_eq!(ep.badge(), 0);
        assert!(!ep.has_senders());
        assert!(!ep.has_receivers());
        assert!(ep.is_idle());
    }

    #[test]
    fn test_endpoint_with_badge() {
        let ep = Endpoint::with_badge(0x1234);
        assert_eq!(ep.badge(), 0x1234);
    }

    #[test]
    fn test_endpoint_queue_operations() {
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
            );

            let mut receiver = TCB::new(
                2,
                cnode_ptr,
                0x40000000,
                VirtAddr::new(0x10000000),
                0x200000,
                0x300000,
            );

            let sender_ptr = &mut sender as *mut TCB;
            let receiver_ptr = &mut receiver as *mut TCB;

            ep.queue_send(sender_ptr);
            assert!(ep.has_senders());
            assert_eq!(ep.send_queue_len(), 1);

            ep.queue_receive(receiver_ptr);
            assert!(ep.has_receivers());
            assert_eq!(ep.recv_queue_len(), 1);

            // Try match
            let matched = ep.try_match();
            assert!(matched.is_some());
            let (s, r) = matched.unwrap();
            assert_eq!(s, sender_ptr);
            assert_eq!(r, receiver_ptr);
        }
    }

    // ========================================================================
    // Untyped Tests
    // ========================================================================

    #[test]
    fn test_untyped_creation() {
        let untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20).unwrap();
        assert_eq!(untyped.size_bits(), 20);
        assert_eq!(untyped.size(), 1024 * 1024);
        assert_eq!(untyped.free_bytes(), 1024 * 1024);
    }

    #[test]
    fn test_untyped_retype() {
        let mut untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20).unwrap();

        // Retype to TCB
        let tcb_addr = untyped.retype(CapType::Tcb, 12).unwrap();
        assert_eq!(tcb_addr, PhysAddr::new(0x50000000));
        assert_eq!(untyped.num_children(), 1);

        // Retype to Endpoint
        let ep_addr = untyped.retype(CapType::Endpoint, 6).unwrap();
        assert!(ep_addr.as_u64() > tcb_addr.as_u64());
        assert_eq!(untyped.num_children(), 2);
    }

    #[test]
    fn test_untyped_revoke() {
        let mut untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20).unwrap();

        untyped.retype(CapType::Tcb, 12).unwrap();
        untyped.retype(CapType::Endpoint, 6).unwrap();
        assert_eq!(untyped.num_children(), 2);

        unsafe {
            untyped.revoke().unwrap();
        }

        assert_eq!(untyped.num_children(), 0);
        assert_eq!(untyped.free_bytes(), 1024 * 1024);
    }

    // ========================================================================
    // Invocation Tests
    // ========================================================================

    #[test]
    fn test_tcb_invocation_set_priority() {
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
                args: &[150],
                cap_args: &[],
            };

            let result = invoke_capability(&cap, args);
            assert!(result.is_ok());
            assert_eq!(tcb.priority(), 150);
        }
    }

    #[test]
    fn test_tcb_invocation_suspend_resume() {
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

            tcb.activate();
            assert_eq!(tcb.state(), ThreadState::Runnable);

            let cap = Capability::new(CapType::Tcb, &mut tcb as *mut _ as usize);

            // Suspend
            let suspend_args = InvocationArgs {
                label: 9,
                args: &[],
                cap_args: &[],
            };
            invoke_capability(&cap, suspend_args).unwrap();
            assert_eq!(tcb.state(), ThreadState::Inactive);

            // Resume
            let resume_args = InvocationArgs {
                label: 10,
                args: &[],
                cap_args: &[],
            };
            invoke_capability(&cap, resume_args).unwrap();
            assert_eq!(tcb.state(), ThreadState::Runnable);
        }
    }

    #[test]
    fn test_invocation_rights_enforcement() {
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

            // Create READ-only capability
            let cap = Capability::new(CapType::Tcb, &mut tcb as *mut _ as usize);
            let read_only = cap.derive(CapRights::READ).unwrap();

            // Try to set priority (requires WRITE)
            let args = InvocationArgs {
                label: 3,
                args: &[150],
                cap_args: &[],
            };

            let result = invoke_capability(&read_only, args);
            assert_eq!(result, Err(InvocationError::InsufficientRights));
        }
    }

    #[test]
    fn test_untyped_invocation_retype() {
        unsafe {
            let mut untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20).unwrap();
            let cap = Capability::new(CapType::UntypedMemory, &mut untyped as *mut _ as usize);

            let args = InvocationArgs {
                label: 0, // Retype
                args: &[4, 12], // TCB (4), 4KB (12)
                cap_args: &[],
            };

            let result = invoke_capability(&cap, args);
            assert!(result.is_ok());
            assert_eq!(untyped.num_children(), 1);
        }
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[test]
    fn test_capability_delegation_chain() {
        // Test: Root -> Child1 -> Child2 capability derivation chain
        let root = Capability::new(CapType::Endpoint, 0x1000);

        let child1 = root.derive(CapRights::READ | CapRights::WRITE).unwrap();
        assert!(child1.rights().contains(CapRights::READ));
        assert!(child1.rights().contains(CapRights::WRITE));
        assert!(!child1.rights().contains(CapRights::GRANT));

        let child2 = child1.derive(CapRights::READ).unwrap();
        assert!(child2.rights().contains(CapRights::READ));
        assert!(!child2.rights().contains(CapRights::WRITE));

        // Cannot escalate privileges
        assert!(child2.derive(CapRights::WRITE).is_err());
    }

    #[test]
    fn test_cnode_capability_management() {
        // Test: Store and manage multiple capabilities in CNode
        unsafe {
            let mut memory = [Capability::null(); 16];
            let paddr = PhysAddr::new(&memory[0] as *const _ as usize);
            let mut cnode = CNode::new(4, paddr).unwrap();

            // Insert various capability types
            let tcb_cap = Capability::new(CapType::Tcb, 0x1000);
            let ep_cap = Capability::new(CapType::Endpoint, 0x2000);
            let untyped_cap = Capability::new(CapType::UntypedMemory, 0x3000);

            cnode.insert(0, tcb_cap).unwrap();
            cnode.insert(1, ep_cap).unwrap();
            cnode.insert(2, untyped_cap).unwrap();

            // Verify all stored correctly
            assert_eq!(cnode.lookup(0).unwrap().cap_type(), CapType::Tcb);
            assert_eq!(cnode.lookup(1).unwrap().cap_type(), CapType::Endpoint);
            assert_eq!(cnode.lookup(2).unwrap().cap_type(), CapType::UntypedMemory);

            // Test iteration
            let mut count = 0;
            for (idx, cap) in cnode.iter() {
                if !cap.is_null() {
                    count += 1;
                    assert!(idx < 3);
                }
            }
            assert_eq!(count, 3);
        }
    }

    #[test]
    fn test_untyped_to_tcb_workflow() {
        // Test: Complete workflow of creating TCB from untyped memory
        unsafe {
            let mut untyped = UntypedMemory::new(PhysAddr::new(0x50000000), 20).unwrap();

            // Retype to TCB
            let tcb_paddr = untyped.retype(CapType::Tcb, 12).unwrap();
            assert_eq!(untyped.num_children(), 1);

            // Create capability for the new TCB
            let tcb_cap = Capability::new(CapType::Tcb, tcb_paddr.as_usize());
            assert_eq!(tcb_cap.cap_type(), CapType::Tcb);
            assert_eq!(tcb_cap.object_ptr(), tcb_paddr.as_usize());

            // Derive read-only capability
            let read_only = tcb_cap.derive(CapRights::READ).unwrap();
            assert_eq!(read_only.rights(), CapRights::READ);
        }
    }

    #[test]
    fn test_endpoint_with_badged_capabilities() {
        // Test: Multiple clients identified by badges
        let endpoint = Endpoint::new();
        let base_cap = Capability::new(CapType::Endpoint, &endpoint as *const _ as usize);

        // Mint badged capabilities for different clients
        let client1 = base_cap.mint(0x1111).unwrap();
        let client2 = base_cap.mint(0x2222).unwrap();
        let client3 = base_cap.mint(0x3333).unwrap();

        assert_eq!(client1.guard(), 0x1111);
        assert_eq!(client2.guard(), 0x2222);
        assert_eq!(client3.guard(), 0x3333);

        // All point to same endpoint
        assert_eq!(client1.object_ptr(), client2.object_ptr());
        assert_eq!(client2.object_ptr(), client3.object_ptr());
    }
}
