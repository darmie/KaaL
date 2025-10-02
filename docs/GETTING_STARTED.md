# Getting Started with KaaL Development

This guide will help you get started with KaaL development, from setting up your environment to implementing your first component.

---

## Prerequisites

### Platform-Specific Setup

#### macOS (Apple Silicon / M1/M2/M3)

1. **Install Homebrew** (if not already installed)
   ```bash
   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
   ```

2. **Rust Toolchain** (1.70+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup default stable

   # Add ARM64 and x86_64 targets
   rustup target add aarch64-unknown-none
   rustup target add x86_64-unknown-none
   ```

3. **Development Tools**
   ```bash
   # Install build essentials via Homebrew
   brew install cmake git python3

   # Install QEMU for testing (with x86_64 and ARM emulation)
   brew install qemu

   # Verify QEMU installation
   qemu-system-x86_64 --version
   qemu-system-aarch64 --version
   ```

4. **seL4 Dependencies**
   ```bash
   # Install Python dependencies
   pip3 install --user sel4-deps camkes-deps

   # Install cross-compilation toolchain for x86_64
   brew install x86_64-elf-gcc

   # Or use LLVM (recommended for Apple Silicon)
   brew install llvm
   export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
   ```

5. **Optional but Recommended**
   ```bash
   # Install ARM cross-compiler (for AArch64 targets)
   brew install aarch64-elf-gcc

   # Install debugging tools
   brew install gdb lldb

   # Install performance tools
   brew install flamegraph cargo-instruments
   ```

#### Linux (Ubuntu/Debian)

1. **Rust Toolchain** (1.70+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup default stable
   ```

2. **Development Tools**
   ```bash
   # Install build essentials
   sudo apt update
   sudo apt install build-essential git cmake

   # Install QEMU for testing
   sudo apt install qemu-system-x86 qemu-system-arm qemu-system-aarch64
   ```

3. **seL4 Dependencies**
   ```bash
   # Install seL4 build dependencies
   sudo apt install python3-pip python3-dev
   pip3 install sel4-deps camkes-deps

   # Install cross-compilation toolchains
   sudo apt install gcc-aarch64-linux-gnu gcc-x86-64-linux-gnu
   ```

4. **Cross-Compilation Toolchains**
   ```bash
   # For x86_64
   rustup target add x86_64-unknown-none

   # For AArch64
   rustup target add aarch64-unknown-none
   ```

### Common Tools (All Platforms)

- **VS Code** with extensions:
  - rust-analyzer (Rust language support)
  - CodeLLDB (debugging)
  - Even Better TOML
  - Error Lens

- **Cargo extensions**:
  ```bash
  # Code coverage
  cargo install cargo-tarpaulin

  # Benchmarking
  cargo install cargo-criterion

  # Performance profiling
  cargo install cargo-flamegraph

  # Security auditing
  cargo install cargo-audit

  # Dependency tree visualization
  cargo install cargo-tree
  ```

### Verification

Verify your setup:

```bash
# Check Rust
rustc --version
cargo --version

# Check QEMU
qemu-system-x86_64 --version
qemu-system-aarch64 --version  # Should work on both platforms

# Check targets
rustup target list --installed

# Check seL4 tools
python3 -c "import sel4_deps; print('seL4 deps OK')"

# macOS specific: Check Rosetta 2 (for x86_64 emulation)
# This should be already installed on Apple Silicon Macs
arch
```

Expected output on Mac Silicon:
- `arch` should show `arm64`
- Both QEMU variants should be available
- Rust should show native `aarch64-apple-darwin` host

---

## Project Setup

### Clone the Repository

```bash
git clone https://github.com/your-org/kaal.git
cd kaal
```

### Build the Project

```bash
# Build all components
cargo build --workspace

# Run tests
cargo test --workspace

# Run linter
cargo clippy --workspace -- -D warnings

# Format code
cargo fmt --workspace
```

### Run in QEMU

```bash
# Currently we're in Phase 1, so this will be available later
# cargo run
```

---

## Project Structure

```
kaal/
├── Cargo.toml                 # Workspace configuration
├── README.md                  # Project overview
├── .CLAUDE                    # Coding standards
│
├── docs/                      # Documentation
│   ├── ARCHITECTURE.md        # Architecture overview
│   ├── IMPLEMENTATION_PLAN.md # Development roadmap
│   └── GETTING_STARTED.md     # This file
│
├── internal_resource/         # Internal documents
│   └── technical_arch_implementation.md
│
├── runtime/                   # Layer 1: Runtime Services
│   ├── cap_broker/           # Capability management
│   ├── ipc/                  # Shared memory IPC
│   ├── allocator/            # Memory allocation
│   └── dddk/                 # Driver Development Kit
│
├── components/                # Layer 3: System Services
│   ├── vfs/                  # Virtual file system
│   ├── posix/                # POSIX compatibility
│   ├── network/              # Network stack
│   └── drivers/              # Device drivers
│
├── tools/                     # Developer tooling
│   └── sel4-compose/         # CLI tool
│
├── examples/                  # Example applications
│   ├── 01-hello-world/
│   ├── 02-memory/
│   └── ...
│
└── tests/                     # Integration tests
    ├── integration/
    └── hardware_sim/
```

---

## Development Workflow

### 1. Pick a Task

Check the current phase in [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) and choose a task.

**Current Phase:** Phase 1 - Foundation (Weeks 1-8)

Active tasks:
- Capability Broker implementation
- Shared memory IPC
- Memory allocator
- DDDK framework

### 2. Create a Feature Branch

```bash
git checkout -b feature/capability-broker-init
```

### 3. Implement Following Standards

**Key Rules from [.CLAUDE](../.CLAUDE):**

1. **No Placeholders** - Every function must be fully implemented
2. **Error Handling** - Always use `Result`, never `unwrap()` in library code
3. **Testing** - Write unit tests, integration tests, and hardware sim tests
4. **Documentation** - Document all public APIs
5. **Safety** - Document all `unsafe` blocks with safety invariants

**Example:**

```rust
/// Allocates a capability slot from the broker
///
/// # Arguments
/// * `size` - Size of the capability in bytes
///
/// # Returns
/// Allocated capability slot
///
/// # Errors
/// Returns `CapabilityError::OutOfSlots` if no slots available
///
/// # Panics
/// Never panics
pub fn allocate_cap(&mut self, size: usize) -> Result<CapSlot> {
    // Real implementation, not placeholder
    if self.free_slots.is_empty() {
        return Err(CapabilityError::OutOfSlots);
    }

    let slot = self.free_slots.pop().unwrap(); // Safe: checked above
    slot.retype(size)?;

    Ok(slot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_cap_success() {
        let mut broker = CapBroker::new();
        let cap = broker.allocate_cap(4096).unwrap();
        assert!(cap.is_valid());
    }

    #[test]
    fn test_allocate_cap_out_of_slots() {
        let mut broker = CapBroker::with_capacity(1);
        let _cap1 = broker.allocate_cap(4096).unwrap();
        let result = broker.allocate_cap(4096);
        assert!(matches!(result, Err(CapabilityError::OutOfSlots)));
    }
}
```

### 4. Write Tests

**Unit Tests** (in same file):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Test implementation
    }

    #[test]
    fn test_error_handling() {
        // Test error cases
    }

    #[test]
    fn test_edge_cases() {
        // Test boundary conditions
    }
}
```

**Integration Tests** (in `tests/` directory):
```rust
// tests/integration/cap_broker_test.rs

#[test]
fn test_cap_broker_with_vfs() {
    // Test cross-component interaction
}
```

### 5. Run Pre-Commit Checks

```bash
# All tests must pass
cargo test --workspace

# No warnings allowed
cargo clippy --workspace -- -D warnings

# Code must be formatted
cargo fmt --workspace -- --check

# Coverage should be >80%
cargo tarpaulin --out Html --output-dir coverage
```

### 6. Create Pull Request

```bash
git add .
git commit -m "feat(cap_broker): implement capability allocation"
git push origin feature/capability-broker-init
```

Then create a PR on GitHub with:
- Clear description of changes
- Link to relevant issue
- Test results
- Documentation updates

---

## Common Tasks

### Adding a New Component

1. **Create the component directory:**
   ```bash
   mkdir -p components/my_component/src
   cd components/my_component
   ```

2. **Create Cargo.toml:**
   ```toml
   [package]
   name = "my-component"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   thiserror.workspace = true
   log.workspace = true
   ```

3. **Implement the component:**
   ```rust
   // src/lib.rs

   //! My Component - Brief description
   //!
   //! # Integration Points
   //! - Depends on: List dependencies
   //! - Provides to: List dependents

   use thiserror::Error;

   #[derive(Debug, Error)]
   pub enum MyComponentError {
       #[error("Something went wrong: {0}")]
       SomeError(String),
   }

   pub type Result<T> = core::result::Result<T, MyComponentError>;

   pub struct MyComponent {
       // Fields
   }

   impl MyComponent {
       pub fn new() -> Result<Self> {
           // Implementation
           Ok(Self { /* ... */ })
       }
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_my_component() {
           let component = MyComponent::new().unwrap();
           // Test assertions
       }
   }
   ```

4. **Add to workspace:**
   ```toml
   # In root Cargo.toml
   [workspace]
   members = [
       # ... existing members
       "components/my_component",
   ]
   ```

### Writing a Device Driver with DDDK

```rust
use dddk::prelude::*;

#[derive(Driver)]
#[pci(vendor = 0x1234, device = 0x5678)]
#[resources(mmio = "bar0", irq = "auto", dma = "4MB")]
pub struct MyDriver {
    #[mmio]
    regs: &'static mut MyRegisters,

    #[dma_ring(size = 256)]
    rx_ring: DmaRing<RxDescriptor>,
}

#[driver_impl]
impl MyDriver {
    #[init]
    fn initialize(&mut self) -> Result<()> {
        // Initialize hardware
        self.regs.control.write(CTRL_ENABLE);
        Ok(())
    }

    #[interrupt]
    fn handle_interrupt(&mut self) {
        // Handle interrupts
        let status = self.regs.status.read();
        if status & STATUS_RX != 0 {
            self.process_rx();
        }
    }

    fn process_rx(&mut self) {
        while let Some(desc) = self.rx_ring.next_complete() {
            // Process received data
        }
    }
}
```

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench --workspace

# Run specific benchmark
cargo bench --package cap-broker cap_allocation

# Generate flamegraph
cargo flamegraph --bench cap_allocation
```

---

## Debugging

### Using LLDB with QEMU (Recommended for macOS)

```bash
# Terminal 1: Start QEMU with GDB server (LLDB compatible)
qemu-system-x86_64 \
    -kernel target/x86_64-unknown-none/debug/kaal \
    -s -S \
    -display none \
    -serial stdio

# Terminal 2: Connect LLDB (native on macOS)
lldb target/x86_64-unknown-none/debug/kaal
(lldb) gdb-remote localhost:1234
(lldb) breakpoint set --name main
(lldb) continue

# Alternative: For AArch64 development on Apple Silicon
qemu-system-aarch64 \
    -machine virt -cpu cortex-a57 \
    -kernel target/aarch64-unknown-none/debug/kaal \
    -s -S \
    -nographic

lldb target/aarch64-unknown-none/debug/kaal
(lldb) gdb-remote localhost:1234
```

### Using GDB with QEMU (Linux or with brew gdb on macOS)

```bash
# Terminal 1: Start QEMU with GDB server
qemu-system-x86_64 \
    -kernel target/x86_64-unknown-none/debug/kaal \
    -s -S

# Terminal 2: Connect GDB
# On macOS, you may need to use gdb-multiarch or brew's gdb
gdb target/x86_64-unknown-none/debug/kaal
(gdb) target remote :1234
(gdb) break main
(gdb) continue

# macOS note: If using Homebrew GDB, you may need to sign it:
# https://sourceware.org/gdb/wiki/PermissionsDarwin
```

### VS Code Debugging

Create `.vscode/launch.json`:

**For macOS (using LLDB - CodeLLDB extension):**
```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "custom",
            "name": "Debug KaaL (x86_64)",
            "targetCreateCommands": [
                "target create ${workspaceFolder}/target/x86_64-unknown-none/debug/kaal"
            ],
            "processCreateCommands": [
                "gdb-remote localhost:1234"
            ],
            "preLaunchTask": "start-qemu-x86"
        },
        {
            "type": "lldb",
            "request": "custom",
            "name": "Debug KaaL (AArch64 - Native)",
            "targetCreateCommands": [
                "target create ${workspaceFolder}/target/aarch64-unknown-none/debug/kaal"
            ],
            "processCreateCommands": [
                "gdb-remote localhost:1234"
            ],
            "preLaunchTask": "start-qemu-aarch64"
        }
    ]
}
```

**For Linux (using GDB):**
```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "gdb",
            "request": "attach",
            "name": "Debug KaaL",
            "executable": "${workspaceFolder}/target/x86_64-unknown-none/debug/kaal",
            "target": "localhost:1234",
            "remote": true,
            "cwd": "${workspaceFolder}",
            "preLaunchTask": "start-qemu"
        }
    ]
}
```

Create `.vscode/tasks.json` for QEMU startup:
```json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "start-qemu-x86",
            "type": "shell",
            "command": "qemu-system-x86_64",
            "args": [
                "-kernel", "${workspaceFolder}/target/x86_64-unknown-none/debug/kaal",
                "-s", "-S",
                "-nographic"
            ],
            "isBackground": true,
            "problemMatcher": {
                "pattern": {
                    "regexp": "^$",
                    "file": 1,
                    "location": 2,
                    "message": 3
                },
                "background": {
                    "activeOnStart": true,
                    "beginsPattern": ".",
                    "endsPattern": "."
                }
            }
        },
        {
            "label": "start-qemu-aarch64",
            "type": "shell",
            "command": "qemu-system-aarch64",
            "args": [
                "-machine", "virt",
                "-cpu", "cortex-a57",
                "-kernel", "${workspaceFolder}/target/aarch64-unknown-none/debug/kaal",
                "-s", "-S",
                "-nographic"
            ],
            "isBackground": true,
            "problemMatcher": {
                "pattern": {
                    "regexp": "^$",
                    "file": 1,
                    "location": 2,
                    "message": 3
                },
                "background": {
                    "activeOnStart": true,
                    "beginsPattern": ".",
                    "endsPattern": "."
                }
            }
        }
    ]
}
```

### Logging

```rust
use log::{debug, info, warn, error};

fn my_function() {
    debug!("Debug information");
    info!("Informational message");
    warn!("Warning: something might be wrong");
    error!("Error occurred: {}", error);
}
```

Set log level:
```bash
RUST_LOG=debug cargo test
```

---

## Platform-Specific Notes

### macOS Apple Silicon Considerations

1. **Architecture Targets**
   - Native development: `aarch64-apple-darwin` (your Mac's native architecture)
   - KaaL targets: `aarch64-unknown-none` (bare metal ARM) or `x86_64-unknown-none` (emulated via QEMU)
   - You can develop and test both architectures on Apple Silicon

2. **QEMU Performance**
   - AArch64 QEMU (`qemu-system-aarch64`): Near-native speed (KVM acceleration on Linux, TCG on macOS)
   - x86_64 QEMU (`qemu-system-x86_64`): Slower (emulation, no hardware acceleration on ARM)
   - For fastest iteration: Develop on AArch64 target when possible

3. **Cross-Compilation**
   ```bash
   # Building for x86_64 from ARM Mac
   cargo build --target x86_64-unknown-none

   # Building for AArch64 (native architecture)
   cargo build --target aarch64-unknown-none

   # Both will work, x86_64 may compile slower
   ```

4. **LLVM vs GCC**
   - LLVM (recommended): Native on macOS, better integration
   - GCC: Available via Homebrew but requires code signing for debugging

5. **Memory Tools**
   - Valgrind: Not available on Apple Silicon
   - Alternative: Use `cargo-instruments` (native macOS profiling)
   ```bash
   cargo install cargo-instruments
   cargo instruments --template Allocations
   ```

6. **Performance Profiling**
   ```bash
   # Use native macOS Instruments
   cargo instruments --template "Time Profiler" --bench cap_allocation

   # Or use flamegraph (cross-platform)
   cargo flamegraph --bench cap_allocation
   ```

### Troubleshooting

#### macOS: "xcrun: error: invalid active developer path"
```bash
xcode-select --install
```

#### macOS: GDB Code Signing Issues
Use LLDB instead, or follow: https://sourceware.org/gdb/wiki/PermissionsDarwin

#### macOS: Python `sel4-deps` Installation Fails
```bash
# Use Python 3 from Homebrew
brew install python3
pip3 install --user --break-system-packages sel4-deps camkes-deps
```

#### Linux: QEMU Not Starting
```bash
# Check QEMU installation
which qemu-system-x86_64

# Install missing architecture support
sudo apt install qemu-system-arm qemu-system-aarch64
```

#### Cross-Platform: Cargo Build Fails
```bash
# Update Rust
rustup update

# Clean build artifacts
cargo clean

# Rebuild with verbose output
cargo build --verbose
```

## Tips and Best Practices

### 1. Read Documentation First
- [ARCHITECTURE.md](ARCHITECTURE.md) for system design
- [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for current priorities
- [.CLAUDE](../.CLAUDE) for coding standards

### 2. Start Small
- Begin with unit tests
- Then integration tests
- Finally full system tests

### 3. Ask Questions
- Open GitHub discussions
- Check existing issues
- Review code comments

### 4. Follow the Standards
- No `unwrap()` in library code
- Document all `unsafe` blocks
- Write comprehensive tests
- Keep functions small (<50 lines)

### 5. Optimize Later
- Correctness first
- Then benchmark
- Then optimize hot paths

---

## Getting Help

- **Documentation:** [docs/](.)
- **Issues:** [GitHub Issues](https://github.com/your-org/kaal/issues)
- **Discussions:** [GitHub Discussions](https://github.com/your-org/kaal/discussions)
- **Email:** team@kaal.dev

---

## Next Steps

1. **Set up your environment** using the instructions above
2. **Read** [ARCHITECTURE.md](ARCHITECTURE.md) to understand the system
3. **Pick a task** from [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md)
4. **Start coding** following [.CLAUDE](../.CLAUDE) standards
5. **Submit a PR** with tests and documentation

Welcome to KaaL development! 🚀
