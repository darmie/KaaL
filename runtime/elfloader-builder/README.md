# KaaL Elfloader Builder

Build tool for packaging seL4 kernel and KaaL root task into a bootable image.

## Overview

This tool is the **host-side** component of KaaL's boot system. It runs during the build process to combine the elfloader, kernel, and root task into a single bootable image.

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                   Build Process (Host)                       │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  Step 1: Build Components                                    │
│  ┌────────────────┐  ┌────────────┐  ┌──────────────┐      │
│  │ Elfloader      │  │ seL4       │  │ Root Task    │      │
│  │ (Rust)         │  │ Kernel     │  │ (Rust)       │      │
│  └────────┬───────┘  └──────┬─────┘  └──────┬───────┘      │
│           │                  │                │              │
│           ▼                  ▼                ▼              │
│    elfloader.elf      kernel.elf       root-task.elf        │
│                                                               │
│  Step 2: kaal-elfloader-builder                             │
│  ┌──────────────────────────────────────────────────┐       │
│  │ Parse ELF files                                  │       │
│  │ Extract loadable segments                        │       │
│  │ Calculate physical addresses                     │       │
│  │ Serialize with postcard                          │       │
│  │ Patch elfloader with payload                     │       │
│  └───────────────────────┬──────────────────────────┘       │
│                          ▼                                   │
│                  bootimage.elf                               │
│                  (Final bootable image)                      │
│                                                               │
└───────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                   Runtime (Target ARM64)                     │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  Elfloader boots:                                            │
│  1. Deserialize embedded payload                            │
│  2. Load kernel to {:#x}                                     │
│  3. Load root task to {:#x}                                  │
│  4. Setup MMU + page tables                                  │
│  5. Jump to seL4 kernel                                      │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

## Usage

### Basic Command

```bash
kaal-elfloader-builder \
  --loader runtime/elfloader/target/aarch64-unknown-none/release/libkaal_elfloader.a \
  --kernel /path/to/kernel.elf \
  --app /path/to/root-task.elf \
  --out bootimage.elf
```

### Custom Addresses

```bash
kaal-elfloader-builder \
  --loader elfloader.elf \
  --kernel kernel.elf \
  --app root-task.elf \
  --out bootimage.elf \
  --kernel-paddr 0x40000000 \
  --app-offset 0x400000
```

### Arguments

| Argument | Required | Default | Description |
|----------|----------|---------|-------------|
| `--loader` | Yes | - | Path to elfloader ELF |
| `--kernel` | Yes | - | Path to seL4 kernel ELF |
| `--app` | Yes | - | Path to root task ELF |
| `--out` | Yes | - | Output bootable image path |
| `--kernel-paddr` | No | `0x40000000` | Kernel physical load address |
| `--app-offset` | No | `0x200000` | Offset between kernel and app |

## Payload Format

The tool creates a serialized payload using `postcard`:

```rust
struct Payload {
    kernel_regions: Vec<Region>,
    kernel_entry: usize,
    user_regions: Vec<Region>,
    user_entry: usize,
    total_data_size: usize,
}

struct Region {
    paddr: usize,       // Where to load in physical memory
    vaddr: usize,       // Virtual address (from ELF)
    size: usize,        // Total size (including BSS)
    data_offset: usize, // Offset in payload data
    data_size: usize,   // Actual data size (< size if BSS)
}
```

**Payload Structure**:
```
┌─────────────────────────────────────────────────┐
│ Postcard-serialized Payload metadata           │
│ (variable size, typically <1KB)                 │
├─────────────────────────────────────────────────┤
│ Kernel segment 1 data                           │
├─────────────────────────────────────────────────┤
│ Kernel segment 2 data                           │
├─────────────────────────────────────────────────┤
│ ...                                             │
├─────────────────────────────────────────────────┤
│ User segment 1 data                             │
├─────────────────────────────────────────────────┤
│ User segment 2 data                             │
├─────────────────────────────────────────────────┤
│ ...                                             │
└─────────────────────────────────────────────────┘
```

## How It Works

### 1. ELF Parsing

Parses kernel and app ELF files to extract:
- Entry point
- Loadable segments (PT_LOAD)
- Physical/virtual addresses
- Segment sizes (file vs memory)

### 2. Address Calculation

```
Kernel: 0x40000000 (default, configurable)
├─ Segment 1: 0x40000000
├─ Segment 2: 0x40100000
└─ ...

User App: Kernel end + 2MB offset
├─ Segment 1: calculated
├─ Segment 2: calculated
└─ ...
```

### 3. Serialization

Uses `postcard` (compact binary format) to serialize metadata.
Appends all segment data after metadata.

### 4. Elfloader Patching

**Current Implementation** (v0.1.0):
- Writes payload to separate `.payload` file
- **TODO**: Directly patch elfloader binary

**Future** (v0.2.0):
- Use `object` crate to modify elfloader ELF
- Add new `.rodata.payload` section
- Update `PAYLOAD_START` and `PAYLOAD_SIZE` symbols
- Write final bootable ELF

## Example Output

```
═══════════════════════════════════════════════════════════
  KaaL Elfloader Builder v0.1.0
═══════════════════════════════════════════════════════════

Configuration:
  Loader: elfloader.elf
  Kernel: kernel.elf
  App:    root-task.elf
  Output: bootimage.elf
  Kernel paddr: 0x40000000

Parsing ELF: kernel.elf
  Entry point: 0x40000000
  Program headers: 2
  Load segment:
    Phys: 0x40000000 - 0x40120000
    Virt: 0xffffff8000000000 - 0xffffff8000120000
    File size: 0x115000, Mem size: 0x120000
    BSS: 11264 bytes to be zeroed
  Extracted 2 regions, 1179648 bytes total

User app paddr: 0x40322000

Parsing ELF: root-task.elf
  Entry point: 0x400000
  Program headers: 3
  Load segment:
    Phys: 0x40322000 - 0x40450000
    Virt: 0x400000 - 0x52e000
    File size: 0x125000, Mem size: 0x12e000
  Extracted 3 regions, 1232896 bytes total

Payload summary:
  Kernel entry: 0x40000000
  User entry:   0x400000
  Kernel range: 0x40000000 - 0x40120000
  User range:   0x40322000 - 0x40450000
  Total data:   2412544 bytes

Serializing payload...
  Metadata: 256 bytes
  Complete payload: 2412800 bytes

✓ Payload written to: bootimage.payload

Next steps:
  1. Link payload into elfloader
  2. Create final bootable ELF

═══════════════════════════════════════════════════════════
```

## Integration with Build System

### Makefile Example

```makefile
ELFLOADER = runtime/elfloader/target/aarch64-unknown-none/release/libkaal_elfloader.a
KERNEL = build/kernel.elf
ROOT_TASK = target/aarch64-sel4/release/root-task
BOOTIMAGE = bootimage.elf

all: $(BOOTIMAGE)

$(ELFLOADER):
	cd runtime/elfloader && cargo build --release --target aarch64-unknown-none

$(KERNEL):
	cd kernel && mkdir -p build && cd build && \
	cmake .. -G Ninja -DPLATFORM=qemu-arm-virt && \
	ninja kernel.elf

$(ROOT_TASK):
	cargo build --release --bin root-task --target aarch64-sel4

$(BOOTIMAGE): $(ELFLOADER) $(KERNEL) $(ROOT_TASK)
	kaal-elfloader-builder \
		--loader $(ELFLOADER) \
		--kernel $(KERNEL) \
		--app $(ROOT_TASK) \
		--out $(BOOTIMAGE)
```

### Docker Example

```dockerfile
# Stage 1: Build elfloader
FROM rust:latest AS elfloader-build
WORKDIR /build
COPY runtime/elfloader .
RUN cargo build --release --target aarch64-unknown-none

# Stage 2: Build seL4 kernel
FROM kaal-kernel-builder AS kernel-build
WORKDIR /build/kernel
RUN cmake .. -G Ninja -DPLATFORM=qemu-arm-virt && ninja

# Stage 3: Build root task
FROM rust:latest AS roottask-build
WORKDIR /build
COPY runtime/root-task .
RUN cargo build --release --target aarch64-sel4

# Stage 4: Combine with elfloader-builder
FROM rust:latest AS final
COPY --from=elfloader-build /build/target/aarch64-unknown-none/release/libkaal_elfloader.a /elfloader.a
COPY --from=kernel-build /build/kernel/build/kernel.elf /kernel.elf
COPY --from=roottask-build /build/target/aarch64-sel4/release/root-task /root-task.elf

RUN cargo install kaal-elfloader-builder
RUN kaal-elfloader-builder \
    --loader /elfloader.a \
    --kernel /kernel.elf \
    --app /root-task.elf \
    --out /bootimage.elf
```

## Troubleshooting

### "Payload not initialized!" Error

This means the elfloader was built without being patched by the builder.
Always run `kaal-elfloader-builder` after building the elfloader.

### ELF Parsing Errors

Ensure input files are valid ELF64 binaries for ARM64:
```bash
file kernel.elf
# Should show: ELF 64-bit LSB executable, ARM aarch64
```

### Address Conflicts

If kernel and app overlap, adjust `--app-offset`:
```bash
kaal-elfloader-builder \
  --app-offset 0x800000  # Increase offset to 8MB
  ...
```

## Development

### Building

```bash
cd runtime/elfloader-builder
cargo build --release
```

### Testing

```bash
cargo test
```

### Logging

Set `RUST_LOG` for detailed output:
```bash
RUST_LOG=debug kaal-elfloader-builder ...
```

## See Also

- [runtime/elfloader/README.md](../elfloader/README.md) - Elfloader runtime documentation
- [IMPLEMENTATION.md](../elfloader/IMPLEMENTATION.md) - Technical details
