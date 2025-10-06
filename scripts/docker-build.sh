#!/bin/bash
# Build KaaL using Docker on Mac Silicon

set -e

echo "╔════════════════════════════════════════════════════════╗"
echo "║   KaaL Docker Build for Mac Silicon                    ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker Desktop."
    exit 1
fi

echo "✅ Docker is running"
echo ""

# Build the Docker image (includes seL4 kernel build)
echo "📦 Building Docker image (this may take 10-15 minutes first time)..."

# Default build args (can be overridden with env vars)
PLATFORM=${PLATFORM:-qemu-arm-virt}
KERNEL_ARCH_ARM=${KERNEL_ARCH_ARM:-ON}
SEL4_ARCH_AARCH64=${SEL4_ARCH_AARCH64:-ON}
WORD_SIZE=${WORD_SIZE:-64}
ARM_HYP=${ARM_HYP:-OFF}
SIMULATION=${SIMULATION:-TRUE}
CROSS_COMPILER=${CROSS_COMPILER:-aarch64-linux-gnu-}

docker build \
  --build-arg PLATFORM=${PLATFORM} \
  --build-arg KERNEL_ARCH_ARM=${KERNEL_ARCH_ARM} \
  --build-arg SEL4_ARCH_AARCH64=${SEL4_ARCH_AARCH64} \
  --build-arg WORD_SIZE=${WORD_SIZE} \
  --build-arg ARM_HYP=${ARM_HYP} \
  --build-arg SIMULATION=${SIMULATION} \
  --build-arg CROSS_COMPILER=${CROSS_COMPILER} \
  -t kaal-dev:latest .

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Docker image built successfully!"
    echo ""

    # Extract built artifacts to host
    echo "📦 Extracting build artifacts to host..."
    mkdir -p build

    # Create temporary container and copy artifacts
    CONTAINER_ID=$(docker create kaal-dev:latest)
    docker cp "$CONTAINER_ID:/workspace/build/." build/ 2>/dev/null || true
    docker cp "$CONTAINER_ID:/workspace/target/aarch64-sel4-microkit/release/minimal_microkit.elf" build/ 2>/dev/null || true
    docker rm "$CONTAINER_ID" > /dev/null

    echo "✅ Build artifacts extracted to ./build/"
    ls -lh build/ 2>/dev/null || echo "No artifacts found"
    echo ""
    echo "Next steps:"
    echo "  1. Run: ./scripts/docker-shell.sh    # Enter development shell"
    echo "  2. Run: ./scripts/docker-test.sh     # Run QEMU tests"
else
    echo ""
    echo "❌ Docker build failed"
    exit 1
fi
