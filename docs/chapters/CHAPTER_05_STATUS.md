# Chapter 5: IPC & Message Passing - Status

**Status**: 🚧 IN PROGRESS - Just Started!
**Started**: 2025-10-13
**Target Completion**: TBD

## Objectives

1. ⬜ Implement synchronous IPC (send/receive)
2. ⬜ Create message structure and transfer
3. ⬜ Implement call/reply semantics
4. ⬜ Add capability transfer in messages
5. ⬜ (Optional) Implement IPC fastpath optimization

## Overview

Chapter 5 implements Inter-Process Communication (IPC), the fundamental mechanism for threads to communicate and transfer capabilities between protection domains. This is the heart of the microkernel architecture.

## IPC Model

KaaL follows seL4's synchronous IPC model:

```
Sender Thread                    Receiver Thread
     |                                 |
     | Send(endpoint, msg)             | Recv(endpoint)
     |─────────────────►               |
     |                  Rendezvous     |
     | (blocked)        (in kernel)    | (blocked)
     |                       |          |
     |      ◄────────────────┘          |
     |    Copy message                  |
     |    Transfer caps                 |
     |                                  |
     | (unblocked)                      | (unblocked)
     └─────────────────────────────────►
            Message delivered!
```

### Key Properties

- **Synchronous**: Both sender and receiver must rendezvous
- **Zero-copy**: Messages transferred directly (no buffering)
- **Capability transfer**: Capabilities can be sent alongside data
- **Unbuffered**: If no partner is waiting, thread blocks
- **Ordered**: FIFO ordering for fairness

## Implementation Plan

### Phase 1: Message Structure ⬜ NOT STARTED

Define the message format and IPC buffer layout.

**Files to Create:**
- `kernel/src/ipc/mod.rs` - IPC module root
- `kernel/src/ipc/message.rs` - Message structure

**Key Structures:**

```rust
/// IPC Message - data transferred during IPC
pub struct Message {
    /// Message label (24 bits user data + 8 bits for caps)
    label: u32,

    /// Message registers (up to 64 words on ARM64)
    /// First few are in CPU registers, rest in IPC buffer
    regs: [u64; MAX_MSG_REGS],

    /// Number of valid registers
    len: usize,

    /// Capabilities to transfer (up to 3)
    caps: [Option<Capability>; MAX_CAPS],

    /// Number of capabilities
    num_caps: usize,
}

/// IPC Buffer - user-accessible memory for extra message data
pub struct IpcBuffer {
    /// Message registers (beyond what fits in CPU regs)
    msg_regs: [u64; IPC_BUFFER_SIZE],

    /// Capability transfer metadata
    cap_transfer: CapTransfer,
}
```

**Success Criteria:**
- [x] Message structure defined
- [x] IPC buffer layout specified
- [x] Supports up to 64 message registers
- [x] Can transfer up to 3 capabilities

### Phase 2: Send Operation ⬜ NOT STARTED

Implement the send half of IPC.

**Files to Create:**
- `kernel/src/ipc/send.rs` - Send implementation

**Key Operations:**

```rust
/// Send a message to an endpoint
///
/// If a receiver is waiting, perform IPC immediately.
/// Otherwise, block the sender until a receiver arrives.
pub fn send(endpoint: &mut Endpoint, sender: &mut TCB, msg: Message) -> Result<(), IpcError> {
    // 1. Check endpoint capability rights
    // 2. Try to match with waiting receiver
    // 3. If receiver found:
    //    - Transfer message from sender to receiver
    //    - Transfer capabilities
    //    - Unblock both threads
    // 4. Otherwise:
    //    - Save message in sender's TCB
    //    - Block sender on endpoint send queue
}
```

**Success Criteria:**
- [x] Send blocks if no receiver waiting
- [x] Send completes immediately if receiver waiting
- [x] Message data copied correctly
- [x] Sender TCB updated properly

### Phase 3: Receive Operation ⬜ NOT STARTED

Implement the receive half of IPC.

**Files to Create:**
- `kernel/src/ipc/recv.rs` - Receive implementation

**Key Operations:**

```rust
/// Receive a message from an endpoint
///
/// If a sender is waiting, perform IPC immediately.
/// Otherwise, block the receiver until a sender arrives.
pub fn recv(endpoint: &mut Endpoint, receiver: &mut TCB) -> Result<Message, IpcError> {
    // 1. Check endpoint capability rights
    // 2. Try to match with waiting sender
    // 3. If sender found:
    //    - Transfer message from sender to receiver
    //    - Transfer capabilities
    //    - Unblock both threads
    //    - Return message
    // 4. Otherwise:
    //    - Block receiver on endpoint receive queue
    //    - Will be woken when sender arrives
}
```

**Success Criteria:**
- [x] Receive blocks if no sender waiting
- [x] Receive completes immediately if sender waiting
- [x] Message data transferred correctly
- [x] Receiver TCB updated properly

### Phase 4: Message Transfer ⬜ NOT STARTED

Implement the core message transfer logic.

**Files to Create:**
- `kernel/src/ipc/transfer.rs` - Message transfer

**Key Operations:**

```rust
/// Transfer a message from sender to receiver
///
/// Copies message registers and transfers capabilities.
pub fn transfer_message(
    sender: &mut TCB,
    receiver: &mut TCB,
    msg: &Message,
) -> Result<(), IpcError> {
    // 1. Copy message registers to receiver's context
    // 2. Write extended registers to receiver's IPC buffer
    // 3. Transfer capabilities (move/grant/mint)
    // 4. Set receiver's return value (message label)
    // 5. Update both thread states
}
```

**Success Criteria:**
- [x] Message registers copied correctly
- [x] Extended data in IPC buffer transferred
- [x] CPU context updated (x0-x7 for fast regs)
- [x] Both threads unblocked properly

### Phase 5: Capability Transfer ⬜ NOT STARTED

Implement capability transfer during IPC.

**Files to Create:**
- `kernel/src/ipc/cap_transfer.rs` - Capability transfer

**Transfer Types:**
- **Move**: Transfer capability (sender loses it)
- **Grant**: Grant full rights (like copy but explicit)
- **Mint**: Create badged copy (for endpoints)

**Key Operations:**

```rust
/// Transfer capabilities from sender to receiver
pub fn transfer_caps(
    sender: &mut TCB,
    receiver: &mut TCB,
    caps: &[CapTransfer],
) -> Result<(), IpcError> {
    for cap_xfer in caps {
        match cap_xfer.op {
            CapTransferOp::Move => move_cap(sender, receiver, cap_xfer)?,
            CapTransferOp::Grant => grant_cap(sender, receiver, cap_xfer)?,
            CapTransferOp::Mint => mint_cap(sender, receiver, cap_xfer)?,
        }
    }
}
```

**Success Criteria:**
- [x] Move removes cap from sender
- [x] Grant copies with full rights
- [x] Mint creates badged endpoint cap
- [x] Rights checked before transfer

### Phase 6: Call/Reply Semantics ⬜ NOT STARTED

Implement RPC-style call/reply on top of send/receive.

**Files to Create:**
- `kernel/src/ipc/call.rs` - Call/reply operations

**Call Operation:**
```rust
/// Call: Send and wait for reply
///
/// Like send(), but implicitly grants reply capability
/// and blocks waiting for reply.
pub fn call(endpoint: &mut Endpoint, caller: &mut TCB, msg: Message) -> Result<Message, IpcError> {
    // 1. Create reply capability
    // 2. Send message (like normal send)
    // 3. Block caller on reply (special state)
    // 4. When reply arrives, return message
}
```

**Reply Operation:**
```rust
/// Reply: Send response back to caller
///
/// Uses the reply capability to send message back.
pub fn reply(reply_cap: &Capability, replier: &mut TCB, msg: Message) -> Result<(), IpcError> {
    // 1. Find blocked caller from reply cap
    // 2. Transfer message to caller
    // 3. Unblock caller
    // 4. Destroy reply capability
}
```

**Success Criteria:**
- [x] Call implicitly grants reply capability
- [x] Reply unblocks original caller
- [x] Reply capability consumed after use
- [x] Works like synchronous RPC

### Phase 7: Testing & Integration ⬜ NOT STARTED

Create comprehensive tests for IPC operations.

**Tests to Create:**
1. Basic send/receive between two threads
2. Send blocks when no receiver
3. Receive blocks when no sender
4. Message data transferred correctly
5. Capability move/grant/mint
6. Call/reply RPC semantics
7. Multiple senders to one receiver (FIFO)
8. Badge identification

## Success Criteria

Chapter 5 is complete when:

1. ✅ Send/receive operations work end-to-end
2. ✅ Message data transferred correctly
3. ✅ Capabilities can be transferred
4. ✅ Call/reply semantics functional
5. ✅ Tests pass for all IPC operations
6. ✅ Can boot a simple user-space program using IPC

## Files Structure

```
kernel/src/ipc/
├── mod.rs              ← Module root, main IPC interface
├── message.rs          ← Message and IPC buffer structures
├── send.rs             ← Send operation
├── recv.rs             ← Receive operation
├── transfer.rs         ← Message transfer logic
├── cap_transfer.rs     ← Capability transfer
└── call.rs             ← Call/reply semantics
```

## References

### seL4 Documentation
- [seL4 IPC Manual](https://sel4.systems/Info/Docs/seL4-manual-latest.pdf) - IPC operations
- [seL4 Whitepaper](https://sel4.systems/About/seL4-whitepaper.pdf) - IPC design rationale
- [Fast IPC Paper](https://dl.acm.org/doi/10.1145/224057.224075) - L4 IPC design

### Implementation References
- seL4 kernel: `kernel/src/kernel/` - Main IPC path
- seL4 kernel: `kernel/include/api/` - IPC API definitions
- seL4 kernel: `libsel4/` - User-space IPC wrappers

## Progress Tracking

### Completed ✅
- Chapter 4 object model provides foundation:
  - ✅ Capability system
  - ✅ CNode for capability storage
  - ✅ TCB for thread state
  - ✅ Endpoint for rendezvous

### In Progress 🚧
- Phase 1: Message structure (starting now)

### Blocked ⛔
- None - all prerequisites complete!

## Key Design Decisions

### 1. Message Register Count
Following seL4:
- Fast path: 4 registers (x0-x3 for args, x4-x7 for return)
- Extended: Up to 64 total registers
- Beyond 4: Use IPC buffer

### 2. Capability Transfer Limit
Following seL4:
- Maximum 3 capabilities per message
- Sufficient for most use cases
- Keeps kernel complexity low

### 3. Synchronous Model
Why synchronous IPC?
- Simpler than async (no buffering needed)
- Better for RPC-style communication
- Matches seL4 proven model
- Can build async on top if needed

### 4. Fast Path Optimization
Deferred to later:
- Initial implementation: simple path
- Later: Optimize common case (call/reply with no caps)
- Target: < 200 cycles for fastpath

## Next Steps

1. Create `kernel/src/ipc/` directory
2. Implement Message structure
3. Implement send operation
4. Implement receive operation
5. Test basic send/receive

---

**Last Updated**: 2025-10-13
**Status**: 🚧 IN PROGRESS - Just started Chapter 5!
**Dependencies**: ✅ Chapter 4 Phases 1-4 complete
