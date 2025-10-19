# System Init - Developer Playground

## Overview

`system_init` is the **developer entry point** for the KaaL microkernel. This is where you should add:
- Application logic
- Feature demos
- Experiments
- Integration tests

**DO NOT** modify kernel or root-task for application-level development. Keep them minimal and focused on system initialization.

## Architecture

```
┌─────────────────────────────────────┐
│         Kernel (EL1)                │
│  - Memory management                │
│  - Process scheduling                │
│  - Syscall handling                  │
│  - Low-level initialization          │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│      Root-Task (EL0, privileged)    │
│  - Runtime service initialization   │
│  - Component registry setup         │
│  - Autostart component spawning     │
│  - ONE-TIME initialization only     │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│    system_init (EL0, first service) │  ← YOU ARE HERE!
│  - Application logic                │
│  - Feature demos                    │
│  - Additional component spawning    │
│  - YOUR experiments go here         │
└─────────────────────────────────────┘
```

## What Goes Where

### Kernel (`kernel/`)
- **DO**: Memory allocator, page tables, exception handlers, syscall dispatch
- **DON'T**: Application features, demos, integration tests

### Root-Task (`runtime/root-task/`)
- **DO**: Initialize runtime services, spawn autostart components
- **DON'T**: Application logic, feature demos, experiments

### System Init (`components/system-init/`) ← **YOUR PLAYGROUND**
- **DO**: Everything else!
  - Spawn additional components
  - Test IPC patterns
  - Demo new features
  - Build applications
  - Experiment with syscalls

## Available Infrastructure

When `system_init` starts, the following are already initialized:

✅ **Memory Management**
- Physical memory allocator
- Virtual memory (per-process page tables)
- Syscalls: `SYS_MEMORY_ALLOCATE`, `SYS_MEMORY_MAP`, `SYS_MEMORY_UNMAP`

✅ **IPC**
- Shared memory
- Notification objects
- Decentralized channel establishment
- Syscalls: `SYS_SHMEM_REGISTER`, `SYS_SHMEM_QUERY`, `SYS_NOTIFICATION_CREATE`

✅ **Capabilities**
- seL4-style capability system
- Capability spaces per process
- Syscalls: `SYS_CAP_ALLOCATE`, `SYS_CAP_INSERT_SELF`

✅ **Components**
- Component registry
- ELF loading
- Autostart components already running

## Current Limitations

🚧 **Component Spawning from Userspace**

Currently, only root-task can spawn components. You need privileged access to:
- Load ELF binaries
- Create page tables
- Allocate TCBs

**TODO**: Implement `SYS_COMPONENT_SPAWN` syscall to allow system_init to spawn components.

Until then, the IPC demo runs from root-task (see `runtime/root-task/src/main.rs`).

## Example: Adding Your Experiment

```rust
impl Component for SystemInit {
    fn run(&mut self) -> ! {
        unsafe {
            // Print banner
            syscall::print("[system_init] My Experiment\n");

            // Your code here!
            // - Test new syscalls
            // - Allocate memory
            // - Create IPC channels
            // - Whatever you want!

            // Event loop
            loop {
                syscall::wait(notification_cap);
            }
        }
    }
}
```

## Future Roadmap

1. **SYS_COMPONENT_SPAWN** - Allow userspace component spawning
2. **Move IPC Demo** - From root-task to system_init
3. **VFS Integration** - Mount filesystems, load binaries from disk
4. **Shell Component** - Interactive development shell
5. **Test Framework** - Automated testing infrastructure

## Philosophy

**Kernel & Root-Task**: Minimal, stable, rarely change
**System Init**: Experimental, flexible, change often

This separation allows rapid application development without touching critical system code.

---

**TL;DR**: Add your code to `system_init`, not kernel or root-task!
