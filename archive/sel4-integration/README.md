# seL4 Integration Archive

This directory contains the original seL4-based implementation of KaaL Framework components. These implementations are **archived for reference only** and are not part of the current native Rust microkernel.

## Purpose

The code in this directory serves as:
- **API reference** for designing KaaL Framework SDK
- **Historical record** of the seL4 integration phase
- **Design inspiration** for capability broker and IPC patterns

## Archived Components

### Runtime Services
- **cap_broker** - Original seL4 capability broker
- **ipc** - Shared memory IPC with seL4 notifications
- **allocator** - Memory allocator
- **sel4-platform** - seL4 platform abstraction
- **sel4-mock** - Mock seL4 bindings for testing
- **sel4-rust-mock** - Rust seL4 mock wrapper

### Driver Framework
- **dddk** - Device Driver Development Kit (procedural macros)
- **dddk-runtime** - DDDK runtime support

### Components
- **components/vfs** - Virtual filesystem skeleton
- **components/posix** - POSIX compatibility layer skeleton
- **components/network** - Network stack skeleton
- **components/drivers** - Driver collection skeleton

### Tools
- **tools/kaal-compose** - Project management tool

## Current Implementation

The **current** KaaL Framework is built on a native Rust microkernel (Chapters 0-7) with the following structure:

```
├── kernel/                    # Native Rust microkernel (EL1)
├── runtime/
│   ├── capability-broker/     # New native implementation
│   ├── memory-manager/        # New native implementation
│   ├── elfloader/            # Bootloader
│   └── root-task/            # First userspace program
├── sdk/                       # (Coming in Chapter 9)
│   ├── kaal-sdk/
│   └── dddk/
└── examples/                  # (Coming in Chapter 9)
```

## Using This Archive

When implementing new KaaL Framework features, you may reference these files for:

1. **API Design**: The old capability broker API was well-designed
2. **Pattern Matching**: seL4's capability model inspired KaaL's design
3. **IPC Patterns**: Shared memory + notifications pattern
4. **Driver Abstractions**: DDDK macro design

## Important Note

**Do NOT import or depend on these crates in new code.** They are for reference only. The seL4 submodule is also excluded, so these crates will not compile in the current workspace.

---

**Archived**: 2025-10-14
**Reason**: Transition from seL4-based to native Rust microkernel
**Status**: Read-only reference material
