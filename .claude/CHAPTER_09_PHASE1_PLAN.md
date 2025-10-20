# Chapter 9 Phase 1: Runtime Services Foundation - Implementation Plan

**Status**: 🔄 In Progress
**Started**: 2025-10-15
**Estimated Duration**: 2 weeks

---

## Overview

Chapter 9 Phase 1 focuses on building the foundation for runtime services by:
1. Creating a robust boot info structure for userspace services
2. Implementing Capability Broker (~5K LOC)
3. Implementing Memory Manager (~3K LOC)
4. Enhancing root-task to use these services
5. Enabling dynamic component spawning

---

## Progress Tracker

### ✅ Completed Tasks

- [x] **Boot Info Structure Design** (2025-10-15)
  - Created [kernel/src/boot/boot_info.rs](../../kernel/src/boot/boot_info.rs)
  - Includes: UntypedRegion, DeviceRegion, CapabilitySlot
  - ~400 LOC with validation and tests
  - File: [boot_info.rs](../../kernel/src/boot/boot_info.rs)

- [x] **Cleanup: Remove dummy-roottask** (2025-10-15)
  - Removed obsolete runtime/dummy-roottask directory
  - Clarified that runtime/root-task is the actual implementation

### 🔄 In Progress

- [ ] **Kernel-Side Boot Info Generation**
  - Location: [kernel/src/boot/root_task.rs](../../kernel/src/boot/root_task.rs)
  - Task: Populate BootInfo structure during root task creation
  - Pass boot info to userspace at known virtual address (0x8000_0000)

### 📋 Planned Tasks

1. **Capability Broker Implementation** (~5K LOC)
   - Location: `runtime/capability-broker/`
   - Objectives:
     - Hide capability complexity from applications
     - Device resource allocation
     - Untyped memory management
     - IPC endpoint creation

2. **Memory Manager Implementation** (~3K LOC)
   - Location: `runtime/memory-manager/`
   - Objectives:
     - Physical memory allocation
     - Virtual address space management
     - Page table management

3. **Enhanced Root Task**
   - Location: `runtime/root-task/`
   - Objectives:
     - Initialize capability broker and memory manager
     - Replace hardcoded echo-server with dynamic component loading
     - Spawn initial system services

4. **Full IPC Testing** (Chapter 5 deferred tests)
   - Location: `tests/integration/`
   - Objectives:
     - Multi-component send/receive
     - Capability transfer (grant/mint/derive)
     - Call/reply RPC semantics
     - IPC latency benchmarking

5. **Documentation**
   - Create `docs/chapters/CHAPTER_09_PHASE1_STATUS.md`
   - Update BLOCKERS_AND_IMPROVEMENTS.md
   - Update MICROKERNEL_CHAPTERS.md

---

## Implementation Details

### Boot Info Structure

**Design** ([boot_info.rs](../../kernel/src/boot/boot_info.rs)):
```rust
pub struct BootInfo {
    magic: u32,                    // "KAAL" magic number
    version: u32,                  // Structure version
    num_untyped_regions: u32,      // Count of untyped regions
    num_device_regions: u32,       // Count of device regions
    num_initial_caps: u32,         // Count of initial caps

    // Configuration
    cspace_root_slot: u64,
    vspace_root_slot: u64,
    ipc_buffer_vaddr: u64,
    ram_size: u64,
    kernel_virt_base: u64,
    user_virt_start: u64,

    // Arrays
    untyped_regions: [UntypedRegion; 128],
    device_regions: [DeviceRegion; 32],
    initial_caps: [CapabilitySlot; 256],
}
```

**Memory Location**: Fixed virtual address at 0x8000_0000 (2GB)

**Population Flow**:
1. Kernel creates BootInfo structure in root_task.rs
2. Kernel allocates physical frame for boot info
3. Kernel populates all fields (untyped regions, devices, caps)
4. Kernel maps boot info at 0x8000_0000 in root task's VSpace
5. Root task reads boot info and passes to capability broker

### Capability Broker Architecture

**Responsibilities**:
- Centralized capability management
- Hide seL4-style complexity from applications
- Resource allocation (memory, devices, IPC endpoints)
- Capability delegation policies

**API Design** (to be implemented):
```rust
// Capability Broker API
pub fn allocate_memory(size: usize) -> Result<MemoryCap, Error>;
pub fn allocate_device(device_id: DeviceId) -> Result<DeviceCap, Error>;
pub fn create_endpoint() -> Result<EndpointCap, Error>;
pub fn spawn_component(elf_path: &str) -> Result<ProcessId, Error>;
```

### Memory Manager Architecture

**Responsibilities**:
- Physical frame allocation
- Virtual address space management
- Page table creation and mapping
- Memory accounting

**API Design** (to be implemented):
```rust
// Memory Manager API
pub fn alloc_frames(count: usize) -> Result<PhysAddr, Error>;
pub fn free_frames(addr: PhysAddr, count: usize);
pub fn create_vspace() -> Result<VSpaceId, Error>;
pub fn map_page(vspace: VSpaceId, vaddr: VirtAddr, paddr: PhysAddr, perms: Permissions);
```

---

## Technical Challenges

### Challenge 1: Boot Info Size
- **Issue**: BootInfo structure is ~40KB (128 untyped + 32 device + 256 cap slots)
- **Solution**: Limit to essential regions, use const arrays with compile-time size checks
- **Status**: ✅ Resolved - Structure verified < 64KB

### Challenge 2: Userspace Pointer Validation
- **Issue**: Root task needs to read boot info from kernel-mapped memory
- **Solution**: Map boot info page into root task's VSpace at known address
- **Status**: 📋 Planned - Will implement in boot info generation

### Challenge 3: Initial Capability Space
- **Issue**: How to represent initial capabilities in BootInfo
- **Solution**: Use CapabilitySlot array with type, address, and rights fields
- **Status**: ✅ Resolved - Design complete

---

## Testing Strategy

### Unit Tests
- [x] BootInfo creation and validation
- [x] UntypedRegion size calculations
- [ ] Boot info population logic
- [ ] Capability broker API
- [ ] Memory manager API

### Integration Tests
- [ ] Boot info passed correctly to userspace
- [ ] Capability broker allocates resources
- [ ] Memory manager maps pages
- [ ] Multi-component IPC (Chapter 5 deferred)

### System Tests
- [ ] Root task spawns capability broker
- [ ] Capability broker spawns memory manager
- [ ] Dynamic component loading (replace echo-server)
- [ ] Full IPC testing with real components

---

## Timeline

| Week | Tasks | Status |
|------|-------|--------|
| Week 1 | Boot info generation, Capability Broker skeleton | 🔄 In Progress |
| Week 2 | Memory Manager, Enhanced root task, IPC tests | 📋 Planned |

---

## Next Immediate Steps

1. ✅ Design boot info structure
2. 🔄 **Implement kernel-side boot info generation** (CURRENT)
3. Create capability broker skeleton
4. Implement capability broker API
5. Create memory manager skeleton
6. Implement memory manager API
7. Enhance root task to use services
8. Complete IPC integration tests

---

## Notes

- **Nushell Build System**: Deferred to post-Chapter 9 (user's request)
- **Existing Root Task**: Already functional with ELF loading and process spawning
- **Echo-Server**: Currently hardcoded, will be replaced with dynamic loading
- **Chapter 8 Verification**: Can be done after Chapter 9 Phase 1 for complete system testing

---

**Last Updated**: 2025-10-15
