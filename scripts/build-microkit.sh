#!/bin/bash
# Build KaaL with seL4 Microkit (Default Mode)

set -e

echo "🚀 Building KaaL with seL4 Microkit..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check if microkit SDK is available
if [ ! -d "external/microkit-sdk" ]; then
    echo "⚠️  Microkit SDK not found. Downloading..."
    mkdir -p external
    cd external

    # Download appropriate SDK for platform
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "📦 Downloading macOS SDK..."
        # Note: Microkit may not have macOS SDK, use Linux in Docker/VM
        echo "❌ Microkit doesn't support macOS natively."
        echo ""
        echo "Options for macOS development:"
        echo "   1. Use Docker: docker run -it --rm -v \$(pwd):/kaal rust:latest"
        echo "   2. Use Lima VM: brew install lima && limactl start"
        echo "   3. Cross-compile from macOS (advanced)"
        echo ""
        echo "For testing, you can use mock mode:"
        echo "   cargo test --features mock-sel4"
        exit 1
    else
        echo "📦 Downloading Microkit SDK for Linux..."
        wget https://github.com/seL4/microkit/releases/download/1.4.0/microkit-sdk-1.4.0-linux-x86-64.tar.gz
        tar xf microkit-sdk-1.4.0-linux-x86-64.tar.gz
        mv microkit-sdk-1.4.0 microkit-sdk
        rm microkit-sdk-1.4.0-linux-x86-64.tar.gz
    fi

    cd ..
fi

# Build with microkit feature (default)
echo "Building KaaL components..."
cargo build "$@"

echo ""
echo "✅ Build complete!"
echo "   Mode: seL4 Microkit (default)"
echo "   Target: $(rustc -vV | grep host | cut -d' ' -f2)"
echo ""
echo "Next steps:"
echo "  1. Create system.xml configuration"
echo "  2. Build system image: microkit build system.xml"
echo "  3. Run in QEMU: qemu-system-aarch64 -M virt -cpu cortex-a53 -kernel loader.img"
