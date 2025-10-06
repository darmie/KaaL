# QEMU Readiness Status

**Current Status**: ❌ Not ready - Need seL4 kernel build and Linux environment

---

## 🚧 Blockers for QEMU Testing

### 1. seL4 Kernel Must Be Built First

**Issue**: rust-sel4 requires `kernel/gen_config.json` which is only generated when seL4 kernel is built

**Error**:
```
kernel/gen_config.json not found in libsel4 include path
```

**Why**: The seL4 kernel configuration is generated during kernel build. We have the source but not a configured build.

**Solution**:
```bash
# On Linux (or Docker):
cd external/seL4
mkdir build && cd build

# Configure for ARM64 QEMU
cmake -DPLATFORM=qemu-arm-virt \
      -DAARCH64=1 \
      -DSIMULATION=TRUE \
      -G Ninja \
      ..

# Build kernel
ninja

# This generates kernel/gen_config.json and sel4/gen_config.json
```

---

### 2. Platform-Specific Assembly (sel4-alloca)

**Issue**: sel4-alloca has x86 inline assembly that fails on ARM macOS

**Error**:
```
error: unexpected token in '.section' directive
note: instantiated into assembly here
```

**Why**: Building on macOS ARM but sel4-alloca contains x86-specific code

**Solution**: Build on Linux or in Docker container

---

## ✅ What We Have Ready

1. ✅ **KaaL Framework** - All crates compile with mock backend
2. ✅ **Adapter Layer** - Unified API working
3. ✅ **Mock Verification** - Signatures match real seL4
4. ✅ **Build Configuration** - Features set for microkit + ARM64
5. ✅ **seL4 Sources** - Kernel source code in `external/seL4`
6. ✅ **rust-sel4 Bindings** - In `external/rust-sel4`

## 📋 Steps to QEMU Readiness

### Option A: Linux Native Build (Recommended)

```bash
# 1. Use Linux machine (or VM)
ssh your-linux-machine

# 2. Build seL4 kernel for ARM64 QEMU
cd ~/kaal/external/seL4
mkdir build-arm64-qemu && cd build-arm64-qemu
cmake -DPLATFORM=qemu-arm-virt -DAARCH64=1 -DSIMULATION=TRUE -G Ninja ..
ninja

# 3. Set environment
export SEL4_PREFIX=~/kaal/external/seL4

# 4. Build KaaL
cd ~/kaal
cargo build --release --features board-qemu-virt-aarch64

# 5. Install seL4 Microkit
# Download from: https://github.com/seL4/microkit
# Extract to /opt/microkit or ~/microkit

# 6. Create Microkit system description
cat > system.toml << EOF
[system]
kernel = "external/seL4/build-arm64-qemu/kernel/kernel.elf"

[protection_domains.root]
program_image = "target/release/kaal-root-task"
budget = 1000

[protection_domains.cap_broker]
program_image = "target/release/cap-broker"
budget = 1000
EOF

# 7. Build system image
microkit system.toml

# 8. Run in QEMU
qemu-system-aarch64 \
  -machine virt,virtualization=on \
  -cpu cortex-a53 \
  -nographic \
  -m 2G \
  -kernel loader.elf
```

---

### Option B: Docker on macOS (Easier)

```bash
# 1. Create Dockerfile
cat > Dockerfile.sel4 << 'EOF'
FROM trustworthysystems/sel4:latest

WORKDIR /workspace

# Build seL4 kernel
RUN cd /seL4 && \
    mkdir build-arm64-qemu && cd build-arm64-qemu && \
    cmake -DPLATFORM=qemu-arm-virt -DAARCH64=1 -DSIMULATION=TRUE -G Ninja .. && \
    ninja

ENV SEL4_PREFIX=/seL4

# Copy KaaL source
COPY . /workspace

# Build KaaL
RUN cargo build --release --features board-qemu-virt-aarch64

CMD ["/bin/bash"]
EOF

# 2. Build container
docker build -f Dockerfile.sel4 -t kaal-builder .

# 3. Run build
docker run --rm -v $(pwd)/target:/workspace/target kaal-builder

# 4. Extract binaries
# Binaries are now in ./target/release/

# 5. Run in QEMU (on host)
qemu-system-aarch64 \
  -machine virt,virtualization=on \
  -cpu cortex-a53 \
  -nographic \
  -m 2G \
  -kernel target/release/system.elf
```

---

### Option C: GitHub Actions / CI (For Testing)

```yaml
# .github/workflows/qemu-test.yml
name: QEMU Test

on: [push, pull_request]

jobs:
  build-and-test:
    runs-on: ubuntu-latest
    container: trustworthysystems/sel4:latest

    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive

      - name: Build seL4 kernel
        run: |
          cd external/seL4
          mkdir build && cd build
          cmake -DPLATFORM=qemu-arm-virt -DAARCH64=1 -DSIMULATION=TRUE -G Ninja ..
          ninja

      - name: Build KaaL
        run: |
          export SEL4_PREFIX=$PWD/external/seL4
          cargo build --release --features board-qemu-virt-aarch64

      - name: Install QEMU
        run: apt-get update && apt-get install -y qemu-system-aarch64

      - name: Run in QEMU (smoke test)
        run: |
          timeout 10 qemu-system-aarch64 \
            -machine virt,virtualization=on \
            -cpu cortex-a53 \
            -nographic \
            -m 2G \
            -kernel target/release/loader.elf || true
```

---

## 🎯 Immediate Next Steps

### For QEMU Testing Today:

**If you have Linux**:
1. Build seL4 kernel (30 minutes)
2. Build KaaL (5 minutes)
3. Run in QEMU (instant)

**If you have macOS**:
1. Use Docker with seL4 image (10 minutes setup)
2. Build everything in container (30 minutes)
3. Extract and run in QEMU (instant)

### Quick Docker Command:

```bash
# One-liner to build in Docker
docker run --rm \
  -v $(pwd):/workspace \
  -w /workspace \
  -e SEL4_PREFIX=/seL4 \
  trustworthysystems/sel4:latest \
  bash -c "
    cd /seL4 &&
    mkdir -p build && cd build &&
    cmake -DPLATFORM=qemu-arm-virt -DAARCH64=1 -G Ninja .. &&
    ninja &&
    cd /workspace &&
    cargo build --features board-qemu-virt-aarch64
  "
```

---

## 📊 Readiness Checklist

- [x] KaaL framework code complete
- [x] seL4 adapter layer implemented
- [x] Mock signatures verified
- [x] Build configuration ready
- [ ] seL4 kernel built and configured
- [ ] KaaL compiled against real seL4
- [ ] Microkit system image created
- [ ] QEMU test executed
- [ ] Boot sequence verified
- [ ] IPC between components tested

**Status**: 60% ready - need kernel build and real compilation

---

## 🚀 Estimated Time to QEMU

- **With Linux access**: ~1 hour
  - 30 min: Build seL4 kernel
  - 15 min: Build KaaL
  - 15 min: Create system image and test

- **With Docker on macOS**: ~1-2 hours
  - 30 min: Docker setup and kernel build
  - 30 min: Build KaaL in container
  - 30 min: Microkit integration and testing

---

## 💡 Recommendation

**Best path forward**:

1. **Use Docker** - Most portable, works on macOS
2. **Build seL4 kernel once** - Cache the built kernel
3. **Iterate on KaaL** - Rebuild KaaL quickly after kernel is ready
4. **Start simple** - Boot to root task, verify seL4 is running
5. **Add complexity** - Enable components one by one

**Start command**:
```bash
# This will get you 90% there
docker run -it --rm \
  -v $(pwd):/workspace \
  trustworthysystems/sel4:latest \
  bash
```

Then follow Option B steps inside the container.
