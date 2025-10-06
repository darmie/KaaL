# System Composition Example

This example demonstrates a complete multi-component KaaL system with:
- **Serial Driver** - UART hardware driver
- **Filesystem** - VFS service
- **Network Driver** - E1000 network card (placeholder)

## Architecture

```
┌─────────────────┐         ┌──────────────┐         ┌─────────────────┐
│ Serial Driver   │◄───────►│ Filesystem   │◄───────►│ Network Driver  │
│ (Priority 200)  │  IPC    │ (Priority    │  IPC    │ (Priority 150)  │
│                 │  Ring   │  100)        │  Ring   │                 │
└─────────────────┘         └──────────────┘         └─────────────────┘
        ▲                                                     ▲
        │ MMIO                                                │ MMIO
        │ IRQ 33                                              │ IRQ 11
        ▼                                                     ▼
   UART Device                                         E1000 NIC
```

## Building for Different Modes

### Mode 1: Mock (Testing Only)

Fast unit testing with mocks:

```bash
# Build
cargo build --features mock-sel4

# Run
cargo run --features mock-sel4 --bin system-composition

# Test
cargo test --features mock-sel4
```

### Mode 2: Microkit (Production - Default)

Production deployment on seL4 Microkit:

```bash
# 1. Build KaaL components as separate binaries
cargo build --release

# 2. Create ELF files for each component (requires separate build)
# See build-components.sh script below

# 3. Generate bootable system image with Microkit
microkit build system.xml

# 4. Run in QEMU
qemu-system-aarch64 \
    -M virt \
    -cpu cortex-a53 \
    -kernel loader.img \
    -nographic \
    -serial mon:stdio
```

**Note:** Microkit requires Linux. On macOS, use Docker:

```bash
docker run -it --rm -v $(pwd):/kaal rustlang/rust:nightly \
    bash -c "cd /kaal && microkit build system.xml"
```

### Mode 3: Runtime (Advanced)

Direct Rust seL4 runtime for full control:

```bash
# Build with runtime mode
cargo build --no-default-features --features sel4-runtime-mode

# Requires custom linker script and seL4 kernel
# See docs/SEL4_INTEGRATION.md for details
```

## Component ELF Files

For Microkit deployment, each component needs to be built as a separate ELF binary:

### Build Script: `build-components.sh`

```bash
#!/bin/bash
# Build individual component ELF files for Microkit

set -e

echo "Building KaaL components for Microkit..."

# Build serial driver
cargo build --release --bin serial-driver-component
cp target/release/serial-driver-component serial_driver.elf

# Build filesystem
cargo build --release --bin filesystem-component
cp target/release/filesystem-component filesystem.elf

# Build network driver
cargo build --release --bin network-driver-component
cp target/release/network-driver-component network_driver.elf

echo "✅ Components built:"
ls -lh *.elf
```

## System Configuration

The `system.xml` file defines:

### Memory Regions

- **`uart_mmio`** - UART device registers (0x09000000 on ARM QEMU virt)
- **`serial_to_fs_ring`** - Shared ring buffer (64KB) for serial→filesystem IPC
- **`net_to_app_ring`** - Shared ring buffer (64KB) for network→app IPC

### Protection Domains

Each component runs in isolation:

| Component | Priority | Resources | Purpose |
|-----------|----------|-----------|---------|
| serial_driver | 200 | UART MMIO, IRQ 33 | Hardware I/O |
| filesystem | 100 | Shared memory | VFS service |
| network_driver | 150 | E1000 MMIO, IRQ 11 | Network I/O |

### IPC Channels

- **Channel 1**: Serial driver ↔ Filesystem (ID 1↔2)
- **Channel 2**: Network driver ↔ Filesystem (ID 3↔4)

## Testing

### Unit Tests (Mock Mode)

```bash
cargo test --features mock-sel4 --bin system-composition
```

### Integration Test (Microkit in QEMU)

```bash
# Build system
./build-components.sh
microkit build system.xml

# Run in QEMU
qemu-system-aarch64 -M virt -cpu cortex-a53 -kernel loader.img -nographic

# Expected output:
# [seL4] Booting...
# [serial_driver] Initialized UART at 0x09000000
# [filesystem] VFS service ready
# [system] All components running ✓
```

## Platform Support

### ARM QEMU Virt Machine (Default)

```bash
qemu-system-aarch64 -M virt -cpu cortex-a53 -kernel loader.img -nographic
```

**Device addresses:**
- UART: 0x09000000, IRQ 33
- RAM: 128MB at 0x40000000

### x86_64 PC (Alternative)

Modify `system.xml`:
- Change UART address to 0x3F8 (COM1)
- Change IRQ to 4
- Update E1000 PCI configuration

```bash
qemu-system-x86_64 -kernel loader.img -nographic
```

## Extending the System

### Add a New Component

1. **Create component entry point:**

```rust
// examples/my-component/src/main.rs
#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // Your logic
    }
}
```

2. **Add to `system.xml`:**

```xml
<protection_domain name="my_component" priority="120">
  <program_image path="my_component.elf"/>
  <!-- Add resources, channels -->
</protection_domain>
```

3. **Build and integrate:**

```bash
cargo build --release --bin my-component
cp target/release/my-component my_component.elf
microkit build system.xml
```

## Troubleshooting

### "Component failed to start"

- Check ELF file exists and is valid: `file serial_driver.elf`
- Verify MMIO addresses match your platform
- Check IRQ numbers are correct

### "MMIO access fault"

- Ensure memory region permissions include `rw`
- Verify `cached="false"` for device memory
- Check physical address is correct for your platform

### "IRQ not firing"

- Verify IRQ number matches device
- Check IRQ is not already allocated
- Ensure notification endpoint is created

## Resources

- [SEL4_INTEGRATION.md](../../docs/SEL4_INTEGRATION.md) - Deployment guide
- [HOBBYIST_GUIDE.md](../../docs/HOBBYIST_GUIDE.md) - Getting started
- [seL4 Microkit Manual](https://github.com/seL4/microkit/blob/main/docs/manual.md)

## Next Steps

1. Build component ELF files
2. Test system.xml configuration
3. Deploy to QEMU
4. Add your custom components!
