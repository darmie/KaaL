# TODO: Complete Capability Granting Implementation

## Status: WIP - DOES NOT COMPILE

The TCB struct has been updated to include capabilities, but test files need updating.

## Completed ✅

1. Added `capabilities: u64` field to TCB struct
2. Defined capability bit constants:
   - `CAP_MEMORY` (bit 0): memory_allocate, memory_map, memory_unmap
   - `CAP_PROCESS` (bit 1): process_create, process_delete
   - `CAP_IPC` (bit 2): notifications, endpoints
   - `CAP_CAPS` (bit 3): capability operations
   - `CAP_ALL`: all capabilities (0xFFFFFFFFFFFFFFFF)
3. Added `TCB::has_capability()` helper method
4. Updated `TCB::new()` signature to accept capabilities parameter
5. Updated root-task creation to grant CAP_ALL
6. Updated idle thread creation with no capabilities
7. Updated sys_process_create to use CAP_ALL (temporary)

## Remaining Work 🚧

###  1. Fix Test Files (Compilation Errors)

Need to add `TCB::CAP_ALL` parameter to all TCB::new() calls in:
- `src/objects/test_runner.rs` (multiple calls)
- `src/ipc/test_runner.rs`
- `src/objects/endpoint.rs` (test functions)
- `src/objects/tests.rs`
- `src/objects/invoke.rs` (test functions)

**Quick Fix**: Add `TCB::CAP_ALL,` as the 7th parameter to each TCB::new() call.

### 2. Update sys_process_create Syscall

File: `kernel/src/syscall/mod.rs`

Current (line ~736):
```rust
let tcb = TCB::new(
    pid, cspace_ptr, page_table_root, ipc_buffer,
    entry_point, stack_pointer,
    TCB::CAP_ALL,  // TODO: Accept from caller
);
```

Needs:
- Add `capabilities: u64` parameter to sys_process_create function signature
- Pass capabilities from syscall arguments (x7 register?)
- Update syscall number definition if needed

### 3. Update SDK spawn_from_elf()

File: `sdk/kaal-sdk/src/component/spawn.rs`

Needs:
- Parse capabilities from component metadata (passed from components.toml)
- Pass capabilities to sys_process_create syscall
- Add capabilities parameter to spawn_from_elf() function

### 4. Add Syscall Capability Checks

Add capability checking to sensitive syscalls:

**Memory syscalls** (require CAP_MEMORY):
- sys_memory_allocate
- sys_memory_map
- sys_memory_unmap

**Process syscalls** (require CAP_PROCESS):
- sys_process_create

**IPC syscalls** (require CAP_IPC):
- sys_notification_create
- sys_notification_signal
- sys_notification_wait/poll
- sys_endpoint_send/recv/call/reply

**Capability syscalls** (require CAP_CAPS):
- sys_cap_allocate
- sys_cap_insert_self
- sys_cap_delete

Example:
```rust
fn sys_memory_allocate(...) -> Result<...> {
    let current_tcb = scheduler::current_thread();
    if !current_tcb.has_capability(TCB::CAP_MEMORY) {
        return Err(Error::NoCapability);
    }
    // ... rest of implementation
}
```

### 5. Parse Capabilities from components.toml

File: `build-system/builders/codegen.nu` or component_loader

Needs:
- Parse `capabilities` array from components.toml for each component
- Convert capability strings to bitmask:
  - `"memory:allocate"` → CAP_MEMORY
  - `"process:create"` → CAP_PROCESS
  - `"ipc:*"` → CAP_IPC
  - etc.
- Pass capability bitmask through spawn chain:
  - components.toml → component_registry.rs → spawn_from_elf() → sys_process_create

### 6. Test End-to-End

1. Build with capability granting enabled
2. Mark system_init with needed capabilities in components.toml:
   ```toml
   capabilities = [
       "memory:allocate",
       "memory:map",
       "process:create",
       "ipc:*",
   ]
   ```
3. Enable `spawned_by="system_init"` for IPC components
4. Verify system_init can spawn IPC producer/consumer
5. Verify components WITHOUT capabilities get denied

## Next Session

Start by fixing the compilation errors in test files, then proceed with implementing syscall capability checks.
