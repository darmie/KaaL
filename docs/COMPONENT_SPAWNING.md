# Component Spawning Implementation Plan

**Goal**: Enable root-task to spawn independent userspace components (threads)

**Status**: 🚧 In Progress

---

## Architecture

### Component Model
- **Component** = Single thread with dedicated:
  - Page table (TTBR0) for address space isolation
  - Stack (256KB)
  - CSpace (capability space) - for now, NULL/shared with root
  - IPC buffer (4KB)

### Component Lifecycle
```
1. Root-task has embedded ELF binary
2. Root-task calls ELF loader
   ├── Parses ELF headers
   ├── Allocates memory for code/data
   ├── Creates page table
   ├── Maps code/data/stack
   └── Calls sys_thread_create()
3. Kernel creates TCB
   ├── Allocates TCB from frame allocator
   ├── Initializes context (PC, SP, TTBR0)
   ├── Adds to scheduler
   └── Returns thread ID
4. Component starts executing
```

---

## Implementation Tasks

### 1. Kernel: sys_thread_create Syscall (~150 LOC)

**Location**: `kernel/src/syscall/mod.rs`, `kernel/src/syscall/numbers.rs`

**Signature**:
```rust
sys_thread_create(entry: u64, stack: u64, page_table: u64) -> u64
```

**Arguments**:
- `entry`: Entry point (PC/ELR_EL1)
- `stack`: Stack pointer (SP_EL0)
- `page_table`: Physical address of page table root (TTBR0)

**Returns**: Thread ID (or u64::MAX on error)

**Implementation**:
1. Allocate frame for TCB
2. Create TCB with `TCB::new()`
3. Add to scheduler via `scheduler::add_thread()`
4. Return TID

**Dependencies**:
- `crate::objects::TCB`
- `crate::scheduler::add_thread()`
- `crate::memory::alloc_frame()`

### 2. Root-task: Simple ELF Loader (~200 LOC)

**Location**: `runtime/root-task/src/elf.rs`

**Functions**:
```rust
pub struct ElfLoader;

impl ElfLoader {
    /// Load ELF from memory
    pub fn load(elf_data: &[u8]) -> Result<LoadedElf>;
}

pub struct LoadedElf {
    pub entry_point: usize,
    pub stack_top: usize,
    pub page_table: usize,
}
```

**Process**:
1. Parse ELF64 header (check magic, architecture)
2. Find LOAD segments
3. For each segment:
   - Allocate frames via `sys_memory_allocate()`
   - Copy data from ELF to allocated memory
   - Track virtual addresses
4. Create page table:
   - Allocate root page table frame
   - Map all LOAD segments
   - Map stack (allocate 64 pages = 256KB)
5. Return entry point, stack top, page table

### 3. Test Component: echo-server (~100 LOC)

**Location**: `examples/echo-server/`

**Structure**:
```
examples/echo-server/
├── Cargo.toml
├── .cargo/config.toml  # Build for aarch64-unknown-none
└── src/
    └── main.rs
```

**Code**:
```rust
#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Print banner
    sys_print("Echo server started!\n");

    loop {
        // Wait for IPC message
        // Echo it back
        // (For Phase 1: just print and loop)
        sys_print("Echo server running...\n");

        // Sleep/yield
        loop { unsafe { core::arch::asm!("wfi"); } }
    }
}
```

### 4. Root-task Integration (~50 LOC)

**Location**: `runtime/root-task/src/main.rs`

**Add**:
```rust
// Embedded echo-server binary
static ECHO_SERVER: &[u8] = include_bytes!(
    "../../target/aarch64-unknown-none/release/echo-server"
);

// In _start():
sys_print("[root_task] Spawning echo-server component...\n");
let loaded = ElfLoader::load(ECHO_SERVER)?;
let tid = sys_thread_create(
    loaded.entry_point,
    loaded.stack_top,
    loaded.page_table
);
sys_print("[root_task] Echo server spawned with TID: ");
print_number(tid);
```

---

## Testing Plan

### Phase 1: Basic Spawning
- ✅ sys_thread_create compiles
- ✅ Root-task can call syscall
- ✅ Kernel creates TCB
- ✅ Component appears in scheduler

### Phase 2: Component Execution
- ✅ Component's _start() is called
- ✅ Component can print via syscalls
- ✅ Scheduler switches between root-task and component

### Phase 3: IPC (Future)
- Root-task sends message to component
- Component receives and echoes back
- Multi-way IPC works

---

## File Checklist

- [ ] `kernel/src/syscall/numbers.rs` - Add SYS_THREAD_CREATE
- [ ] `kernel/src/syscall/mod.rs` - Implement sys_thread_create()
- [ ] `runtime/root-task/src/elf.rs` - ELF loader
- [ ] `runtime/root-task/src/main.rs` - Component spawning
- [ ] `examples/echo-server/Cargo.toml` - Component crate
- [ ] `examples/echo-server/src/main.rs` - Component code
- [ ] Update build.sh to build echo-server
- [ ] Embed echo-server into root-task

---

## Success Criteria

✅ When system boots:
1. Root-task prints "Spawning echo-server..."
2. Kernel creates TCB for echo-server
3. Scheduler adds echo-server to ready queue
4. Echo-server's _start() executes
5. Echo-server prints "Echo server started!"
6. Both root-task and echo-server appear in system

This demonstrates **multi-component microkernel system**! 🎉
