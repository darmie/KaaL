# Capability Granting Implementation Summary

## Status: Phase 1 Complete ✅

This document summarizes the capability-based security infrastructure implemented in KaaL.

## What Was Implemented

### 1. Capability Infrastructure in TCB
- **File**: `kernel/src/objects/tcb.rs`
- Added `capabilities: u64` field to TCB struct
- Defined capability bit constants:
  - `CAP_MEMORY (bit 0)`: memory_allocate, memory_map, memory_unmap
  - `CAP_PROCESS (bit 1)`: process_create
  - `CAP_IPC (bit 2)`: notification and endpoint operations
  - `CAP_CAPS (bit 3)`: cap_allocate, cap_insert_into, cap_insert_self
  - `CAP_ALL (0xFFFF...)`: all capabilities for privileged processes
- Added `has_capability()` helper method for checking capabilities
- Updated `TCB::new()` signature to accept capabilities parameter

### 2. Capability Grants at Boot
- **File**: `kernel/src/boot/root_task.rs`
- Root-task receives CAP_ALL (full privileges)
- **File**: `kernel/src/boot/mod.rs`
- Idle thread receives no capabilities (can't call syscalls)

### 3. Runtime Capability Checking
- **File**: `kernel/src/syscall/mod.rs`
- Added capability checks to all major syscall handlers:
  - `sys_memory_allocate()` - requires CAP_MEMORY
  - `sys_memory_map()` - requires CAP_MEMORY
  - `sys_memory_unmap()` - requires CAP_MEMORY
  - `sys_process_create()` - requires CAP_PROCESS
  - `sys_cap_allocate()` - requires CAP_CAPS
  - `sys_cap_insert_into()` - requires CAP_CAPS
  - `sys_cap_insert_self()` - requires CAP_CAPS
- Each syscall checks caller's capabilities before proceeding
- Returns permission denied (u64::MAX) if required capability not present

### 4. Process Creation with Capabilities
- **File**: `kernel/src/syscall/mod.rs`
- Updated `sys_process_create()` to accept capabilities parameter (passed in x10)
- New processes receive specified capability bitmask instead of always CAP_ALL
- **File**: `sdk/kaal-sdk/src/syscall.rs`
- Updated SDK wrapper to accept and pass capabilities
- Extended syscall! macro to support 10 arguments (8 args + priority + capabilities)
- **File**: `runtime/root-task/src/main.rs`
- Updated root-task's sys_process_create wrapper for x10 register
- **File**: `runtime/root-task/src/component_loader.rs`
- Temporarily grants CAP_ALL to all spawned components (TODO: parse from manifest)

### 5. SDK Spawn API
- **File**: `sdk/kaal-sdk/src/component/spawn.rs`
- Updated `spawn_from_elf()` to accept capabilities parameter
- Passes capabilities through to sys_process_create

## How It Works

```rust
// 1. TCB stores capability bitmask
pub struct TCB {
    capabilities: u64,  // Each bit represents a capability
    // ... other fields
}

// 2. Syscalls check capabilities at runtime
fn sys_memory_allocate(size: u64) -> u64 {
    unsafe {
        let current_tcb = crate::scheduler::current_thread();
        if !(*current_tcb).has_capability(TCB::CAP_MEMORY) {
            return u64::MAX; // Permission denied
        }
    }
    // ... actual implementation
}

// 3. New processes receive specific capabilities
let pid = syscall::process_create(
    entry_point,
    stack_pointer,
    page_table_root,
    cspace_root,
    code_phys,
    code_vaddr,
    code_size,
    stack_phys,
    priority,
    capabilities,  // Passed from caller
)?;
```

## Current Behavior

- ✅ Root-task: CAP_ALL (full privileges)
- ✅ Idle thread: No capabilities
- ✅ Spawned components: CAP_ALL (temporary - should be limited)
- ✅ Syscalls enforce capability checks
- ✅ Processes can be spawned with specific capabilities

## Testing Results

Running the system in QEMU shows:
- ✅ System boots successfully
- ✅ Root-task has full syscall access
- ✅ system_init spawns and runs
- ✅ Memory allocation syscalls work
- ✅ Process creation syscalls work
- ✅ No permission denied errors (all components have CAP_ALL)

## Remaining Work (Phase 2)

### 1. Capability Parsing from components.toml
**File**: `build-system/builders/codegen.nu`

Parse capability declarations and convert to bitmask:
```toml
[[component]]
name = "ipc_producer"
capabilities = [
    "memory:map",           # CAP_MEMORY
    "notification:signal",  # CAP_IPC
    "notification:wait",    # CAP_IPC
]
```

Should generate:
```rust
const CAP_BITMASK: u64 = (1 << 0) | (1 << 2);  // CAP_MEMORY | CAP_IPC
```

### 2. Pass Actual Capabilities to Components
**File**: `runtime/root-task/src/component_loader.rs`

Replace:
```rust
const CAP_ALL: u64 = 0xFFFFFFFFFFFFFFFF;
let pid = sys_process_create(..., CAP_ALL);
```

With:
```rust
let capabilities = desc.capabilities_bitmask;  // From generated registry
let pid = sys_process_create(..., capabilities);
```

### 3. Test Capability Enforcement
Create test components with limited capabilities and verify:
- Components without CAP_MEMORY cannot allocate memory
- Components without CAP_PROCESS cannot spawn new processes
- Components without CAP_CAPS cannot manage capabilities
- Permission denied errors are properly returned

### 4. Documentation
- Document capability model in chapter docs
- Update component development guide
- Add capability debugging guide

## Design Notes

### Why Bitmask?
- Fast checking: single AND operation
- Compact: 64 bits = 64 possible capabilities
- seL4-style: follows microkernel best practices
- Future-proof: bits 4-63 reserved for future capabilities

### Why Not Per-Syscall Capabilities?
We use coarse-grained capability groups (MEMORY, PROCESS, IPC, CAPS) instead of per-syscall capabilities because:
1. Simpler to reason about
2. Matches typical component needs (drivers need memory, services need IPC)
3. Reduces complexity in components.toml
4. Can be refined later if needed

### Security Considerations
- Root-task must be trusted (has CAP_ALL)
- Capability delegation is safe (can only grant capabilities you have)
- Capability revocation not yet implemented (future work)
- Idle thread correctly has no capabilities

## Related Files

**Kernel**:
- `kernel/src/objects/tcb.rs` - TCB with capabilities field
- `kernel/src/syscall/mod.rs` - Syscall handlers with capability checks
- `kernel/src/boot/root_task.rs` - Root-task capability grant
- `kernel/src/boot/mod.rs` - Idle thread capability grant

**SDK**:
- `sdk/kaal-sdk/src/syscall.rs` - Syscall wrappers and macro
- `sdk/kaal-sdk/src/component/spawn.rs` - spawn_from_elf API

**Runtime**:
- `runtime/root-task/src/main.rs` - Root-task syscall wrapper
- `runtime/root-task/src/component_loader.rs` - Component spawning

**Build System**:
- `build-system/builders/codegen.nu` - Component registry generation (TODO: capability parsing)

## Commits

1. `feat(kernel): Add capability infrastructure to TCB` - Core capability bitmask and checking
2. `feat(kernel): Fix compilation errors from TCB signature change` - Test file updates
3. `feat(kernel): Add capability checks to syscall handlers` - Runtime enforcement
4. `feat(kernel): sys_process_create accepts capabilities parameter` - Capability passing

## Next Session

Start with capability parsing from components.toml:
1. Update codegen.nu to parse capability strings
2. Generate capability bitmasks in component registry
3. Update component_loader.rs to use parsed capabilities
4. Test with limited capability components
