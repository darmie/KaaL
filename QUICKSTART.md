# KaaL Quick Start

**Build seL4-based systems with the KaaL CLI**

## Prerequisites

- ✅ Docker Desktop installed and running (Mac users)
- ✅ Rust toolchain installed
- ✅ 10GB free disk space

## Quick Start

### Step 1: Install KaaL CLI

```bash
cargo install --path tools/kaal-compose
```

Or build from source:

```bash
cargo build --release -p kaal-compose
# The binary will be at target/release/kaal-compose
```

### Step 2: Create a New Project

```bash
kaal new my-system
cd my-system
```

This creates a minimal KaaL project with:
- `src/main.rs` - Your root task entry point
- `Cargo.toml` - Dependencies configured
- `Dockerfile` - Build environment
- `.kaal/config.toml` - Project configuration
- `README.md` - Basic usage instructions

Available templates:
- `minimal` - Basic seL4 root task (default)
- `driver` - Device driver template
- `system` - Complete system composition

```bash
kaal new my-driver --template driver
```

### Step 3: Build Your System

```bash
kaal build
```

This will:
- Auto-detect macOS and use Docker build
- Build seL4 kernel for ARM64
- Compile your root task
- Extract binary to `build/system.elf`

For release builds:
```bash
kaal build --release
```

### Step 4: Run in QEMU

```bash
kaal run
```

This will:
- Launch QEMU with your system
- Boot seL4 kernel
- Execute your root task

**To exit QEMU**: Press `Ctrl+A` then `X`

For debugging with GDB:
```bash
kaal run --debug
# In another terminal: gdb build/system.elf, then (gdb) target remote :1234
```

### Step 5: Development (optional)

Edit `src/main.rs` to implement your system logic:

```rust
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Your system initialization here

    // Example: Print to seL4 debug console
    unsafe {
        sel4_platform::adapter::seL4_DebugPutChar(b'H');
        sel4_platform::adapter::seL4_DebugPutChar(b'i');
        sel4_platform::adapter::seL4_DebugPutChar(b'\n');
    }

    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("wfi"); } }
}
```

Then rebuild and run:
```bash
kaal build && kaal run
```

---

## That's It! 🎉

You're now building real operating systems on your Mac Silicon!

**Next steps**:
- See [docs/BUILD_INSTRUCTIONS.md](docs/BUILD_INSTRUCTIONS.md) for advanced options
- See [docs/CROSS_PLATFORM.md](docs/CROSS_PLATFORM.md) for deployment targets
- See [docs/QEMU_READINESS.md](docs/QEMU_READINESS.md) for detailed QEMU info

---

## Troubleshooting

**Docker not running?**
```bash
# Start Docker Desktop from Applications
open -a Docker
```

**Build fails?**
```bash
# Clean and rebuild
docker system prune -a
./scripts/docker-build.sh
```

**QEMU won't start?**
```bash
# Check the build succeeded
ls -lh target/release/kaal-root-task
```

**Want to rebuild?**
```bash
# Full rebuild
./scripts/docker-build.sh
./scripts/docker-test.sh
```
