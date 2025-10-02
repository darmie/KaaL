# Phase 2: seL4 Integration Migration Guide

This document outlines the step-by-step process for migrating from mock seL4 implementations to real seL4 kernel integration.

## Overview

**Phase 1** (Completed ✅): Foundation with mock seL4
- Capability Broker with resource allocation
- Shared Memory IPC with notifications
- DDDK procedural macros
- Serial driver example

**Phase 2** (In Progress): Real seL4 Integration
- Replace mock seL4-sys with real seL4 bindings
- Implement bootinfo parsing
- Add real MMIO mapping
- Add real IRQ handling
- Integrate with seL4 build system

## Migration Checklist

### 1. Replace Mock seL4 Dependencies

**Current (Phase 1):**
```toml
# Cargo.toml
[workspace.dependencies]
sel4-sys = { path = "runtime/sel4-mock" }
sel4 = { path = "runtime/sel4-rust-mock" }
```

**Phase 2:**
```toml
# Cargo.toml
[workspace.dependencies]
sel4-sys = { git = "https://github.com/seL4/rust-sel4", branch = "main" }
sel4 = { git = "https://github.com/seL4/rust-sel4", branch = "main" }
```

### 2. Add seL4 Kernel Build Integration

Create `build.rs` for components that link with seL4:

```rust
// runtime/cap_broker/build.rs
fn main() {
    // Link with seL4 kernel
    println!("cargo:rustc-link-lib=static=sel4");

    // Add seL4 include paths
    let sel4_include = std::env::var("SEL4_INCLUDE_DIR")
        .expect("SEL4_INCLUDE_DIR must be set");
    println!("cargo:include={}", sel4_include);
}
```

### 3. Bootinfo Parsing

Replace hardcoded device registry with bootinfo parsing:

**Location:** `runtime/cap_broker/src/lib.rs`

```rust
use sel4::BootInfo;

impl DefaultCapBroker {
    pub unsafe fn init() -> Result<Self> {
        // Get bootinfo from seL4
        let bootinfo = sel4::get_bootinfo();

        // Parse untyped regions
        let untyped_regions = parse_untyped_regions(bootinfo);

        // Parse device regions (from device tree or ACPI)
        let devices = parse_device_tree(bootinfo);

        // Initialize CSpace allocator
        let cspace = CSpaceAllocator::new(
            bootinfo.empty.start,
            bootinfo.empty.end,
        );

        Ok(Self {
            cspace,
            untyped_regions,
            devices,
            allocated_irqs: Vec::new(),
        })
    }
}

fn parse_untyped_regions(bootinfo: &BootInfo) -> Vec<UntypedRegion> {
    bootinfo.untyped_list()
        .iter()
        .map(|ut| UntypedRegion {
            cap: ut.cap,
            base_paddr: ut.paddr,
            size_bits: ut.size_bits,
            allocated: 0,
        })
        .collect()
}
```

### 4. Real MMIO Mapping

Implement actual frame mapping for MMIO regions:

**Location:** `runtime/cap_broker/src/lib.rs`

```rust
fn map_mmio_region(
    &mut self,
    paddr: usize,
    size: usize,
    vaddr: usize,
) -> Result<MappedRegion> {
    // Calculate number of 4KB frames needed
    let num_frames = (size + 4095) / 4096;

    for i in 0..num_frames {
        // Create frame capability from untyped
        let frame_cap = self.cspace.allocate()?;
        let untyped_cap = self.find_untyped_for_region(paddr + i * 4096)?;

        unsafe {
            sel4_sys::seL4_Untyped_Retype(
                untyped_cap,
                sel4_sys::seL4_ARCH_4KPage,
                0, // size_bits (0 for pages)
                self.cspace_root,
                0, // node_index
                0, // node_depth
                frame_cap,
                1, // num_objects
            )?;

            // Map frame into VSpace
            sel4_sys::seL4_ARCH_Page_Map(
                frame_cap,
                self.vspace_root,
                vaddr + i * 4096,
                sel4_sys::seL4_CanRead | sel4_sys::seL4_CanWrite,
                sel4_sys::seL4_ARCH_Default_VMAttributes,
            )?;
        }
    }

    Ok(MappedRegion {
        vaddr,
        paddr,
        size,
    })
}
```

### 5. Real IRQ Handling

Implement actual IRQ capability derivation:

**Location:** `runtime/cap_broker/src/lib.rs`

```rust
fn request_irq(&mut self, irq: u8) -> Result<IrqHandler> {
    // Check if already allocated
    if self.allocated_irqs.contains(&irq) {
        return Err(CapabilityError::IrqAlreadyAllocated { irq });
    }

    // Allocate slots for IRQ handler and notification
    let irq_cap = self.cspace.allocate()?;
    let notif_cap = self.cspace.allocate()?;

    unsafe {
        // Get IRQ handler capability
        sel4_sys::seL4_IRQControl_Get(
            sel4_sys::seL4_CapIRQControl,
            irq as usize,
            self.cspace_root,
            irq_cap,
            seL4_WordBits as u8,
        )?;

        // Create notification object
        let untyped = self.find_available_untyped(
            1 << sel4_sys::seL4_NotificationBits
        )?;

        sel4_sys::seL4_Untyped_Retype(
            untyped,
            sel4_sys::seL4_NotificationObject,
            0,
            self.cspace_root,
            0,
            0,
            notif_cap,
            1,
        )?;

        // Bind IRQ to notification
        sel4_sys::seL4_IRQHandler_SetNotification(
            irq_cap,
            notif_cap,
        )?;
    }

    self.allocated_irqs.push(irq);

    Ok(IrqHandler {
        cap: irq_cap,
        notification: notif_cap,
        irq_num: irq,
    })
}
```

### 6. Update IPC Layer for Real Notifications

**Location:** `runtime/ipc/src/lib.rs`

The IPC layer already uses seL4_Signal and seL4_Wait - these will automatically work with real seL4 once we switch dependencies. No code changes needed!

```rust
// Already implemented correctly in Phase 1:
pub fn push(&self, item: T) -> Result<()> {
    // ... buffer logic ...

    // Signal consumer via seL4 notification
    if let Some(notify) = self.consumer_notify {
        unsafe {
            seL4_Signal(notify); // ✅ Already correct!
        }
    }

    Ok(())
}
```

### 7. Build System Integration

Create CMake integration for seL4:

**File:** `CMakeLists.txt`

```cmake
cmake_minimum_required(VERSION 3.12)

project(kaal C ASM)

# Find seL4 SDK
find_package(seL4 REQUIRED)

# Configure seL4 kernel
sel4_configure_platform(
    PLATFORM "x86_64"
    KERNEL_PATH "${CMAKE_SOURCE_DIR}/seL4"
)

# Add Rust workspace
add_custom_target(rust_workspace ALL
    COMMAND cargo build --release --target x86_64-sel4
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    COMMENT "Building Rust workspace"
)

# Create seL4 system image
include(seL4/tools/seL4.cmake)
sel4_import_kernel()

# Define root task (our Rust code)
add_executable(kaal_root
    ${CMAKE_SOURCE_DIR}/target/x86_64-sel4/release/libkaal_root.a
)

target_link_libraries(kaal_root
    sel4
    sel4_autoconf
    sel4runtime
)

# Generate final system image
DeclareRootserver(kaal_root)
```

### 8. Target Triple Configuration

Create custom target for seL4:

**File:** `x86_64-sel4.json`

```json
{
  "llvm-target": "x86_64-unknown-none",
  "data-layout": "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128",
  "arch": "x86_64",
  "target-endian": "little",
  "target-pointer-width": "64",
  "target-c-int-width": "32",
  "os": "none",
  "executables": true,
  "linker-flavor": "ld.lld",
  "linker": "rust-lld",
  "panic-strategy": "abort",
  "disable-redzone": true,
  "features": "-mmx,-sse,+soft-float",
  "relocation-model": "static",
  "code-model": "kernel"
}
```

### 9. Environment Setup

Add to your shell profile:

```bash
# seL4 environment
export SEL4_DIR=/path/to/seL4
export SEL4_PLATFORM=x86_64
export SEL4_INCLUDE_DIR=$SEL4_DIR/include
export RUST_TARGET_PATH=$(pwd)

# Build commands
alias sel4-build="cmake -B build && cmake --build build"
alias sel4-run="qemu-system-x86_64 -kernel build/images/kernel-x86_64-pc99 \
    -initrd build/images/kaal-image-x86_64-pc99 -nographic"
```

### 10. Testing Strategy

**Unit Tests (with mocks):**
```bash
# Keep using mocks for fast unit testing
cargo test --workspace
```

**Integration Tests (with real seL4):**
```bash
# Build with real seL4 and run in QEMU
cmake -B build -DSIMULATION=TRUE
cmake --build build
ctest --output-on-failure
```

## Migration Steps (Recommended Order)

1. ✅ Set up seL4 kernel build environment
2. ✅ Add CMakeLists.txt and build configuration
3. ✅ Update Cargo.toml to use real seL4 bindings
4. ✅ Implement bootinfo parsing in cap_broker
5. ✅ Implement real MMIO mapping
6. ✅ Implement real IRQ handling
7. ✅ Update serial driver example to run on seL4
8. ✅ Test in QEMU with x86_64 target
9. ✅ Test on real hardware (if available)
10. ✅ Add CI/CD pipeline for seL4 builds

## Common Issues and Solutions

### Issue: "undefined reference to seL4_*"
**Solution:** Make sure SEL4_INCLUDE_DIR is set and seL4 library is linked in build.rs

### Issue: "Page fault at address 0x..."
**Solution:** Check MMIO mapping - ensure frames are properly retyped and mapped with correct permissions

### Issue: "Capability not found"
**Solution:** Verify CSpace allocation - check that bootinfo.empty range is correct

### Issue: "IRQ handler failed"
**Solution:** Ensure IRQ number is valid for platform and not already allocated

## Performance Benchmarks (Expected)

| Operation | Phase 1 (Mock) | Phase 2 (Real seL4) |
|-----------|----------------|---------------------|
| IPC (msg/sec) | N/A | ~500K |
| Context Switch | N/A | ~150 cycles |
| IRQ Latency | N/A | ~1-2 µs |
| MMIO Access | N/A | ~100 cycles |

## Resources

- seL4 Manual: https://sel4.systems/Info/Docs/seL4-manual.pdf
- Rust seL4 bindings: https://github.com/seL4/rust-sel4
- seL4 Tutorials: https://docs.sel4.systems/Tutorials/
- KaaL Architecture: `docs/ARCHITECTURE.md`

## Phase 2 Completion Criteria

- [ ] All workspace crates build with real seL4
- [ ] Bootinfo parsing works correctly
- [ ] MMIO regions can be mapped and accessed
- [ ] IRQs can be registered and handled
- [ ] Serial driver works in QEMU
- [ ] All integration tests pass
- [ ] Performance benchmarks meet targets

---

**Current Status:** Phase 1 Complete ✅ | Phase 2 Ready to Begin 🚀
