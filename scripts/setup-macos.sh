#!/bin/bash
# Setup script for KaaL development on macOS (Apple Silicon)

set -e  # Exit on error

echo "🚀 Setting up KaaL development environment for macOS..."
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're on Apple Silicon
ARCH=$(uname -m)
if [ "$ARCH" != "arm64" ]; then
    echo -e "${YELLOW}Warning: This script is optimized for Apple Silicon (ARM64)${NC}"
    echo "Current architecture: $ARCH"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Step 1: Check for Homebrew
echo -e "${GREEN}[1/8]${NC} Checking Homebrew..."
if ! command -v brew &> /dev/null; then
    echo "Homebrew not found. Installing..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    # Add Homebrew to PATH for Apple Silicon
    echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
    eval "$(/opt/homebrew/bin/brew shellenv)"
else
    echo "✓ Homebrew already installed"
fi

# Step 2: Check for Rust
echo -e "${GREEN}[2/8]${NC} Checking Rust installation..."
if ! command -v rustc &> /dev/null; then
    echo "Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "✓ Rust already installed ($(rustc --version))"
fi

# Step 3: Install development tools
echo -e "${GREEN}[3/8]${NC} Installing development tools..."
brew_packages=(
    cmake
    python3
    qemu
    llvm
)

for package in "${brew_packages[@]}"; do
    if brew list "$package" &>/dev/null; then
        echo "✓ $package already installed"
    else
        echo "Installing $package..."
        brew install "$package"
    fi
done

# Step 4: Add Rust targets
echo -e "${GREEN}[4/8]${NC} Adding Rust compilation targets..."
rustup target add aarch64-unknown-none
rustup target add x86_64-unknown-none
echo "✓ Targets added"

# Step 5: Install Python dependencies
echo -e "${GREEN}[5/8]${NC} Installing seL4 Python dependencies..."
pip3 install --user --break-system-packages sel4-deps camkes-deps 2>/dev/null || \
pip3 install --user sel4-deps camkes-deps
echo "✓ Python dependencies installed"

# Step 6: Install Cargo tools
echo -e "${GREEN}[6/8]${NC} Installing Cargo development tools..."
cargo_tools=(
    cargo-tarpaulin    # Code coverage
    cargo-criterion    # Benchmarking
    cargo-flamegraph   # Profiling
    cargo-audit        # Security auditing
    cargo-instruments  # macOS-specific profiling
)

for tool in "${cargo_tools[@]}"; do
    if cargo install --list | grep -q "^$tool v"; then
        echo "✓ $tool already installed"
    else
        echo "Installing $tool..."
        cargo install "$tool" || echo -e "${YELLOW}Warning: Failed to install $tool${NC}"
    fi
done

# Step 7: Configure environment
echo -e "${GREEN}[7/8]${NC} Configuring environment..."

# Add LLVM to PATH
LLVM_PATH="/opt/homebrew/opt/llvm/bin"
if [[ ":$PATH:" != *":$LLVM_PATH:"* ]]; then
    echo "export PATH=\"$LLVM_PATH:\$PATH\"" >> ~/.zshrc
    export PATH="$LLVM_PATH:$PATH"
    echo "✓ LLVM added to PATH (restart shell or run: source ~/.zshrc)"
fi

# Step 8: Verify installation
echo -e "${GREEN}[8/8]${NC} Verifying installation..."
echo ""

echo "Checking tools:"
echo "  Rust:    $(rustc --version)"
echo "  Cargo:   $(cargo --version)"
echo "  QEMU x86:   $(qemu-system-x86_64 --version | head -n1)"
echo "  QEMU ARM:   $(qemu-system-aarch64 --version | head -n1)"
echo "  Python:  $(python3 --version)"
echo "  LLVM:    $(llvm-config --version 2>/dev/null || echo 'not in PATH yet')"
echo ""

echo "Rust targets installed:"
rustup target list --installed | grep -E "(aarch64|x86_64)-unknown-none"
echo ""

# Test QEMU
echo "Testing QEMU..."
qemu-system-x86_64 -version > /dev/null && echo "  ✓ QEMU x86_64 works"
qemu-system-aarch64 -version > /dev/null && echo "  ✓ QEMU AArch64 works"
echo ""

# Success message
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✨ Setup complete!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Next steps:"
echo "  1. Restart your terminal or run: source ~/.zshrc"
echo "  2. Navigate to the project: cd $(pwd)"
echo "  3. Build the project: cargo build --workspace"
echo "  4. Run tests: cargo test --workspace"
echo ""
echo "Optional VS Code extensions (recommended):"
echo "  code --install-extension rust-lang.rust-analyzer"
echo "  code --install-extension vadimcn.vscode-lldb"
echo "  code --install-extension tamasfe.even-better-toml"
echo ""
echo "For more information, see: docs/GETTING_STARTED.md"
echo ""
