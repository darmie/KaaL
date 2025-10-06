# Cross-Platform OS Development with KaaL

## Build Host vs Target Platform

```
╔══════════════════════════════════════════════════════════════╗
║  BUILD HOST          vs          TARGET PLATFORM             ║
║  (Where you compile)             (Where OS runs)             ║
╚══════════════════════════════════════════════════════════════╝

BUILD HOST (Current Limitation):
  - Linux (native)
  - macOS/Windows (via Docker)
  └─> This is a TOOLING limitation, not fundamental

TARGET PLATFORM (Your OS runs here):
  ✅ ARM64 boards (Raspberry Pi, BeagleBone, etc.)
  ✅ x86_64 PCs
  ✅ RISC-V boards
  ✅ Embedded devices
  ✅ IoT hardware
  └─> ANY seL4-supported architecture!
```

## 📊 Build Matrix

| Build Host | Target Platform | Status | Notes |
|------------|-----------------|--------|-------|
| **Linux** | ARM64 | ✅ Works | Full seL4 support |
| **Linux** | x86_64 | ✅ Works | Full seL4 support |
| **Linux** | RISC-V | ✅ Works | Full seL4 support |
| **macOS** (via Docker) | ARM64 | ✅ Works | Use seL4 container |
| **macOS** (via Docker) | x86_64 | ✅ Works | Use seL4 container |
| **macOS** (native) | Any | ⚠️ Mocks only | Framework dev |
| **Windows** (via WSL2) | Any | ✅ Works | Same as Linux |

## 🚀 Cross-Platform Workflow

### Scenario 1: Build on Linux, Deploy to ARM

```bash
# Build host: Ubuntu on x86_64
export SEL4_PREFIX=/path/to/seL4
cargo build --features board-qemu-virt-aarch64

# Output: OS image for ARM64
# Deploy to: Raspberry Pi, BeagleBone, ARM servers, etc.
```

### Scenario 2: Build on macOS (Docker), Deploy to x86 PC

```bash
# Build host: macOS with Docker
docker run -v $(pwd):/workspace \
  -e SEL4_PREFIX=/seL4 \
  trustworthysystems/sel4:latest \
  cargo build --features board-pc99

# Output: OS image for x86_64
# Deploy to: Any PC, laptop, x86 server
```

### Scenario 3: Build on Windows (WSL2), Deploy to RISC-V

```bash
# Build host: Windows WSL2 (Ubuntu)
export SEL4_PREFIX=/path/to/seL4
cargo build --features board-qemu-virt-riscv64

# Output: OS image for RISC-V
# Deploy to: RISC-V boards, simulators
```

## 🔧 Why Linux Build Requirement?

The current Linux requirement is due to:

1. **seL4 SDK tooling** - Built for Linux
2. **Cross-compilation toolchains** - Easier on Linux
3. **Kernel config generation** - Requires native build

**This is NOT fundamental** - it's a tooling issue we're addressing.

## 🎯 Target Hardware Examples

### ARM64 Deployments
- **Raspberry Pi 4/5** - Consumer SBC
- **BeagleBone Black** - Embedded development
- **NXP i.MX8** - Industrial IoT
- **Odroid-C4** - Edge computing
- **Custom ARM boards** - Your hardware

### x86_64 Deployments
- **Intel NUC** - Small form factor PC
- **Desktop PC** - Standard x86 hardware
- **Laptops** - Mobile devices
- **Server hardware** - Data centers
- **Virtual machines** - QEMU, VMware

### RISC-V Deployments
- **Pine64 Star64** - RISC-V SBC
- **SiFive boards** - Development hardware
- **Custom RISC-V** - Your silicon

## 💡 Cross-Compilation Example

```bash
# Build OS for 3 different architectures from ONE build host:

# ARM64 image
cargo build --release --features board-qemu-virt-aarch64
mv target/release/my_os.elf deploy/arm64/

# x86 image
cargo build --release --features board-pc99
mv target/release/my_os.elf deploy/x86_64/

# RISC-V image
cargo build --release --features board-qemu-virt-riscv64
mv target/release/my_os.elf deploy/riscv64/

# Now deploy each image to its respective hardware!
```

## 🐳 Docker Workflow (Recommended for macOS)

```bash
# Use seL4 build container
cat > Dockerfile << 'EOF'
FROM trustworthysystems/sel4:latest

WORKDIR /workspace
COPY . .

# Build for all platforms
RUN cargo build --features board-qemu-virt-aarch64 && \
    cargo build --features board-pc99 && \
    cargo build --features board-qemu-virt-riscv64
EOF

docker build -t my-os-builder .
docker run --rm -v $(pwd)/deploy:/workspace/target my-os-builder

# Outputs ready for ALL platforms!
```

## 📝 Summary

**Your OS deployment**:
- Runs on: ARM, x86, RISC-V, embedded, IoT, servers, anything seL4 supports
- Build from: Linux (best), Docker (good), WSL2 (works)
- Develop on: Any platform with mocks for algorithm work

**The "Linux requirement" is a BUILD TOOL issue, not an OS limitation.**

We're actively working to enable native macOS builds by generating seL4 configs without full kernel build.
