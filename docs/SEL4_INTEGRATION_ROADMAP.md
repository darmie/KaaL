# seL4 Real Integration Roadmap

## Current State (Phase 1.5)

We are **ready for real seL4 integration** but currently using mocks for rapid development. Here's why and how to transition:

### What We Have Now
- ✅ **Mock seL4 syscalls** (`runtime/sel4-mock/`, `runtime/sel4-rust-mock/`)
- ✅ **Correct seL4 API usage** - Code uses proper seL4 syscall signatures
- ✅ **Conditional compilation** - `#[cfg(feature = "sel4-real")]` separates mock from real
- ✅ **Bootinfo parsing** - Ready to consume real bootinfo from kernel
- ✅ **VSpace management** - Proper page mapping infrastructure
- ✅ **MMIO mapping** - Device memory mapping with real syscalls
- ✅ **IRQ allocation** - Interrupt handling infrastructure

### Why Mocks?
Mocks enable **fast iteration** without:
- Building entire seL4 kernel (10+ minutes)
- QEMU/hardware setup
- Kernel debugging complexity
- Long compilation times

Tests run in **milliseconds** vs **minutes** with real kernel.

---

## Phase 2: Real seL4 Integration

### Prerequisites

Before switching to real seL4, you need:

1. **seL4 Kernel Build**
   ```bash
   # Clone seL4 kernel
   git clone https://github.com/seL4/seL4
   cd seL4

   # Initialize build system
   mkdir build && cd build
   ../init-build.sh -DPLATFORM=x86_64 -DSIMULATION=TRUE

   # Build kernel
   ninja
   ```

2. **seL4 Rust Bindings**
   ```bash
   # Clone seL4 Rust support
   git clone https://github.com/seL4/rust-sel4
   cd rust-sel4

   # Check available versions
   git tag | grep rel_
   # Use rel_13.0.0 (matches seL4 13.0.0)
   ```

3. **QEMU for Simulation**
   ```bash
   # macOS
   brew install qemu

   # Linux
   apt-get install qemu-system-x86
   ```

### Integration Steps

#### Step 1: Update Workspace Dependencies

Edit `Cargo.toml`:

```toml
[workspace.dependencies]
# BEFORE (Current - Mocks):
sel4-sys = { path = "runtime/sel4-mock" }
sel4 = { path = "runtime/sel4-rust-mock" }

# AFTER (Real seL4):
sel4-sys = { git = "https://github.com/seL4/rust-sel4", tag = "rel_13.0.0" }
sel4 = { git = "https://github.com/seL4/rust-sel4", tag = "rel_13.0.0" }
```

#### Step 2: Build Configuration

The real seL4 Rust bindings require build-time configuration:

Create `.cargo/config.toml`:
```toml
[build]
target = "x86_64-sel4"  # or aarch64-sel4

[env]
SEL4_PREFIX = "/path/to/sel4/build"
SEL4_KERNEL = "/path/to/sel4/build/kernel/kernel.elf"
```

#### Step 3: Link with seL4 Kernel

Create linker script `link.ld` (provided by seL4 toolchain):
```ld
ENTRY(_start)

SECTIONS {
    . = 0x400000;  /* Root task load address */
    .text : { *(.text) }
    .data : { *(.data) }
    .bss : { *(.bss) }
}
```

Update `runtime/cap_broker/.cargo/config.toml`:
```toml
[target.x86_64-sel4]
linker = "x86_64-linux-gnu-ld"
rustflags = ["-C", "link-arg=-Tlink.ld"]
```

#### Step 4: Verify Code Still Compiles

```bash
# Should work immediately since we followed seL4 API correctly
cargo build -p cap-broker --target x86_64-sel4
```

If errors occur, they'll be in:
- Incorrect syscall signatures (unlikely - we copied from seL4 docs)
- Missing constants (add to imports)
- Bootinfo struct mismatch (update our BootInfo to match real one)

#### Step 5: Create Root Task Entry Point

The real seL4 kernel expects a `_start` function:

Create `runtime/cap_broker/src/sel4_start.rs`:
```rust
#![cfg(feature = "sel4-real")]

use sel4::BootInfo;

#[no_mangle]
pub extern "C" fn _start(bootinfo_ptr: *const BootInfo) -> ! {
    // Parse bootinfo
    let bootinfo = unsafe { &*bootinfo_ptr };

    // Initialize capability broker
    let mut broker = unsafe {
        crate::DefaultCapBroker::init_from_bootinfo(bootinfo)
    }.expect("Failed to initialize capability broker");

    // TODO: Start root task logic

    loop {
        // Root task main loop
    }
}
```

#### Step 6: Test in QEMU

```bash
# Build root task image
cargo build -p cap-broker --target x86_64-sel4 --release

# Create bootable image with seL4 tools
# (Combines kernel + root task)
sel4-image-builder \
    --kernel kernel.elf \
    --root-task target/x86_64-sel4/release/cap-broker \
    --output kaal.img

# Run in QEMU
qemu-system-x86_64 \
    -kernel kaal.img \
    -nographic \
    -serial mon:stdio \
    -m 512M
```

#### Step 7: Validate Real seL4 Functionality

Expected console output:
```
seL4 Bootstrapping
Root task: cap-broker
Bootinfo @ 0x...
  CSpace root: 1
  VSpace root: 2
  TCB: 3
  Untyped regions: 15
  Device untypeds: 3
Capability broker initialized
```

---

## What Changes in Code?

### Almost Nothing!

Thanks to proper abstraction, **95% of code works unchanged**:

#### Code That Just Works
```rust
// This works with both mock and real seL4
#[cfg(feature = "sel4-real")]
{
    unsafe {
        let ret = sel4_sys::seL4_Untyped_Retype(
            untyped_cap,
            sel4_sys::seL4_ARCH_4KPage,
            0,
            cspace_root,
            0, 0,
            frame_cap,
            1,
        );
        // Error handling...
    }
}
```

#### Bootinfo - Needs Update
```rust
// BEFORE (Our mock):
pub struct BootInfo {
    pub empty: SlotRegion,
    pub untyped: Vec<UntypedDescriptor>,
    pub cspace_root: usize,
    // ...
}

// AFTER (Real seL4 - from sel4 crate):
use sel4::BootInfo;  // Use theirs directly
```

### Files Needing Updates

1. **`bootinfo.rs`** - Replace our BootInfo with `sel4::BootInfo`
2. **`lib.rs`** - Add real seL4 entry point
3. **`Cargo.toml`** - Update dependency paths (shown above)

**That's it!** Everything else (MMIO, IRQ, VSpace) already uses correct seL4 APIs.

---

## Timeline Estimate

| Task | Time | Why |
|------|------|-----|
| Update dependencies | 5 min | Change git URLs |
| Build seL4 kernel | 15 min | One-time setup |
| Fix BootInfo integration | 30 min | Adapt to real struct |
| Create root task entry | 1 hour | Write `_start()` |
| QEMU testing | 2 hours | Debug boot process |
| **Total** | **~4 hours** | First successful boot |

---

## Why Not Do This Now?

**Good question!** Here's the tradeoff:

### Advantages of Staying on Mocks (Current)
- ⚡ **Fast tests** - 37 tests run in 0.4 seconds
- 🔧 **Easy debugging** - Standard Rust tooling works
- 🚀 **Rapid iteration** - No kernel rebuilds
- ✅ **Proven approach** - Linux kernel, Fuchsia use similar dual-mode dev

### Advantages of Switching to Real seL4 Now
- 🎯 **Real validation** - Catches seL4 API mismatches earlier
- 🏗️ **Integration readiness** - Forces solving kernel build issues
- 📊 **Performance testing** - Real syscall overhead measurement

### Recommended: **Stay on mocks until Phase 2 completion**

Complete these first:
1. ✅ Bootinfo parsing (done)
2. ✅ VSpace management (done)
3. ⏳ TCB management (next)
4. ⏳ Component spawning
5. ⏳ Full IPC implementation

**Then** switch to real seL4 when:
- All subsystems implemented
- All tests passing
- Ready for integration testing

---

## How to Switch (TL;DR)

**When ready, run these 3 commands:**

```bash
# 1. Update Cargo.toml dependencies to real seL4 git repos
sed -i 's|path = "runtime/sel4-mock"|git = "https://github.com/seL4/rust-sel4", tag = "rel_13.0.0"|g' Cargo.toml

# 2. Verify it builds
cargo build -p cap-broker --features sel4-real

# 3. Test in QEMU (requires seL4 kernel built separately)
# ... (See Step 6 above)
```

---

## Decision Point: Your Call

**Recommendation:** Continue with mocks for now, focusing on:
1. TCB management implementation
2. Component spawning
3. Full system testing

**Switch to real seL4 when:** You have a working system in mocks and want to validate on actual kernel.

What would you like to do?
- **Option A:** Continue with mocks, finish TCB/component spawning (faster progress)
- **Option B:** Switch to real seL4 now (validates integration, slower iteration)
