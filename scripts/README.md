# KaaL Build Scripts

Utility scripts for building and verifying KaaL framework.

## 🔨 Build Scripts

### `build_microkit.sh`
Build KaaL with **real seL4** (Microkit mode).

```bash
./scripts/build_microkit.sh
```

**Requirements**: Linux + seL4 SDK with `SEL4_PREFIX` set

---

### `build-mock.sh`
Build KaaL with **mock backend** (for development).

```bash
./scripts/build-mock.sh
```

**Platform**: Works on any platform (macOS, Linux, Windows)

---

### `build-runtime.sh`
Build KaaL with **seL4 runtime mode** (advanced).

```bash
./scripts/build-runtime.sh
```

**Requirements**: Linux + seL4 SDK

---

### `build-microkit.sh`
Alternative Microkit build script.

```bash
./scripts/build-microkit.sh
```

---

## ✅ Verification Scripts

### `verify_real_sel4_default.sh`
**Primary verification**: Confirms KaaL defaults to real seL4, not mocks.

```bash
./scripts/verify_real_sel4_default.sh
```

**Tests**:
- ✅ Default build requires seL4 SDK
- ✅ Mock mode works with explicit flag
- ✅ Default features include microkit
- ✅ Mock is NOT in default features

---

### `test_backend_selection.sh`
Test all backend modes (mock, microkit, runtime).

```bash
./scripts/test_backend_selection.sh
```

**Tests**:
- Mock mode (default in old config)
- Explicit mock mode
- Microkit mode (expected to fail on macOS)
- Feature tree verification

---

## 🛠️ Setup Scripts

### `setup-macos.sh`
Set up development environment on macOS.

```bash
./scripts/setup-macos.sh
```

**Installs**:
- Rust toolchain
- Development dependencies
- Configures for mock-mode development

---

## 📊 Quick Reference

| Script | Purpose | Platform | Requires seL4 |
|--------|---------|----------|---------------|
| `build_microkit.sh` | Production build | Linux | ✅ Yes |
| `build-mock.sh` | Development build | Any | ❌ No |
| `build-runtime.sh` | Advanced seL4 | Linux | ✅ Yes |
| `verify_real_sel4_default.sh` | Verify production-first | Any | ❌ No |
| `test_backend_selection.sh` | Test all modes | Any | ❌ No |
| `setup-macos.sh` | macOS dev setup | macOS | ❌ No |

---

## 🚀 Typical Workflows

### Linux Development (Production)
```bash
# 1. Set seL4 environment
export SEL4_PREFIX=/path/to/seL4

# 2. Build with real seL4
./scripts/build_microkit.sh

# 3. Verify
./scripts/verify_real_sel4_default.sh
```

### macOS Development (Algorithm Work)
```bash
# 1. Setup environment
./scripts/setup-macos.sh

# 2. Build with mocks
./scripts/build-mock.sh

# 3. Test
cargo test
```

### CI/CD Pipeline
```bash
# Test all backends
./scripts/test_backend_selection.sh

# Verify production-first default
./scripts/verify_real_sel4_default.sh
```

---

## 📝 Adding New Scripts

When adding build scripts:

1. **Name convention**: `build-<mode>.sh` or `verify-<what>.sh`
2. **Make executable**: `chmod +x scripts/your-script.sh`
3. **Add header**: Include description and usage
4. **Update this README**: Document the new script

Example template:
```bash
#!/bin/bash
# Description: What this script does
# Usage: ./scripts/your-script.sh

set -e  # Exit on error

echo "=== Your Script Name ==="
# ... your commands ...
```

---

## 🔗 Related Documentation

- [BUILD_INSTRUCTIONS.md](../BUILD_INSTRUCTIONS.md) - Complete build guide
- [BUILD_MODES.md](../docs/BUILD_MODES.md) - Build mode details
- [README.md](../README.md) - KaaL framework overview
