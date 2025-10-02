# Troubleshooting Guide

This document contains solutions to common issues encountered during KaaL development.

---

## Build Issues

### Issue: `ModuleNotFoundError: No module named 'tempita'`

**Symptoms:**
```
ModuleNotFoundError: No module named 'tempita'
assertion failed: cmd.status().unwrap().success()
```

**Cause:** seL4 Python dependencies not installed

**Solution:**

```bash
# Install required Python packages
pip3 install --user tempita ply jinja2 sel4-deps

# Or on macOS with Homebrew Python:
pip3 install --user --break-system-packages tempita ply jinja2 sel4-deps

# Verify installation
python3 -c "import tempita; print('tempita OK')"
python3 -c "import ply; print('ply OK')"
```

**Alternative:** Run the setup script which installs all dependencies:
```bash
./scripts/setup-macos.sh  # On macOS
```

### Issue: `sel4-sys` build fails

**Symptoms:**
```
error: failed to run custom build command for `sel4-sys`
```

**Solutions:**

1. **Install seL4 dependencies:**
   ```bash
   pip3 install sel4-deps camkes-deps
   ```

2. **Use feature flag to skip seL4 for now (Phase 1 development):**
   Add this to workspace Cargo.toml temporarily:
   ```toml
   [patch.crates-io]
   sel4-sys = { git = "https://github.com/seL4/rust-sel4", branch = "main" }
   ```

3. **Mock seL4 for testing (recommended for Phase 1):**
   We'll create mock implementations for initial development

---

## Compilation Issues

### Issue: Workspace member not found

**Symptoms:**
```
error: failed to load manifest for workspace member
```

**Solution:**
Ensure all workspace members have valid `Cargo.toml` files:
```bash
# Check all members
for dir in runtime/* components/* tools/*; do
  if [ ! -f "$dir/Cargo.toml" ]; then
    echo "Missing: $dir/Cargo.toml"
  fi
done
```

### Issue: Target not found

**Symptoms:**
```
error: target not found: x86_64-unknown-none
```

**Solution:**
```bash
rustup target add x86_64-unknown-none
rustup target add aarch64-unknown-none
```

---

## Runtime Issues

### Issue: QEMU not starting

**Symptoms:**
QEMU hangs or crashes immediately

**Solutions:**

1. **Check QEMU installation:**
   ```bash
   qemu-system-x86_64 --version
   qemu-system-aarch64 --version
   ```

2. **Verify kernel binary:**
   ```bash
   file target/x86_64-unknown-none/debug/kaal
   # Should show: ELF 64-bit executable
   ```

3. **Try with verbose output:**
   ```bash
   qemu-system-x86_64 \
       -kernel target/x86_64-unknown-none/debug/kaal \
       -d int,cpu_reset \
       -no-reboot \
       -nographic
   ```

---

## macOS Specific Issues

### Issue: LLDB can't attach

**Symptoms:**
```
error: attach failed: operation not permitted
```

**Solution:**
On macOS, code signing is required for debugging. Use LLDB instead of GDB:
```bash
lldb target/aarch64-unknown-none/debug/kaal
```

### Issue: Homebrew GDB not working

**Solution:**
Use LLDB (native on macOS) or follow GDB code signing instructions:
https://sourceware.org/gdb/wiki/PermissionsDarwin

### Issue: `xcrun` error

**Symptoms:**
```
xcrun: error: invalid active developer path
```

**Solution:**
```bash
xcode-select --install
```

---

## Development Workflow Issues

### Issue: Slow x86_64 builds on Apple Silicon

**Cause:** Cross-compilation overhead

**Solutions:**

1. **Develop on native architecture:**
   ```bash
   cargo build --target aarch64-unknown-none
   ```

2. **Use AArch64 for iteration:**
   ```bash
   # Fast development cycle
   cargo build --target aarch64-unknown-none
   cargo test --target aarch64-unknown-none

   # Test x86 compatibility before commit
   cargo build --target x86_64-unknown-none
   ```

3. **Parallel builds:**
   ```bash
   cargo build --target aarch64-unknown-none &
   cargo build --target x86_64-unknown-none &
   wait
   ```

---

## Testing Issues

### Issue: Tests fail to run

**Symptoms:**
```
error: test failed, to rerun pass '--lib'
```

**Solutions:**

1. **Run with verbose output:**
   ```bash
   cargo test --workspace -- --nocapture
   ```

2. **Run specific test:**
   ```bash
   cargo test --package cap-broker test_name
   ```

3. **Check for missing dependencies:**
   ```bash
   cargo test --workspace --verbose
   ```

---

## Debugging Issues

### Issue: LLDB can't find symbols

**Symptoms:**
```
warning: unable to find debug symbols
```

**Solution:**
Ensure debug symbols are enabled:
```toml
# Cargo.toml
[profile.dev]
debug = true

[profile.release]
debug = true  # For release with symbols
```

### Issue: Breakpoints not hitting

**Solutions:**

1. **Verify debug build:**
   ```bash
   cargo build --target aarch64-unknown-none
   # NOT --release
   ```

2. **Check QEMU is waiting:**
   ```bash
   # QEMU should pause with -s -S flags
   qemu-system-aarch64 -s -S ...
   ```

3. **Verify LLDB connection:**
   ```lldb
   (lldb) gdb-remote localhost:1234
   (lldb) image list  # Should show your binary
   ```

---

## VS Code Issues

### Issue: rust-analyzer not working

**Solutions:**

1. **Reload VS Code:**
   `Cmd+Shift+P` → "Reload Window"

2. **Clear rust-analyzer cache:**
   ```bash
   rm -rf ~/.cache/rust-analyzer
   ```

3. **Check workspace settings:**
   Ensure `.vscode/settings.json` exists

### Issue: CodeLLDB extension not working

**Solution:**
```bash
# Install/reinstall extension
code --install-extension vadimcn.vscode-lldb

# Check LLDB is available
lldb --version
```

---

## Performance Issues

### Issue: Slow compilation

**Solutions:**

1. **Use incremental compilation:**
   ```bash
   export CARGO_INCREMENTAL=1
   ```

2. **Reduce dependencies:**
   Only build what you need:
   ```bash
   cargo build --package cap-broker
   ```

3. **Use mold linker (Linux) or lld (macOS):**
   ```toml
   # .cargo/config.toml
   [target.x86_64-unknown-none]
   linker = "rust-lld"
   ```

---

## Dependency Issues

### Issue: Dependency version conflicts

**Solution:**
```bash
# Update Cargo.lock
cargo update

# Or clean and rebuild
cargo clean
cargo build
```

### Issue: `sel4-deps` installation fails

**Solutions:**

macOS:
```bash
pip3 install --user --break-system-packages sel4-deps
```

Linux:
```bash
sudo apt install python3-dev
pip3 install --user sel4-deps
```

---

## Getting More Help

If you encounter an issue not listed here:

1. **Check documentation:**
   - [GETTING_STARTED.md](GETTING_STARTED.md)
   - [MAC_SILICON_SETUP.md](MAC_SILICON_SETUP.md)
   - [ARCHITECTURE.md](ARCHITECTURE.md)

2. **Search existing issues:**
   - https://github.com/your-org/kaal/issues

3. **Ask for help:**
   - GitHub Discussions: https://github.com/your-org/kaal/discussions
   - Tag your question with relevant labels (macOS, linux, build, runtime, etc.)

4. **Provide details when asking:**
   - Platform (macOS ARM64, Linux x86_64, etc.)
   - Rust version (`rustc --version`)
   - Complete error message
   - Steps to reproduce

---

**Last Updated:** 2025-10-02
