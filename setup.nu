#!/usr/bin/env nu
# KaaL Framework Development Setup Script
# Installs all required dependencies for building KaaL on different platforms

def main [
    --skip-rust       # Skip Rust toolchain setup
    --skip-platform   # Skip platform-specific dependencies
    --skip-nushell    # Skip Nushell installation check
    --verify-only     # Only verify existing installation
] {
    print "🚀 KaaL Framework Setup"
    print "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"

    let os = (sys host | get name)
    print $"Detected OS: ($os)\n"

    if not $skip_nushell {
        check-nushell
    }

    if not $skip_rust {
        setup-rust $verify_only
    }

    if not $skip_platform {
        setup-platform-deps $os $verify_only
    }

    print "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    print "✅ Setup complete!\n"
    print "Next steps:"
    print "  1. Build for QEMU: nu build.nu"
    print "  2. Build and run:  nu build.nu --run"
    print "  3. Run tests:      cargo test --workspace"
    print "  4. Verify kernel:  nu scripts/verify.nu"
}

# Check Nushell installation
def check-nushell [] {
    print "📦 Checking Nushell..."

    try {
        let version = (version | get version)
        print $"  ✅ Nushell ($version) installed"
    } catch {
        print "  ❌ Nushell not found!"
        print "\nInstall Nushell:"
        print "  macOS:   brew install nushell"
        print "  Linux:   cargo install nu"
        print "  Windows: winget install nushell"
        print "  Or visit: https://www.nushell.sh/book/installation.html"
        exit 1
    }
}

# Setup Rust toolchain
def setup-rust [verify_only: bool] {
    print "\n📦 Rust Toolchain Setup"
    print "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Check rustup
    let has_rustup = (which rustup | length) > 0

    if not $has_rustup {
        if $verify_only {
            print "  ❌ rustup not found"
            exit 1
        }

        print "  ⚠️  rustup not found, installing..."
        print "\nRun this command to install rustup:"
        print "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        print "\nOr visit: https://rustup.rs/"
        exit 1
    }

    print "  ✅ rustup found"

    # Install nightly toolchain
    if not $verify_only {
        print "  📥 Installing nightly toolchain..."
        rustup toolchain install nightly
        rustup default nightly
    }

    # Verify nightly
    let rust_version = (rustc --version)
    if ($rust_version | str contains "nightly") {
        print $"  ✅ Rust nightly: ($rust_version)"
    } else {
        print $"  ⚠️  Current: ($rust_version)"
        if not $verify_only {
            print "  Setting nightly as default..."
            rustup default nightly
        }
    }

    # Install required targets
    print "  📥 Installing ARM64 bare-metal target..."
    if not $verify_only {
        rustup target add aarch64-unknown-none
    }

    let targets = (rustup target list --installed | lines)
    if ($targets | any {|t| $t == "aarch64-unknown-none"}) {
        print "  ✅ aarch64-unknown-none target installed"
    } else {
        print "  ❌ aarch64-unknown-none target missing"
        if $verify_only {
            exit 1
        }
    }

    # Install cargo components
    print "  📥 Installing cargo components..."
    if not $verify_only {
        rustup component add rust-src
        rustup component add llvm-tools-preview
    }

    print "  ✅ Rust toolchain ready"
}

# Setup platform-specific dependencies
def setup-platform-deps [os: string, verify_only: bool] {
    print "\n📦 Platform Dependencies"
    print "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    match $os {
        "Darwin" => { setup-macos $verify_only }
        "Linux" => { setup-linux $verify_only }
        "Windows" => { setup-windows $verify_only }
        _ => {
            print $"  ⚠️  Unknown OS: ($os)"
            print "  Please install dependencies manually:"
            print "    - QEMU (qemu-system-aarch64)"
            print "    - Device Tree Compiler (dtc)"
            print "    - LLVM (llvm-objcopy)"
        }
    }
}

# macOS setup
def setup-macos [verify_only: bool] {
    print "  🍎 macOS detected"

    # Check Homebrew
    let has_brew = (which brew | length) > 0
    if not $has_brew {
        print "  ⚠️  Homebrew not found"
        print "\nInstall Homebrew:"
        print '  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'
        if $verify_only {
            exit 1
        }
        return
    }

    print "  ✅ Homebrew found"

    # Required packages
    let packages = [
        ["qemu", "QEMU emulator"],
        ["dtc", "Device Tree Compiler"],
        ["llvm", "LLVM toolchain (for llvm-objcopy)"]
    ]

    for pkg in $packages {
        let name = ($pkg | get 0)
        let desc = ($pkg | get 1)

        print $"  📦 Checking ($desc)..."

        let installed = (brew list | lines | any {|p| $p == $name})
        if $installed {
            print $"    ✅ ($name) installed"
        } else {
            if $verify_only {
                print $"    ❌ ($name) not installed"
            } else {
                print $"    📥 Installing ($name)..."
                brew install $name
            }
        }
    }

    # Verify QEMU
    verify-qemu
}

# Linux setup
def setup-linux [verify_only: bool] {
    print "  🐧 Linux detected"

    # Detect package manager
    let has_apt = (which apt-get | length) > 0
    let has_dnf = (which dnf | length) > 0
    let has_pacman = (which pacman | length) > 0

    if $has_apt {
        setup-linux-apt $verify_only
    } else if $has_dnf {
        setup-linux-dnf $verify_only
    } else if $has_pacman {
        setup-linux-pacman $verify_only
    } else {
        print "  ⚠️  Unknown package manager"
        print "  Please install manually:"
        print "    - qemu-system-aarch64"
        print "    - device-tree-compiler"
        print "    - llvm"
    }

    verify-qemu
}

# Linux APT setup (Debian/Ubuntu)
def setup-linux-apt [verify_only: bool] {
    print "  📦 Using apt package manager"

    let packages = [
        "qemu-system-aarch64",
        "device-tree-compiler",
        "llvm"
    ]

    if not $verify_only {
        print "  📥 Updating package list..."
        sudo apt-get update
    }

    for pkg in $packages {
        let installed = (do -i { dpkg -l $pkg | complete } | get exit_code) == 0

        if $installed {
            print $"  ✅ ($pkg) installed"
        } else {
            if $verify_only {
                print $"  ❌ ($pkg) not installed"
            } else {
                print $"  📥 Installing ($pkg)..."
                sudo apt-get install -y $pkg
            }
        }
    }
}

# Linux DNF setup (Fedora/RHEL)
def setup-linux-dnf [verify_only: bool] {
    print "  📦 Using dnf package manager"

    let packages = [
        "qemu-system-aarch64",
        "dtc",
        "llvm"
    ]

    for pkg in $packages {
        let installed = (do -i { dnf list installed $pkg | complete } | get exit_code) == 0

        if $installed {
            print $"  ✅ ($pkg) installed"
        } else {
            if $verify_only {
                print $"  ❌ ($pkg) not installed"
            } else {
                print $"  📥 Installing ($pkg)..."
                sudo dnf install -y $pkg
            }
        }
    }
}

# Linux Pacman setup (Arch)
def setup-linux-pacman [verify_only: bool] {
    print "  📦 Using pacman package manager"

    let packages = [
        "qemu-system-aarch64",
        "dtc",
        "llvm"
    ]

    for pkg in $packages {
        let installed = (do -i { pacman -Q $pkg | complete } | get exit_code) == 0

        if $installed {
            print $"  ✅ ($pkg) installed"
        } else {
            if $verify_only {
                print $"  ❌ ($pkg) not installed"
            } else {
                print $"  📥 Installing ($pkg)..."
                sudo pacman -S --noconfirm $pkg
            }
        }
    }
}

# Windows setup
def setup-windows [verify_only: bool] {
    print "  🪟 Windows detected"
    print "  ⚠️  Windows support is experimental"
    print "\nRecommended approach:"
    print "  1. Use WSL2 (Windows Subsystem for Linux)"
    print "  2. Run this setup script inside WSL2"
    print "\nOr install manually:"
    print "  - QEMU: https://qemu.weilnetz.de/w64/"
    print "  - LLVM: https://releases.llvm.org/"
    print "  - DTC: Build from source or use WSL2"
}

# Verify QEMU installation
def verify-qemu [] {
    print "\n  🔍 Verifying QEMU..."

    let has_qemu = (which qemu-system-aarch64 | length) > 0
    if $has_qemu {
        let version = (qemu-system-aarch64 --version | lines | get 0)
        print $"  ✅ QEMU: ($version)"
    } else {
        print "  ❌ qemu-system-aarch64 not found in PATH"
        exit 1
    }
}
