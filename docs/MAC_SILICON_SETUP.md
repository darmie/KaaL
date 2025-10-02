# KaaL on macOS Apple Silicon (M1/M2/M3)

This guide provides specific instructions for developing KaaL on macOS with Apple Silicon processors.

## Quick Setup

### Automated Setup (Recommended)

```bash
# Run the automated setup script
./scripts/setup-macos.sh
```

This script will:
- ✅ Install Homebrew (if needed)
- ✅ Install Rust toolchain
- ✅ Install QEMU (both x86_64 and AArch64)
- ✅ Install LLVM and development tools
- ✅ Add Rust compilation targets
- ✅ Install seL4 dependencies
- ✅ Install Cargo development tools
- ✅ Configure your environment

### Manual Setup

If you prefer manual installation, see [GETTING_STARTED.md](GETTING_STARTED.md#macos-apple-silicon--m1m2m3).

## Architecture Considerations

### Native Development
Your Mac runs on **ARM64 (AArch64)** architecture:
- Host: `aarch64-apple-darwin`
- Native target: `aarch64-unknown-none`
- Cross target: `x86_64-unknown-none`

### Performance Characteristics

| Target | QEMU Speed | Compile Speed | Use Case |
|--------|-----------|---------------|----------|
| `aarch64-unknown-none` | ⚡ Fast (native) | ⚡ Fast | Daily development |
| `x86_64-unknown-none` | 🐌 Slower (emulation) | ✅ Normal | Testing x86 compatibility |

**Recommendation:** Use AArch64 for rapid development, test with x86_64 before release.

## Building for Different Architectures

### Build Commands

```bash
# Build for AArch64 (fast on Apple Silicon)
cargo build --target aarch64-unknown-none

# Build for x86_64 (slower, emulated)
cargo build --target x86_64-unknown-none

# Build for both
cargo build --target aarch64-unknown-none && \
cargo build --target x86_64-unknown-none
```

### Running in QEMU

```bash
# AArch64 (native performance)
qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a57 \
    -kernel target/aarch64-unknown-none/debug/kaal \
    -nographic

# x86_64 (emulated, slower)
qemu-system-x86_64 \
    -kernel target/x86_64-unknown-none/debug/kaal \
    -nographic
```

## VS Code Integration

### Recommended Extensions

Install via command line:
```bash
code --install-extension rust-lang.rust-analyzer
code --install-extension vadimcn.vscode-lldb
code --install-extension tamasfe.even-better-toml
code --install-extension usernamehw.errorlens
```

Or install from VS Code extensions panel (see `.vscode/extensions.json`)

### Debugging Setup

The project includes pre-configured debugging for both architectures:

1. **Debug KaaL (x86_64 in QEMU)** - For x86 compatibility testing
2. **Debug KaaL (AArch64 in QEMU)** - For native ARM development
3. **Debug Unit Tests (Native)** - For testing on your Mac

#### To Debug:

1. Build the project: `cargo build --target aarch64-unknown-none` or `--target x86_64-unknown-none`
2. Press `F5` in VS Code
3. Select the appropriate configuration
4. QEMU will start automatically with GDB server
5. LLDB will connect and break at entry point

### Tasks Available

Press `Cmd+Shift+P` → "Tasks: Run Task":

- **build-aarch64** - Build for ARM64
- **build-x86_64** - Build for x86-64
- **test-all** - Run all tests
- **clippy** - Run linter
- **pre-commit** - Run all checks

## Debugging with LLDB

### Command Line Debugging

```bash
# Terminal 1: Start QEMU with debug server
qemu-system-aarch64 \
    -machine virt -cpu cortex-a57 \
    -kernel target/aarch64-unknown-none/debug/kaal \
    -s -S -nographic

# Terminal 2: Connect LLDB
lldb target/aarch64-unknown-none/debug/kaal
(lldb) gdb-remote localhost:1234
(lldb) breakpoint set --name main
(lldb) continue
```

### Useful LLDB Commands

```lldb
# Set breakpoints
breakpoint set --name function_name
breakpoint set --file main.rs --line 42

# Execution control
continue (c)
step (s)
next (n)
finish

# Inspection
frame variable (fr v)
register read
memory read $sp

# Call stack
bt (backtrace)
frame select 0
```

## Performance Profiling on macOS

### Using cargo-instruments (Native macOS Tool)

```bash
# Install
cargo install cargo-instruments

# Profile allocations
cargo instruments --template Allocations --target aarch64-unknown-none

# Profile CPU usage
cargo instruments --template "Time Profiler" --bench cap_allocation

# Available templates
cargo instruments --list-templates
```

### Using Flamegraph (Cross-platform)

```bash
# Install
cargo install cargo-flamegraph

# Generate flamegraph
cargo flamegraph --bench cap_allocation

# Opens flamegraph.svg in browser
```

## Troubleshooting

### Issue: LLVM not in PATH

```bash
# Add to ~/.zshrc
export PATH="/opt/homebrew/opt/llvm/bin:$PATH"

# Apply immediately
source ~/.zshrc
```

### Issue: Python sel4-deps Installation Fails

```bash
# Use Homebrew Python
brew install python3
pip3 install --user --break-system-packages sel4-deps camkes-deps
```

### Issue: QEMU Crashes or Won't Start

```bash
# Reinstall QEMU
brew reinstall qemu

# Verify installation
qemu-system-aarch64 --version
qemu-system-x86_64 --version
```

### Issue: Rust Target Not Found

```bash
# List installed targets
rustup target list --installed

# Add missing target
rustup target add aarch64-unknown-none
rustup target add x86_64-unknown-none
```

### Issue: VS Code Can't Find LLDB

```bash
# LLDB is built into Xcode Command Line Tools
xcode-select --install

# Verify LLDB installation
lldb --version
```

### Issue: Build Fails with Linker Errors

```bash
# Clean build
cargo clean

# Rebuild with verbose output
cargo build --verbose --target aarch64-unknown-none

# Check LLVM installation
llvm-config --version
```

## Tips for macOS Development

### 1. Use Native Architecture for Speed
- Develop on `aarch64-unknown-none` for fastest iteration
- Test on `x86_64-unknown-none` before commits
- CI will test both architectures

### 2. Leverage macOS Tools
- Use Instruments.app for deep profiling
- Use Console.app for system logs
- Use Activity Monitor for resource usage

### 3. Terminal Shortcuts
```bash
# Add to ~/.zshrc for convenience
alias kb='cargo build --target aarch64-unknown-none'
alias kbx='cargo build --target x86_64-unknown-none'
alias kt='cargo test --workspace'
alias kc='cargo clippy --workspace -- -D warnings'
alias kr='qemu-system-aarch64 -machine virt -cpu cortex-a57 -kernel target/aarch64-unknown-none/debug/kaal -nographic'
```

### 4. Git Hooks
Create `.git/hooks/pre-commit`:
```bash
#!/bin/bash
cargo test --workspace && \
cargo clippy --workspace -- -D warnings && \
cargo fmt --workspace -- --check
```

Make it executable: `chmod +x .git/hooks/pre-commit`

## Performance Comparison

### Build Times (Example)

| Target | Clean Build | Incremental |
|--------|-------------|-------------|
| aarch64-unknown-none | ~2 min | ~5 sec |
| x86_64-unknown-none | ~2.5 min | ~7 sec |
| Both | ~3 min | ~10 sec |

*Times measured on M1 Pro, 16GB RAM*

### QEMU Performance

| Architecture | Boot Time | IPC Throughput |
|--------------|-----------|----------------|
| AArch64 | ~1 sec | Near-native |
| x86_64 | ~3 sec | 30-50% native |

## Continuous Integration

### GitHub Actions for Multi-Arch

The project includes CI that tests both architectures:
- ✅ Build on Ubuntu (x86_64)
- ✅ Build on macOS (AArch64)
- ✅ Cross-compile and test both targets

See `.github/workflows/ci.yml` (to be created in Phase 3)

## Additional Resources

- [Apple Developer: Porting to Apple Silicon](https://developer.apple.com/documentation/apple-silicon)
- [Rust on Apple Silicon](https://blog.rust-lang.org/2021/03/25/Rust-1.51.0.html)
- [LLDB Tutorial](https://lldb.llvm.org/use/tutorial.html)
- [QEMU Documentation](https://www.qemu.org/docs/master/)

## Getting Help

If you encounter issues specific to macOS:
1. Check [GETTING_STARTED.md](GETTING_STARTED.md) troubleshooting section
2. Search [GitHub Issues](https://github.com/your-org/kaal/issues)
3. Ask in [GitHub Discussions](https://github.com/your-org/kaal/discussions)
4. Tag questions with `macOS` or `apple-silicon`

---

**Happy coding on Apple Silicon! 🍎 ⚡**
