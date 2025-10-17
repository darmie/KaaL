# IPC Component Spawning Session Summary

## Date
2025-10-17

## Objective
Implement inter-component IPC by spawning ipc-producer and ipc-consumer components, then setting up shared memory channels for communication.

## Progress Summary

### ✅ Successfully Completed

1. **Reverted to clean state**
   - Reset to commit b7defb5 (main) which has working component spawning
   - Avoided buggy TTBR0-switching code from previous attempts
   - System now stable with system_init component working perfectly

2. **Fixed build system to compile ALL components**
   - Modified [build-system/builders/mod.nu](../build-system/builders/mod.nu) line 107-115
   - Changed from building only `autostart==true` components to building ALL components
   - This enables on-demand spawning of any component

3. **Fixed component workspace issues**
   - Added `[workspace]` table to [components/ipc-producer/Cargo.toml](../components/ipc-producer/Cargo.toml)
   - Added `[workspace]` table to [components/ipc-consumer/Cargo.toml](../components/ipc-consumer/Cargo.toml)
   - This prevents them from being part of parent workspace, allowing independent builds

4. **Updated root-task to spawn IPC components**
   - Modified [runtime/root-task/src/main.rs](../runtime/root-task/src/main.rs) Phase 5 section
   - Spawns ipc_producer and ipc_consumer using component loader
   - Both components spawn successfully and are scheduled

5. **Created SDK args module** (foundational work for future)
   - Created [sdk/kaal-sdk/src/args.rs](../sdk/kaal-sdk/src/args.rs)
   - Provides `ComponentArgs` and `ChannelConfig` structures
   - Enables reading initial register arguments (x0, x1, x2)

6. **Added TCB argument setting capability**
   - Added `TCB::set_arguments()` method in [kernel/src/objects/tcb.rs](../kernel/src/objects/tcb.rs)
   - Allows setting initial x0, x1, x2 registers for spawned processes

### ❌ Current Blocker: "Unknown syscall number: 0"

**Symptom**: When scheduler switches to ipc_producer or ipc_consumer, they immediately spam:
```
[syscall] Unknown syscall number: 0
[syscall] Unknown syscall number: 0
[syscall] Unknown syscall number: 0
...
```

**Analysis**:
- system_init component works perfectly with same SDK
- ipc_producer and ipc_consumer were freshly compiled
- All initial registers (x0-x28) are initialized to 0 by `TrapFrame::new()`
- The issue suggests the binary entry point or initial execution is incorrect

**Potential Root Causes**:
1. Component loader extracting wrong entry point from ELF
2. Binary being loaded at wrong virtual address
3. Some initialization code in SDK trying to access registers before _start
4. Relocation or linking issue with freshly compiled components

### 🎯 Architecture Decision: Capability Broker Should Handle IPC Setup

**User Feedback**: "Cap slots should be automatically determined, component spawning will be dynamic, so we shouldn't have to manually pass these arguments."

**Agreed Design**:
- Components should NOT manually manage capability slots
- Capability Broker (or future service discovery system) should handle IPC setup
- Components request connections by name, system handles the details

**Current Interim Approach** (until Capability Broker is a proper service):
- Root-task uses broker library to set up IPC infrastructure
- Well-known capability slots for standard IPC (slots 100-103)
- Components use SDK Channel API which abstracts the details

## Files Modified

### Build System
- `build-system/builders/mod.nu` - Build ALL components, not just autostart

### Component Manifests
- `components/ipc-producer/Cargo.toml` - Added workspace isolation
- `components/ipc-consumer/Cargo.toml` - Added workspace isolation

### Kernel
- `kernel/src/objects/tcb.rs` - Added `set_arguments()` method

### SDK
- `sdk/kaal-sdk/src/args.rs` - NEW: Component argument passing module
- `sdk/kaal-sdk/src/lib.rs` - Export args module

### Runtime
- `runtime/root-task/src/main.rs` - Phase 5: Spawn IPC components

### Components (reverted to original)
- `components/ipc-producer/src/main.rs` - Restored original implementation
- `components/ipc-consumer/src/main.rs` - (should be same as ipc-producer)

## Next Steps

### Immediate (Fix the Blocker)
1. **Debug why ipc-producer/consumer crash with syscall 0**
   - Compare ELF headers: system_init vs ipc_producer
   - Verify entry point addresses are correct
   - Check if binaries are being loaded at correct virtual addresses
   - Add kernel debug output in component loader's ELF parsing

2. **Simplify for testing**
   - Create minimal test component that just prints and yields
   - Verify it works before adding IPC complexity

### Phase 5: IPC Implementation (After blocker is fixed)
1. **Root-task: Setup IPC infrastructure**
   ```rust
   // Allocate shared memory
   let shared_mem = broker.allocate_memory(32768)?;

   // Create notifications
   let producer_notify = broker.create_notification()?;
   let consumer_notify = broker.create_notification()?;

   // Map shared memory into both components
   sys_memory_map_into(producer_pid, shared_mem, 0x80100000, PERMS_RW);
   sys_memory_map_into(consumer_pid, shared_mem, 0x80100000, PERMS_RW);

   // Insert notification caps
   sys_cap_insert_into(producer_pid, 102, consumer_notify);
   sys_cap_insert_into(producer_pid, 103, producer_notify);
   sys_cap_insert_into(consumer_pid, 102, consumer_notify);
   sys_cap_insert_into(consumer_pid, 103, producer_notify);
   ```

2. **Components: Use well-known cap slots**
   ```rust
   // In ipc-producer
   let config = ChannelConfig {
       shared_memory: 0x80100000,  // Well-known address
       receiver_notify: 102,        // Well-known slot
       sender_notify: 103,          // Well-known slot
   };
   let channel = Channel::<u32>::sender(config);
   channel.send(42)?;
   ```

3. **Test full IPC communication**
   - Producer sends messages
   - Consumer receives messages
   - Verify notifications work correctly
   - Check that shared memory is properly synchronized

### Future: Proper Service Discovery
1. **Make Capability Broker a proper IPC service**
2. **Implement service registration and discovery**
3. **Add dynamic channel setup via IPC**
4. **Remove well-known slot requirement**

## Lessons Learned

1. **Build system needs to build all components** - Not just autostart ones, for dynamic spawning
2. **Component workspace isolation is critical** - Prevents cargo conflicts
3. **Manual argument passing doesn't scale** - Need automatic capability allocation
4. **Fresh compilation can expose issues** - Binary differences between old/new builds matter
5. **Background process management is important** - Many orphaned QEMU/nu processes accumulated

## System State

### Clean State Restored
- Kernel: Working, no TTBR0-switching bugs
- Root-task: Can spawn system_init successfully
- system_init: Runs and yields correctly
- Scheduler: Round-robin works between root-task and system_init
- Build system: Compiles all components

### Known Working Syscalls
- SYS_YIELD (0x01)
- SYS_DEBUG_PRINT (0x1001)
- SYS_MEMORY_ALLOCATE (0x11)
- SYS_NOTIFICATION_CREATE (0x17)
- SYS_SIGNAL (0x18)
- SYS_POLL (0x1A)
- SYS_PROCESS_CREATE (0x14)
- SYS_MEMORY_MAP_INTO (0x1B)
- SYS_CAP_INSERT_INTO (0x1C)

### Components Status
- ✅ system_init: Works perfectly
- ❌ ipc_producer: Crashes with syscall 0
- ❌ ipc_consumer: Crashes with syscall 0 (not tested yet, likely same issue)

## References

- Main discussion about Capability Broker handling IPC: This session
- Build system fix: `build-system/builders/mod.nu` lines 107-115
- Component workspace isolation: Cargo error message suggested fix
- TCB argument setting: `kernel/src/objects/tcb.rs` lines 345-353
