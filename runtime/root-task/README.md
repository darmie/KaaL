# KaaL Root Task

The first userspace program that runs on the KaaL microkernel.

## Overview

The root task is a privileged userspace process (running at EL0) that serves as the bootstrap for the entire userspace system. It receives initial capabilities from the kernel and is responsible for:

1. **Resource Management** - Manages UntypedMemory and delegates to other processes
2. **Process Spawning** - Loads and spawns system_init and other early processes
3. **Capability Brokering** - Creates and distributes capabilities to child processes
4. **System Initialization** - Sets up the runtime environment for the rest of the system

The root task is the **only** process spawned directly by the kernel. All other processes are spawned by the root task or its descendants.

## Architecture

```
┌─────────────────────────────────────────────┐
│  Kernel (EL1)                               │
│  ↓ (creates initial caps + spawns)          │
└─────────────────────────────────────────────┘
           ↓
┌─────────────────────────────────────────────┐
│  Root Task (EL0)                            │
│  ┌─────────────────────────────────────┐   │
│  │  Initial Capabilities:               │   │
│  │  - UntypedMemory (32MB)              │   │
│  │  - Own CSpace/VSpace/TCB             │   │
│  │  - IRQControl                        │   │
│  │  - Notification for boot_info        │   │
│  └─────────────────────────────────────┘   │
│  ┌─────────────────────────────────────┐   │
│  │  Responsibilities:                   │   │
│  │  1. Parse boot_info from kernel      │   │
│  │  2. Initialize heap allocator        │   │
│  │  3. Load system_init ELF             │   │
│  │  4. Delegate UntypedMemory           │   │
│  │  5. Spawn system_init process        │   │
│  └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
           ↓
┌─────────────────────────────────────────────┐
│  system_init (EL0)                          │
│  - Spawns ipc_producer, ipc_consumer        │
│  - Spawns uart_driver, notepad              │
│  - Manages application lifecycle            │
└─────────────────────────────────────────────┘
```

## Design Philosophy

**Minimal Privilege:**

The root task has special privileges (access to all initial resources), but it immediately delegates most of them to system_init. This follows the principle of least privilege - the root task's job is to bootstrap the system, not to run it.

**Resource Delegation:**

The root task receives `UntypedMemory` from the kernel and uses `sys_retype` to:

- Create child UntypedMemory for system_init
- Create TCBs, VSpaces, CNodes for child processes
- Create Endpoints and Notifications for IPC

**Single Responsibility:**

The root task does **one thing**: spawn system_init and delegate resources. It doesn't run services, handle devices, or manage applications - that's system_init's job.

## Initial Capabilities

The kernel provides the root task with these initial capabilities in its CSpace:

| Slot | Capability | Purpose |
|------|------------|---------|
| 1    | CSpace (self) | Root task's own capability space |
| 2    | VSpace (self) | Root task's own virtual address space |
| 3    | TCB (self) | Root task's own thread control block |
| 4    | UntypedMemory | 32MB of physical memory for retyping |
| 5    | IRQControl | Permission to create IRQ handlers |
| 6    | Notification | For receiving boot_info from kernel |

Additional capabilities are allocated dynamically using `sys_retype`.

## Boot Sequence

### 1. Kernel Spawns Root Task

```rust
// In kernel initialization:
let root_tcb = create_root_task(untyped_memory, irq_control);
switch_to_el0(root_tcb);
```

### 2. Root Task Entry Point

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Initialize heap allocator
    allocator::init();

    // 2. Receive boot_info from kernel via notification
    let boot_info = receive_boot_info();

    // 3. Load system_init ELF from boot_info
    let system_init_elf = &boot_info.system_init_image;

    // 4. Parse ELF and determine memory requirements
    let elf = elf::parse(system_init_elf)?;

    // 5. Create child UntypedMemory for system_init
    let child_untyped = sys_retype(
        untyped_cap,
        ObjectType::UntypedMemory,
        system_init_untyped_slot,
    )?;

    // 6. Spawn system_init using delegated resources
    spawn_from_elf_with_untyped(elf, child_untyped)?;

    // 7. Yield forever (job done!)
    loop {
        sys_yield();
    }
}
```

### 3. System Init Takes Over

Once system_init is running, it becomes responsible for spawning all other applications.

## Resource Delegation with sys_retype

The root task uses `sys_retype` to create kernel objects from UntypedMemory:

```rust
// Create a 16MB child UntypedMemory for system_init
sys_retype(
    root_untyped_cap,              // Source: Root's 32MB untyped
    ObjectType::UntypedMemory,     // Target type
    system_init_untyped_slot,      // Destination slot
)?;

// Create a TCB for system_init's main thread
sys_retype(
    root_untyped_cap,
    ObjectType::Tcb,
    system_init_tcb_slot,
)?;

// Create a VSpace (page table root) for system_init
sys_retype(
    root_untyped_cap,
    ObjectType::VSpace,
    system_init_vspace_slot,
)?;

// Create a CNode (capability space) for system_init
sys_retype(
    root_untyped_cap,
    ObjectType::CNode,
    system_init_cspace_slot,
)?;
```

The watermark allocator inside `UntypedMemory` ensures no fragmentation - each allocation consumes a contiguous region and advances the watermark.

## ELF Loading

The root task includes an ELF loader that:

1. Parses ELF headers and program segments
2. Allocates pages for each segment (code, data, bss)
3. Maps pages into the new process's VSpace
4. Copies segment data from ELF image
5. Sets entry point and stack pointer
6. Transfers capabilities to child's CSpace

```rust
pub fn spawn_from_elf_with_untyped(
    elf_data: &[u8],
    untyped_cap: usize,
) -> Result<ProcessHandle, ComponentError> {
    // Parse ELF
    let elf = Elf::parse(elf_data)?;

    // Create kernel objects from untyped
    let tcb = sys_retype(untyped_cap, ObjectType::Tcb, ...)?;
    let vspace = sys_retype(untyped_cap, ObjectType::VSpace, ...)?;
    let cspace = sys_retype(untyped_cap, ObjectType::CNode, ...)?;

    // Load segments
    for segment in elf.segments {
        let page = sys_retype(untyped_cap, ObjectType::Page, ...)?;
        sys_memory_map(page, vspace, segment.vaddr, ...)?;
        copy_segment_data(segment, page);
    }

    // Configure TCB
    sys_tcb_configure(tcb, cspace, vspace, elf.entry_point)?;

    // Start thread
    sys_thread_resume(tcb)?;

    Ok(ProcessHandle { tcb, cspace, vspace })
}
```

## Component Registry

The root task includes a generated component registry that lists all components to spawn:

```rust
// Generated by build system from components.toml
pub static COMPONENT_REGISTRY: &[ComponentManifest] = &[
    ComponentManifest {
        name: "system_init",
        elf_data: include_bytes!("../../system-init/target/.../system_init"),
        spawned_by: Root,
    },
    ComponentManifest {
        name: "ipc_producer",
        elf_data: include_bytes!("../../components/ipc_producer/target/.../ipc_producer"),
        spawned_by: SystemInit,
    },
    // ... more components
];
```

The root task only spawns components where `spawned_by = Root`. Other components are spawned by their designated parent.

## Memory Management

The root task uses a simple bump allocator for heap allocations:

```rust
// In allocator.rs
static mut HEAP: [u8; 256 * 1024] = [0; 256 * 1024]; // 256KB heap

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();
```

This is sufficient for the root task's limited lifetime (it runs briefly during boot, then yields forever).

## Capability Management

The root task maintains a simple capability slot allocator:

```rust
static mut NEXT_CAP_SLOT: usize = 10; // First 10 slots are reserved

fn allocate_cap_slot() -> usize {
    unsafe {
        let slot = NEXT_CAP_SLOT;
        NEXT_CAP_SLOT += 1;
        slot
    }
}
```

Capabilities are stored in the root task's CSpace and can be copied/moved to child processes via `sys_cap_copy`.

## Building

The root task is built as part of the main KaaL build:

```bash
cd /path/to/kaal
nu build.nu
```

The build system:

1. Compiles root-task with `no_std` and `no_main`
2. Links with custom linker script for userspace
3. Generates component registry from `components.toml`
4. Embeds root-task ELF into kernel boot parameters

### Manual Build

```bash
cd runtime/root-task
cargo build --release --target aarch64-unknown-none
```

## Code Structure

```text
runtime/root-task/
├── src/
│   ├── main.rs                  # Entry point and main logic
│   ├── allocator.rs             # Heap allocator (bump)
│   ├── elf.rs                   # ELF parser
│   ├── elf_xmas.rs              # ELF loader (xmas = extended)
│   ├── component_loader.rs      # Component spawning logic
│   ├── broker_integration.rs    # Capability brokering
│   └── generated/
│       └── registry.rs          # Component registry (generated)
├── Cargo.toml                   # Dependencies
├── build.rs                     # Build-time code generation
└── root-task.ld                 # Linker script (userspace)
```

## Key Modules

### [src/main.rs](src/main.rs)

- `_start` - Entry point (called by kernel)
- `receive_boot_info` - Parse kernel boot parameters
- Main spawn loop

### [src/elf.rs](src/elf.rs)

- ELF header parsing
- Program header parsing
- Segment validation

### [src/component_loader.rs](src/component_loader.rs)

- `spawn_from_elf_with_untyped` - Main spawn function
- Component manifest handling
- Resource allocation

### [src/allocator.rs](src/allocator.rs)

- Global heap allocator
- Simple bump allocator implementation

## System Calls Used

The root task uses these syscalls:

- `sys_retype` (0x26) - Create kernel objects from UntypedMemory
- `sys_memory_map` (0x15) - Map pages into child VSpace
- `sys_cap_copy` (0x30) - Copy capabilities to child CSpace
- `sys_thread_resume` (0x07) - Start child thread
- `sys_yield` (0x01) - Yield to scheduler
- `sys_wait` (0x19) - Wait for boot_info notification

## Debugging

Enable debug output by setting `debug = true` in root-task's Cargo.toml:

```toml
[features]
debug = []
```

Then use the `printf!` macro:

```rust
printf!("[root-task] Spawning system_init...\n");
```

Debug output goes to UART via `sys_debug_putchar`.

## Testing

```bash
# Build and run in QEMU
nu build.nu
nu run.nu

# Expected boot sequence:
# 1. Kernel boots and spawns root-task
# 2. Root task receives boot_info
# 3. Root task spawns system_init
# 4. System init spawns applications
# 5. Applications run and communicate via IPC
```

## Performance Considerations

**Fast Boot:**

The root task is optimized for fast boot time:

- Simple bump allocator (no malloc overhead)
- Minimal parsing (ELF headers only)
- Direct syscalls (no abstraction layers)

**Small Footprint:**

- Binary size: ~50KB
- Heap usage: 256KB
- Stack usage: 16KB

The root task runs briefly and then yields, so its memory can be reclaimed or reused.

## Dependencies

```toml
[dependencies]
# None - fully self-contained
```

The root task has **zero dependencies** to ensure minimal attack surface and fast boot.

## Capabilities vs Traditional Unix

**Traditional Unix (fork/exec):**

```c
int pid = fork();
if (pid == 0) {
    // Child process - inherits all permissions
    exec("/bin/sh");
}
```

**KaaL (capability-based):**

```rust
// Explicitly delegate only necessary capabilities
let tcb = sys_retype(untyped, ObjectType::Tcb, ...)?;
let vspace = sys_retype(untyped, ObjectType::VSpace, ...)?;
let minimal_cspace = sys_retype(untyped, ObjectType::CNode, ...)?;

// Child only gets what we explicitly give it
sys_cap_copy(endpoint_cap, minimal_cspace, 0)?;
```

**Advantages:**

- Least privilege by default
- Explicit capability transfer
- No ambient authority
- Revocable access (via CDT)

## Limitations

The root task is intentionally simple:

- **No preemption handling** - Assumes it runs to completion
- **No error recovery** - Boot failure is fatal
- **No dynamic configuration** - Component list is compile-time
- **No IPC** - One-way spawn only

These limitations are acceptable because the root task's lifetime is brief and its purpose is singular.

## Future Work

Possible enhancements:

- **Dynamic component discovery** - Load components from filesystem
- **Configuration file parsing** - Read boot config instead of hardcoded
- **Error recovery** - Retry spawn failures with fallback
- **Multi-stage init** - Support init levels (like systemd targets)

However, the current design follows the principle of **simplicity first** - the root task does one thing well.

## License

MIT OR Apache-2.0

## See Also

- [../system-init/README.md](../system-init/README.md) - System init documentation
- [../../kernel/README.md](../../kernel/README.md) - Kernel documentation
- [../../docs/MICROKERNEL_CHAPTERS.md](../../docs/MICROKERNEL_CHAPTERS.md) - Chapter roadmap
