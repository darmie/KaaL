# Centralized IPC Implementation Plan

## Current State

**Working**: Decentralized self-service IPC
- Components call `sdk::channel_setup::establish_channel()`
- Producer: allocates memory → registers via SYS_SHMEM_REGISTER
- Consumer: queries via SYS_SHMEM_QUERY → maps same physical memory
- ChannelBroker provides shared memory registry only

**Problem**: Integration gap - ChannelBroker.establish_channel() is placeholder

## Goal

Implement centralized orchestration where ChannelBroker manages full channel setup.

## Architecture

```
Component A                    Component B
     |                              |
     | "establish channel"          |
     v                              |
ChannelBroker                       |
(in root-task)                      |
     |                              |
     |-- Allocate shared memory     |
     |-- Map into A's address space |
     |-- Map into B's address space-|
     |-- Create notifications        |
     |-- Transfer caps to A          |
     |-- Transfer caps to B ---------|
     |                              |
     v                              v
  Channel established
```

## Implementation Steps

### 1. Broker Interface (runtime/ipc/src/broker.rs)

```rust
pub struct ChannelSetupCallbacks {
    pub memory_allocate: fn(usize) -> Result<usize, ()>,
    pub memory_map_into: fn(tcb_cap: usize, phys: usize, size: usize,
                            virt: usize, perms: usize) -> Result<(), ()>,
    pub notification_create: fn() -> Result<usize, ()>,
    pub cap_insert_into: fn(tcb_cap: usize, slot: usize,
                            cap_type: usize, obj: usize) -> Result<(), ()>,
}

impl ChannelBroker {
    pub fn establish_channel_centralized(
        &mut self,
        producer_tcb_cap: usize,
        consumer_tcb_cap: usize,
        buffer_size: usize,
        callbacks: &ChannelSetupCallbacks,
    ) -> Result<ChannelId, BrokerError> {
        // 1. Allocate shared memory
        let phys_addr = (callbacks.memory_allocate)(buffer_size)?;

        // 2. Choose virtual addresses (0x90000000 for both)
        let virt_addr = 0x90000000;

        // 3. Map into producer
        (callbacks.memory_map_into)(producer_tcb_cap, phys_addr,
                                    buffer_size, virt_addr, PERMS_RW)?;

        // 4. Map into consumer
        (callbacks.memory_map_into)(consumer_tcb_cap, phys_addr,
                                    buffer_size, virt_addr, PERMS_RW)?;

        // 5. Create notification
        let notify_cap = (callbacks.notification_create)()?;

        // 6. Transfer to producer
        (callbacks.cap_insert_into)(producer_tcb_cap, SLOT_NOTIFY,
                                    CAP_NOTIFICATION, notify_cap)?;

        // 7. Transfer to consumer
        (callbacks.cap_insert_into)(consumer_tcb_cap, SLOT_NOTIFY,
                                    CAP_NOTIFICATION, notify_cap)?;

        // 8. Create channel record
        let channel_id = self.next_channel_id.fetch_add(1, Ordering::SeqCst);
        self.channels.insert(channel_id, Channel {
            id: channel_id,
            producer_id,
            consumer_id,
            state: ChannelState::Active,
            shared_memory_phys: phys_addr,
            shared_memory_size: buffer_size,
            producer_vaddr: virt_addr,
            consumer_vaddr: virt_addr,
            producer_notify: notify_cap,
            consumer_notify: notify_cap,
        });

        Ok(channel_id)
    }
}
```

### 2. Root-Task Integration (runtime/root-task/src/main.rs)

```rust
// After spawning producer and consumer
let callbacks = ChannelSetupCallbacks {
    memory_allocate: |size| unsafe {
        let addr = sys_memory_allocate(size);
        if addr == usize::MAX { Err(()) } else { Ok(addr) }
    },
    memory_map_into: |tcb, phys, size, virt, perms| unsafe {
        let result = sys_memory_map_into(tcb, phys, size, virt, perms);
        if result == 0 { Ok(()) } else { Err(()) }
    },
    notification_create: || unsafe {
        let cap = sys_notification_create();
        if cap == usize::MAX { Err(()) } else { Ok(cap) }
    },
    cap_insert_into: |tcb, slot, cap_type, obj| unsafe {
        let result = sys_cap_insert_into(tcb, slot, cap_type, obj);
        if result == 0 { Ok(()) } else { Err(()) }
    },
};

let broker = kaal_ipc::broker::get_broker_mut().unwrap();
let channel_id = broker.establish_channel_centralized(
    producer.tcb_cap_slot,
    consumer.tcb_cap_slot,
    0x1000, // 4KB buffer
    &callbacks,
)?;
```

### 3. Component Simplification

Components no longer call `establish_channel()`. They receive channel info
from broker and just use it:

```rust
// Producer init
pub fn init(config: ChannelConfig) -> Self {
    // config.buffer_addr already mapped by broker
    // config.notification_cap already installed
    IpcProducer { config }
}
```

## Benefits

1. **Closes integration gap**: Broker actually manages channels
2. **Centralized policy**: Broker controls who can communicate
3. **Simpler components**: Don't manage their own setup
4. **Audit trail**: All channels logged by broker
5. **Security**: Components can't bypass broker

## Timeline

1. Implement `ChannelSetupCallbacks` and `establish_channel_centralized()`
2. Update root-task to call broker after spawning components
3. Simplify producer/consumer to receive config instead of self-setup
4. Test end-to-end
5. Commit

## Compatibility

Keep both patterns:
- `establish_channel_centralized()` - new centralized approach
- Existing self-service via SDK still works for flexible use cases

This gives users choice based on their security/flexibility tradeoffs.
