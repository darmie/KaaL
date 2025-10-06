# KaaL Quick Start (Mac Silicon)

**3 simple commands to build and test KaaL with real seL4 on your Mac**

## Prerequisites

- ✅ Docker Desktop installed and running
- ✅ Mac Silicon (M1/M2/M3)
- ✅ 10GB free disk space

## Quick Start

### Step 1: Build Docker Image (one-time, ~15 minutes)

```bash
./scripts/docker-build.sh
```

This will:
- Build seL4 kernel for ARM64
- Set up complete development environment
- Compile KaaL framework

### Step 2: Run QEMU Test

```bash
./scripts/docker-test.sh
```

This will:
- Build KaaL with real seL4
- Launch in QEMU emulator
- Run your OS!

**To exit QEMU**: Press `Ctrl+A` then `X`

### Step 3: Development (optional)

```bash
./scripts/docker-shell.sh
```

Inside the container:
```bash
# Build for different architectures
cargo build --features board-pc99                    # x86_64
cargo build --features board-qemu-virt-riscv64      # RISC-V

# Run tests
cargo test --no-default-features --features mock

# Build release
cargo build --release --features board-qemu-virt-aarch64
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
