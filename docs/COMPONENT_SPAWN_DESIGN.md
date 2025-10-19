# SYS_COMPONENT_SPAWN Design

## Goal

Allow userspace components (specifically `system_init`) to spawn other components without requiring root-task privileges.

## Current State

**Root-Task Only**: Component spawning is currently restricted to root-task via `component_loader`:
- Parses ELF binaries embedded in root-task
- Allocates memory for process image, stack, page tables, CSpace
- Copies segments to memory
- Calls `SYS_PROCESS_CREATE` to create TCB
- Inserts TCB capability for parent

**Problem**: Application logic (IPC demos, tests) runs in root-task, violating separation of concerns.

## Proposed Solutions

### Option A: Full SYS_COMPONENT_SPAWN (Complex)

```rust
// Syscall interface:
sys_component_spawn(name: &str) -> Result<TcbCap, Error>
```

**Kernel does everything**:
1. Lookup component by name in kernel-side registry
2. Parse ELF binary
3. Allocate memory (process image, stack, page table, CSpace)
4. Copy ELF segments
5. Create process via internal process_create
6. Return TCB capability to caller

**Pros**:
- Simple userspace API
- Single syscall does everything

**Cons**:
- Moves complex logic (ELF parsing, registry) into kernel
- Violates microkernel philosophy (mechanism not policy)
- Kernel grows significantly
- Component registry must be in kernel

### Option B: Move ComponentLoader to SDK (Microkernel-Friendly)

**Keep kernel minimal**, provide SDK helper:

```rust
// SDK function (userspace):
pub fn component_spawn(registry: &ComponentRegistry, name: &str)
    -> Result<TcbCap, Error> {
    // 1. Lookup component in userspace registry
    let desc = registry.find(name)?;

    // 2. Parse ELF (userspace)
    let elf = elf::parse(desc.binary_data)?;

    // 3. Allocate memory (existing syscalls)
    let process_mem = syscall::memory_allocate(elf.size())?;
    let stack = syscall::memory_allocate(16384)?;
    let pt_root = syscall::memory_allocate(4096)?;
    let cspace = syscall::memory_allocate(4096)?;

    // 4. Map and copy segments (userspace)
    let virt = syscall::memory_map(process_mem, elf.size(), RW)?;
    elf.copy_segments_to(virt)?;
    syscall::memory_unmap(virt, elf.size())?;

    // 5. Create process (existing syscall)
    let pid = syscall::process_create(
        elf.entry_point,
        stack_ptr,
        pt_root,
        cspace,
    )?;

    // 6. Insert TCB cap (existing syscall)
    let tcb_cap = syscall::cap_allocate()?;
    syscall::cap_insert_self(tcb_cap, CAP_TCB, pid)?;

    Ok(tcb_cap)
}
```

**Pros**:
- Kernel stays minimal (no new syscall needed!)
- Reuses existing syscalls
- Policy (which components to spawn) in userspace
- Matches seL4 philosophy

**Cons**:
- More complex userspace code
- Component registry must be passed to system_init
- Requires moving component_loader logic to SDK

### Option C: Hybrid - Privileged Spawn Service

**Root-task exposes component spawning as IPC service**:

```rust
// Root-task runs ComponentLoaderService
impl ComponentLoaderService {
    fn handle_spawn_request(&mut self, name: &str) -> TcbCap {
        self.loader.spawn(name)
    }
}

// System_init calls via IPC
let tcb_cap = ipc_call(ROOT_TASK_LOADER_EP, SpawnRequest { name: "ipc_producer" })?;
```

**Pros**:
- No kernel changes needed
- Complex logic stays in root-task
- Clean IPC-based architecture

**Cons**:
- Requires IPC endpoint infrastructure
- More latency (IPC overhead)
- Root-task must run a service loop

## Recommendation

**Start with Option B** (SDK helper) because:

1. **Kernel stays minimal** - no new syscalls needed
2. **Reuses existing infrastructure** - all syscalls already exist
3. **Matches microkernel philosophy** - mechanism in kernel, policy in userspace
4. **Clean architecture** - component_loader becomes `kaal_sdk::component::spawn()`

## Implementation Plan

### Phase 1: Move ComponentRegistry to SDK

```rust
// sdk/kaal-sdk/src/component/registry.rs
pub struct ComponentRegistry {
    components: &'static [ComponentDescriptor],
}

impl ComponentRegistry {
    pub fn from_static(components: &'static [ComponentDescriptor]) -> Self {
        ComponentRegistry { components }
    }

    pub fn find(&self, name: &str) -> Option<&ComponentDescriptor> {
        self.components.iter().find(|c| c.name == name)
    }
}
```

### Phase 2: Move ELF Parsing to SDK

```rust
// sdk/kaal-sdk/src/elf/mod.rs
pub fn parse(data: &[u8]) -> Result<ElfInfo, Error> {
    // Move kernel/src/boot/elf.rs logic here
}
```

### Phase 3: Add SDK Spawn Helper

```rust
// sdk/kaal-sdk/src/component/spawn.rs
pub fn spawn(registry: &ComponentRegistry, name: &str)
    -> Result<SpawnResult, Error> {
    // Implementation from component_loader.rs
}
```

### Phase 4: Update System_Init

```rust
// components/system-init/src/main.rs
let registry = generated::component_registry::get_registry();
let producer_tcb = kaal_sdk::component::spawn(registry, "ipc_producer")?;
let consumer_tcb = kaal_sdk::component::spawn(registry, "ipc_consumer")?;
```

### Phase 5: Remove from Root-Task

Root-task only spawns autostart components, hands off registry to system_init.

## Timeline

- **Now**: Document design, add TODOs
- **Next Session**: Implement Phase 1-3 (SDK infrastructure)
- **Following**: Implement Phase 4-5 (move spawning to system_init)

## Benefits

- Clean separation: kernel → runtime → application
- Reusable SDK for all components
- No kernel bloat
- Testable in isolation
- Follows seL4 patterns

