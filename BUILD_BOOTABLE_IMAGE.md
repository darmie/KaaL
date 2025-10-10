# Building a Bootable KaaL Image

Complete guide for building bootable seL4 images with KaaL's Rust-based elfloader.

## Current Status ✅

**The bootable image build system is WORKING and TESTED**

- ✅ seL4 kernel v13.0.0 builds successfully
- ✅ Rust elfloader boots successfully in QEMU
- ✅ Docker multi-stage build system complete
- ✅ QEMU ARM virt platform tested and verified
- ✅ Memory layout correct (DTB at 0x40000000, elfloader at 0x40100000)

## Quick Start

```bash
./tools/build-bootimage.sh --project examples/bootable-demo --lib libkaal_bootable_demo.a --clean --test
```

This command will:
1. Build seL4 kernel (v13.0.0)
2. Build your root task from `examples/bootable-demo`
3. Build Rust elfloader
4. Link everything into `bootimage.elf`
5. Test in QEMU automatically

## Build System Architecture

### Components

1. **[tools/build-bootimage.sh](tools/build-bootimage.sh)** - Main build script
2. **[tools/Dockerfile.bootimage](tools/Dockerfile.bootimage)** - Multi-stage Docker build
3. **[tools/bootimage.ld](tools/bootimage.ld)** - Linker script (CRITICAL FILE)
4. **[tools/test-qemu.sh](tools/test-qemu.sh)** - QEMU testing script

### Docker Multi-Stage Build Process

The build uses 8 Docker stages:

```
Stage 1: builder-base      → Base environment (Rust, ARM64 GCC, build tools)
Stage 2: kernel-builder     → Build seL4 kernel v13.0.0
Stage 3: elfloader-builder  → Build Rust elfloader
Stage 4: roottask-builder   → Build your root task
Stage 5: builder-tool       → Build elfloader-builder (future use)
Stage 6: assembler          → Link everything together
Stage 7: qemu-test          → Optional QEMU boot test
Stage 8: output             → Extract final bootimage.elf
```

## Memory Layout (QEMU ARM virt)

```
Physical Address Range    | Description
--------------------------|----------------------------------
0x00000000 - 0x08000000   | Flash/ROM (128MB)
0x09000000 - 0x09001000   | UART PL011 device
0x0A000000 - 0x40000000   | Other devices (GIC, RTC, GPIO)
0x40000000 - 0x40100000   | Device Tree Blob (DTB) - 1MB
0x40100000 - ...          | Elfloader load address ← START HERE
```

**Why 0x40100000?**
- QEMU places DTB at RAM base (0x40000000)
- DTB occupies 1MB (0x40000000-0x40100000)
- Elfloader must load AFTER DTB to avoid overlap

### Elfloader Internal Layout

```
Address               | Section         | Description
----------------------|-----------------|---------------------------
0x40100000           | .text.boot      | ARM64 entry point (_start)
0x40100xxx           | .text           | Elfloader code
0x401xxxxx           | .rodata         | Read-only data
0x40117000 (aligned) | .kernel_elf     | Embedded kernel ELF (~173KB)
0x40143000 (aligned) | .roottask_data  | Embedded root task (~785KB)
0x402xxxxx           | .data           | Initialized data
0x402xxxxx           | .bss            | Zero-initialized data
0x402xxxxx - 0x403xxx| Stack (1MB)     | Grows downward
```

## Build Script Usage

### Basic Build

```bash
./tools/build-bootimage.sh \
    --project examples/bootable-demo \
    --lib libkaal_bootable_demo.a
```

### Options

- `--project PATH` - Path to your root task project (required)
- `--lib NAME` - Root task library name (required)
- `--clean` - Clean build workspace before starting
- `--keep-workspace` - Preserve build artifacts in `.build-workspace/`
- `--test` - Run QEMU test after build
- `--output PATH` - Custom output path (default: `./bootimage.elf`)

### Examples

```bash
# Clean build with QEMU test
./tools/build-bootimage.sh --project examples/bootable-demo --lib libkaal_bootable_demo.a --clean --test

# Custom output location
./tools/build-bootimage.sh --project examples/my-app --lib libmy_app.a --output /tmp/my-boot.elf

# Keep build artifacts for inspection
./tools/build-bootimage.sh --project examples/bootable-demo --lib libkaal_bootable_demo.a --keep-workspace
```

## Testing in QEMU

### Using build script

```bash
./tools/build-bootimage.sh --project examples/bootable-demo --lib libkaal_bootable_demo.a --test
```

### Manual testing

```bash
./tools/test-qemu.sh bootimage.elf
```

### Direct QEMU command

```bash
qemu-system-aarch64 \
    -machine virt,virtualization=on,highmem=off,secure=off \
    -cpu cortex-a53 \
    -m 512M \
    -nographic \
    -kernel bootimage.elf
```

**Exit QEMU**: Press `Ctrl-A` then `X`

## Expected Boot Output

When the bootimage boots successfully in QEMU, you should see:

```
#!@═══════════════════════════════════════════════════════════
  KaaL Elfloader v0.1.0 - Rust-based seL4 Boot Loader
═══════════════════════════════════════════════════════════

DTB address: 0x40000000
Device tree parsed successfully
Model: linux,dummy-virt
Memory region: 0x40000000 - 0x60000000 (512 MB)

Loading images...
  Kernel: 0x40117000 - 0x40142538 (173 KB)
  User:   0x40143000 - 0x40207508 (785 KB)
Copying kernel to physical address 0x40000000...
Images loaded successfully!

Setting up page tables...
MMU enabled successfully

Jumping to seL4 kernel at 0x40000000...
```

### Debug Checkpoints

The elfloader outputs debug characters at key boot stages:

- `#` - First Rust code executing (_start_rust)
- `!` - UART hardware initialized
- `@` - UART writer ready
- Full banner - Complete initialization successful

If you only see `#` or `#!` and then nothing, the boot has failed at an early stage.

## Creating Your Own Root Task

Your root task must be a bare-metal ARM64 static library:

### Project Structure

```
your-project/
├── Cargo.toml
└── src/
    └── lib.rs
```

### Cargo.toml

```toml
[package]
name = "your-project"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib"]

[profile.release]
panic = "abort"
lto = true
opt-level = "z"

[dependencies]
# Your dependencies here (must be no_std compatible)
```

### src/lib.rs

```rust
#![no_std]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Your initialization code here
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

### Build and Test

```bash
./tools/build-bootimage.sh \
    --project your-project \
    --lib libyour_project.a \
    --clean --test
```

## Linker Script Details

**Location**: [tools/bootimage.ld](tools/bootimage.ld)

This is the **ONLY** linker script used by the build system.

**IMPORTANT**: Do NOT edit `runtime/elfloader/linker.ld` - that file has been removed. See [runtime/elfloader/LINKER_SCRIPT.md](runtime/elfloader/LINKER_SCRIPT.md) for why.

The linker script defines:
- Entry point: `_start` (in `.text.boot` section)
- Load address: `0x40100000`
- Embedded kernel section: `.kernel_elf`
- Embedded root task section: `.roottask_data`
- Stack size: 1MB
- Symbol exports for memory boundaries

## Troubleshooting

### Issue: "DTB overlap" or "ROM regions are overlapping"

**Cause**: Elfloader load address conflicts with DTB.

**Solution**: Ensure [tools/bootimage.ld](tools/bootimage.ld) has `. = 0x40100000;`

### Issue: No QEMU output

**Possible causes**:
1. Wrong QEMU machine configuration
2. Elfloader crashed before UART init
3. Docker build cache used stale files

**Solutions**:
```bash
# Clear Docker cache and rebuild
docker builder prune -af
./tools/build-bootimage.sh --project examples/bootable-demo --lib libkaal_bootable_demo.a --clean

# Check QEMU command matches configuration (see above)
```

### Issue: Entry point shows wrong address

**Cause**: Docker using cached layers with old linker script.

**Solution**:
```bash
docker builder prune -af
./tools/build-bootimage.sh --clean --project examples/bootable-demo --lib libkaal_bootable_demo.a
readelf -h bootimage.elf | grep "Entry point"
# Should show: Entry point address: 0x40100000
```

### Issue: Build fails with "PROJECT_PATH is required"

**Cause**: Missing required arguments.

**Solution**: Always provide both `--project` and `--lib`:
```bash
./tools/build-bootimage.sh --project examples/bootable-demo --lib libkaal_bootable_demo.a
```

## Advanced Usage

### Custom Memory Layout

To change the elfloader load address:

1. Edit [tools/bootimage.ld](tools/bootimage.ld) line 12: `. = 0x40100000;`
2. Update DTB address in [runtime/elfloader/src/arch/aarch64.rs](runtime/elfloader/src/arch/aarch64.rs) if needed
3. Rebuild with `--clean`

### Inspecting Build Artifacts

```bash
# Keep workspace to inspect artifacts
./tools/build-bootimage.sh --project examples/bootable-demo --lib libkaal_bootable_demo.a --keep-workspace

# Check workspace contents
ls -lh .build-workspace/
cat .build-workspace/build-info.txt

# Inspect bootimage
readelf -h bootimage.elf
readelf -l bootimage.elf  # Show program headers
readelf -S bootimage.elf  # Show sections
```

### Debugging with GDB

```bash
# Terminal 1: Start QEMU with GDB server
qemu-system-aarch64 \
    -machine virt,virtualization=on,highmem=off,secure=off \
    -cpu cortex-a53 \
    -m 512M \
    -nographic \
    -kernel bootimage.elf \
    -s -S  # Wait for GDB on port 1234

# Terminal 2: Connect GDB
aarch64-linux-gnu-gdb bootimage.elf
(gdb) target remote :1234
(gdb) b _start
(gdb) c
```

## File Organization

```
kaal/
├── tools/
│   ├── build-bootimage.sh      ← Main build script
│   ├── test-qemu.sh            ← QEMU test script
│   ├── bootimage.ld            ← Linker script (CRITICAL)
│   └── Dockerfile.bootimage    ← Multi-stage build
├── runtime/
│   ├── elfloader/
│   │   ├── src/
│   │   │   ├── lib.rs          ← Main entry point
│   │   │   ├── arch/aarch64.rs ← ARM64 boot code
│   │   │   ├── boot.rs         ← Image loading
│   │   │   ├── mmu.rs          ← Page table setup
│   │   │   └── uart.rs         ← Debug output
│   │   ├── Cargo.toml
│   │   └── LINKER_SCRIPT.md    ← Explains linker script location
│   └── elfloader-builder/      ← Future tool (not yet used)
└── examples/
    └── bootable-demo/          ← Example root task
        ├── src/lib.rs
        └── Cargo.toml
```

## Build System Internals

### How Images Are Embedded

The kernel and root task are embedded as ELF sections using `objcopy`:

```bash
# Embed kernel
aarch64-linux-gnu-objcopy -I binary -O elf64-littleaarch64 -B aarch64 \
    --rename-section .data=.kernel_elf,alloc,load,readonly,data,contents \
    kernel.elf kernel_embed.o

# Embed root task
aarch64-linux-gnu-objcopy -I binary -O elf64-littleaarch64 -B aarch64 \
    --rename-section .data=.roottask_data,alloc,load,readonly,data,contents \
    root-task.a roottask_embed.o
```

These are then linked with the elfloader using [tools/bootimage.ld](tools/bootimage.ld).

### Accessing Embedded Images in Rust

The elfloader accesses embedded images via linker symbols:

```rust
extern "C" {
    static __kernel_image_start: u8;
    static __kernel_image_end: u8;
    static __user_image_start: u8;
    static __user_image_end: u8;
}

let kernel_start = &__kernel_image_start as *const u8 as usize;
let kernel_end = &__kernel_image_end as *const u8 as usize;
```

## Requirements

### Host Requirements
- Docker
- Bash 4.0+
- ~2GB disk space for Docker images

### Container Requirements
All automatically installed in Docker:
- Rust nightly (1.75+)
- ARM64 GCC cross-compiler (aarch64-linux-gnu-gcc)
- CMake, Ninja (for seL4 kernel)
- Python3 with seL4 dependencies
- QEMU (for testing)

## Next Steps

1. **Build your first bootable image**:
   ```bash
   ./tools/build-bootimage.sh --project examples/bootable-demo --lib libkaal_bootable_demo.a --test
   ```

2. **Create your own root task** (see "Creating Your Own Root Task" above)

3. **Integrate with KaaL framework**:
   - Use `cap_broker` for capability management
   - Implement proper IPC handlers
   - Add device drivers using DDDK

4. **Deploy to hardware** (future):
   - Raspberry Pi 4
   - Odroid boards
   - Custom ARM64 platforms

## References

- [seL4 Manual](https://sel4.systems/Info/Docs/seL4-manual.pdf)
- [seL4 QEMU ARM virt](https://docs.sel4.systems/Hardware/QEMUArmVirt/)
- [ARM64 Boot Protocol](https://www.kernel.org/doc/Documentation/arm64/booting.txt)
- [QEMU ARM virt Platform](https://www.qemu.org/docs/master/system/arm/virt.html)
- [runtime/elfloader/LINKER_SCRIPT.md](runtime/elfloader/LINKER_SCRIPT.md) - Linker script documentation

---

**Build System Version**: 1.0
**Last Updated**: 2025 (after QEMU boot fixes)
**Minimum seL4 Version**: 13.0.0
**Rust Version**: nightly (1.75+)
