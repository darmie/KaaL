# KaaL Build System

KaaL provides two build systems:
1. **build.nu** (Nushell) - Modern, modular, recommended
2. **build.sh** (Bash) - Legacy, for compatibility

## Modular Architecture

The Nushell build system uses a modular architecture:

```
build-system/
├── config/mod.nu          # Configuration and platform management
├── utils/mod.nu           # Utilities and helpers
└── builders/
    ├── mod.nu             # High-level build functions
    ├── codegen.nu         # Code generation
    └── components.nu      # Component discovery
```

**Original:**
1. **build.nu** (Nushell) - Modern, recommended
2. **build.sh** (Bash) - Legacy, for compatibility

## Quick Start

### Using Nushell (Recommended)

```bash
# Install Nushell (if not already installed)
# macOS:
brew install nushell

# Linux:
cargo install nu

# Build for QEMU (default platform)
./build.nu

# Build for specific platform
./build.nu --platform qemu-virt

# Verbose output
./build.nu --verbose

# Clean build
./build.nu --clean
```

### Using Bash (Legacy)

```bash
# Build for QEMU (default platform)
./build.sh

# Build for specific platform
./build.sh --platform qemu-virt

# Verbose output
./build.sh --verbose
```

## Why Nushell?

**build.nu** provides several advantages over traditional bash:

### ✅ Type Safety
```nu
# Nushell has structured data types
let config: record = (load-config)
let platform_cfg: record = (get-platform $config $platform)

# vs bash string manipulation
PLATFORM=$(get_config "build" "default_platform")
```

### ✅ Better Error Handling
```nu
# Explicit error handling
if not ($kernel_elf | path exists) {
    error make { msg: "Kernel build failed" }
}

# vs bash
[ -f "$KERNEL_ELF" ] || { echo "ERROR: Kernel not built"; exit 1; }
```

### ✅ Structured Output
```nu
# Pretty printing with type information
print $"✓ Kernel: (ls $kernel_elf | get 0.size | into string)"

# Built-in progress indicators
print-step 1 4 "Building kernel"
```

### ✅ Cross-Platform
Nushell works identically on Linux, macOS, and Windows. No more `sed -i` vs `sed -i ''` issues!

### ✅ Native TOML Support
```nu
# Direct TOML parsing
let config = (open build-config.toml)
let platform = ($config | get platform.qemu-virt)

# vs bash AWK hacks
get_config() {
    awk -v section="$section" -v key="$key" '...' "$config_file"
}
```

### ✅ Component Discovery
```nu
# Automatic component discovery
let components = (open components.toml | get component)
print $"🔍 Discovered ($components | length) component(s)"

for component in $components {
    print $"  ($component.name) - ($component.type)"
}
```

## Build Process

The build system performs these steps:

### 1. Component Discovery
```
🔍 Discovered 6 component(s) from components.toml:
  [✓] system_init (service, priority: 255)
  [✓] serial_driver (driver, priority: 200)
  [✓] timer_driver (driver, priority: 200)
  [✓] process_manager (service, priority: 150)
  [ ] vfs_service (service, priority: 100)
  [ ] shell (application, priority: 50)
```

### 2. Configuration Generation
- **memory_config.rs**: Platform-specific memory layout
- **kernel.ld**: Kernel linker script
- **linker.ld**: Elfloader linker script

### 3. Component Builds
```
[1/4] Building kernel...
  ✓ Kernel: 156.2 KiB

[2/4] Building root-task...
  🔍 Component manifest: /path/to/project/components.toml
  📦 Found 6 component(s)
  ✓ Root-task: 45.3 KiB

[3/4] Creating embeddable objects...
  ✓ kernel.o: 156.3 KiB
  ✓ roottask.o: 45.4 KiB

[4/4] Building elfloader...
  ✓ Final Image: 202.1 KiB
```

### 4. Bootimage Creation
The elfloader embeds:
- Kernel ELF binary
- Root-task ELF binary
- Component manifest metadata

## Platform Configuration

Platforms are defined in `build-config.toml`:

```toml
[platform.qemu-virt]
name = "QEMU virt (ARM64)"
arch = "aarch64"

# Memory layout
ram_base = "0x40000000"
ram_size = "0x8000000"  # 128MB

# Device addresses
uart0_base = "0x09000000"
uart1_base = "0x09010000"

# Boot offsets
elfloader_offset = "0x200000"
kernel_offset = "0x400000"
```

### Supported Platforms

- **qemu-virt** - QEMU ARM64 virt machine (default)
- **rpi4** - Raspberry Pi 4 (ARM64 Cortex-A72)

### Adding a New Platform

1. Add platform section to `build-config.toml`:
   ```toml
   [platform.my-board]
   name = "My Board"
   arch = "aarch64"
   ram_base = "0x80000000"
   # ... other settings
   ```

2. Build:
   ```bash
   ./build.nu --platform my-board
   ```

## Build Flags

### Common Flags

| Flag | Bash | Nushell | Description |
|------|------|---------|-------------|
| Platform | `--platform <name>` | `--platform <name>` or `-p <name>` | Select platform |
| Verbose | `--verbose` or `-v` | `--verbose` or `-v` | Show detailed output |
| Clean | N/A | `--clean` or `-c` | Clean before building |

### Nushell-Specific Features

```bash
# Tab completion
./build.nu --<TAB>
# Shows: --platform, --verbose, --clean, --help

# Help
./build.nu --help
# Shows detailed usage and flags

# Structured output
./build.nu --verbose
# Uses Nushell's table formatting
```

## Environment Variables

Both build systems support:

| Variable | Description | Default |
|----------|-------------|---------|
| `KAAL_PLATFORM` | Override platform | Value from `--platform` |
| `RUSTFLAGS` | Pass flags to rustc | Set automatically |

## Build Artifacts

```
runtime/build/
├── kernel.o          # Kernel embeddable object
└── roottask.o        # Root-task embeddable object

kernel/
├── target/aarch64-unknown-none/release/
│   └── kaal-kernel   # Kernel ELF binary
└── src/generated/
    └── memory_config.rs  # Generated config

runtime/
├── root-task/target/aarch64-unknown-none/release/
│   └── root-task     # Root-task ELF binary
└── elfloader/target/aarch64-unknown-none-elf/release/
    └── elfloader     # Final bootimage
```

## Running the Bootimage

### QEMU (qemu-virt)

```bash
# Run the built image
./build.nu

# Output shows QEMU command:
qemu-system-aarch64 -machine virt -cpu cortex-a53 -m 128M \
    -nographic -kernel runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader

# Or use the run script
./run.sh
```

### Real Hardware (rpi4)

```bash
# Build for Raspberry Pi 4
./build.nu --platform rpi4

# Deploy bootimage to SD card
# (Copy elfloader binary to boot partition as kernel8.img)
```

## Troubleshooting

### "nu: command not found"

Install Nushell:
```bash
# macOS
brew install nushell

# Linux
cargo install nu

# Or use legacy bash build
./build.sh
```

### "components.toml not found"

The build script expects `components.toml` at the project root:
```bash
# Check it exists
ls components.toml

# If missing, create it (see COMPONENTS.md)
```

### "Kernel build failed"

Common causes:
1. **Missing Rust nightly**: `rustup install nightly`
2. **Missing aarch64 target**: `rustup target add aarch64-unknown-none --toolchain nightly`
3. **Missing build-std**: Use nightly with `-Z build-std`

```bash
# Fix
rustup default nightly
rustup component add rust-src
```

### Build Cache Issues

```bash
# Clean all build artifacts
cargo clean

# Nushell clean build
./build.nu --clean

# Manual clean
rm -rf runtime/build kernel/target runtime/*/target
```

## Comparison: Nushell vs Bash

| Feature | Nushell (build.nu) | Bash (build.sh) |
|---------|-------------------|-----------------|
| **Type Safety** | ✅ Records, typed data | ❌ String manipulation |
| **Error Handling** | ✅ Structured errors | ⚠️ Exit codes only |
| **TOML Parsing** | ✅ Native support | ⚠️ AWK hacks |
| **Component Discovery** | ✅ Integrated | ❌ Not implemented |
| **Cross-Platform** | ✅ Linux, macOS, Windows | ⚠️ macOS/Linux only |
| **Tab Completion** | ✅ Built-in | ❌ Manual setup |
| **Help System** | ✅ `--help` flag | ⚠️ Manual docs |
| **Progress Reporting** | ✅ Structured | ⚠️ Echo statements |
| **Maintainability** | ✅ High | ⚠️ Medium |
| **LOC** | 390 | 398 |

## Migration Path

Both build systems coexist:

1. **Now**: Use either `./build.nu` or `./build.sh`
2. **Transition**: Try Nushell for new features
3. **Future**: Bash script maintained for compatibility

## Advanced Usage

### Custom Build Steps

```nu
# Run just the component discovery
nu -c "source build.nu; discover-components"

# Generate only configuration files
nu -c "source build.nu; generate-memory-config (load-config | get platform.qemu-virt)"

# Build individual components
cargo build -p kaal-kernel --target aarch64-unknown-none
```

### CI/CD Integration

```yaml
# GitHub Actions example
- name: Install Nushell
  run: cargo install nu

- name: Build KaaL
  run: ./build.nu --platform qemu-virt --verbose

- name: Run tests
  run: |
    ./build.nu
    qemu-system-aarch64 ... -kernel runtime/elfloader/.../elfloader
```

## References

- [Nushell Documentation](https://www.nushell.sh/)
- [build-config.toml](build-config.toml) - Platform configurations
- [components.toml](components.toml) - Component manifest
- [COMPONENTS.md](COMPONENTS.md) - Component development guide

## Summary

**Recommended**: Use `./build.nu` for modern, maintainable builds with automatic component discovery.

**Compatibility**: Use `./build.sh` if Nushell is not available.

Both build systems produce identical artifacts and support the same platforms.
