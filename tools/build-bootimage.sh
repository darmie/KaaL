#!/usr/bin/env bash
#
# KaaL Bootable Image Builder
#
# This script orchestrates the complete build process using Docker to create
# a bootable seL4 image with KaaL's Rust elfloader.
#
# Usage:
#   ./tools/build-bootimage.sh [options]
#
# Options:
#   --project PATH  Path to project/example (default: examples/bootable-demo)
#   --lib NAME      Root task library name (default: libkaal_bootable_demo.a)
#   --clean         Clean temporary workspace before build
#   --keep-workspace Keep temporary build workspace after completion
#   --test          Run QEMU test after build
#   --output PATH   Output path for bootimage.elf (default: ./bootimage.elf)
#

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
WORKSPACE_DIR="$PROJECT_ROOT/.build-workspace"
OUTPUT_IMAGE="$PROJECT_ROOT/bootimage.elf"
DOCKER_IMAGE="kaal-builder"
CLEAN_WORKSPACE=false
KEEP_WORKSPACE=false
RUN_QEMU_TEST=false
PROJECT_PATH=""
ROOT_TASK_LIB=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --project)
            PROJECT_PATH="$2"
            shift 2
            ;;
        --lib)
            ROOT_TASK_LIB="$2"
            shift 2
            ;;
        --clean)
            CLEAN_WORKSPACE=true
            shift
            ;;
        --keep-workspace)
            KEEP_WORKSPACE=true
            shift
            ;;
        --test)
            RUN_QEMU_TEST=true
            shift
            ;;
        --output)
            OUTPUT_IMAGE="$2"
            shift 2
            ;;
        --help)
            grep '^#' "$0" | grep -v '#!/' | sed 's/^# //'
            exit 0
            ;;
        *)
            echo -e "${RED}Error: Unknown option $1${NC}"
            exit 1
            ;;
    esac
done

# Helper functions
log_info() {
    echo -e "${BLUE}ℹ${NC} $*"
}

log_success() {
    echo -e "${GREEN}✓${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $*"
}

log_error() {
    echo -e "${RED}✗${NC} $*"
}

print_banner() {
    echo -e "${BLUE}"
    echo "═══════════════════════════════════════════════════════════"
    echo "  KaaL Bootable Image Builder v1.0"
    echo "═══════════════════════════════════════════════════════════"
    echo -e "${NC}"
}

cleanup_workspace() {
    if [ "$KEEP_WORKSPACE" = false ] && [ -d "$WORKSPACE_DIR" ]; then
        log_info "Cleaning up workspace..."
        rm -rf "$WORKSPACE_DIR"
        log_success "Workspace cleaned"
    fi
}

# Trap to cleanup on exit
trap cleanup_workspace EXIT

# Main build process
main() {
    print_banner

    # Step 1: Prepare workspace
    log_info "Preparing build workspace at $WORKSPACE_DIR"

    if [ "$CLEAN_WORKSPACE" = true ] && [ -d "$WORKSPACE_DIR" ]; then
        log_info "Removing existing workspace..."
        rm -rf "$WORKSPACE_DIR"
    fi

    mkdir -p "$WORKSPACE_DIR"
    log_success "Workspace ready"

    # Validate required arguments
    if [ -z "$PROJECT_PATH" ]; then
        log_error "PROJECT_PATH is required. Use --project <path>"
        log_error "Example: ./tools/build-bootimage.sh --project examples/bootable-demo --lib libkaal_bootable_demo.a"
        exit 1
    fi

    if [ -z "$ROOT_TASK_LIB" ]; then
        log_error "ROOT_TASK_LIB is required. Use --lib <library-name>"
        log_error "Example: ./tools/build-bootimage.sh --project examples/bootable-demo --lib libkaal_bootable_demo.a"
        exit 1
    fi

    # Step 2: Build Docker image
    log_info "Building Docker image: $DOCKER_IMAGE"
    log_info "  Project: $PROJECT_PATH"
    log_info "  Library: $ROOT_TASK_LIB"
    echo ""

    docker build -f "$PROJECT_ROOT/tools/Dockerfile.bootimage" \
        --build-arg PROJECT_PATH="$PROJECT_PATH" \
        --build-arg ROOT_TASK_LIB="$ROOT_TASK_LIB" \
        -t "$DOCKER_IMAGE" \
        "$PROJECT_ROOT" \
        2>&1 | tee "$WORKSPACE_DIR/docker-build.log"

    if [ ${PIPESTATUS[0]} -ne 0 ]; then
        log_error "Docker build failed. Check $WORKSPACE_DIR/docker-build.log"
        exit 1
    fi

    log_success "Docker image built successfully"

    # Step 3: Extract bootable image from Docker container
    log_info "Extracting bootable image..."

    # Create a temporary container
    CONTAINER_ID=$(docker create "$DOCKER_IMAGE")

    # Copy bootimage.elf from container
    docker cp "$CONTAINER_ID:/output/bootimage.elf" "$OUTPUT_IMAGE"

    # Copy build artifacts for inspection
    docker cp "$CONTAINER_ID:/output/build-info.txt" "$WORKSPACE_DIR/build-info.txt" || true
    docker cp "$CONTAINER_ID:/output/bootimage.payload" "$WORKSPACE_DIR/bootimage.payload" || true

    # Remove temporary container
    docker rm "$CONTAINER_ID" > /dev/null

    log_success "Bootable image extracted to: $OUTPUT_IMAGE"

    # Step 4: Display build information
    log_info "Build Information:"
    echo ""

    if [ -f "$OUTPUT_IMAGE" ]; then
        echo "  Output file: $OUTPUT_IMAGE"
        echo "  Size: $(du -h "$OUTPUT_IMAGE" | cut -f1)"
        echo "  Type: $(file "$OUTPUT_IMAGE" | cut -d: -f2)"
        echo ""
    fi

    if [ -f "$WORKSPACE_DIR/build-info.txt" ]; then
        cat "$WORKSPACE_DIR/build-info.txt"
        echo ""
    fi

    # Step 5: Optional QEMU test
    if [ "$RUN_QEMU_TEST" = true ]; then
        log_info "Starting QEMU test..."
        echo ""
        log_warn "Press Ctrl-A then X to exit QEMU"
        echo ""
        sleep 2

        qemu-system-aarch64 \
            -machine virt \
            -cpu cortex-a53 \
            -m 512M \
            -nographic \
            -kernel "$OUTPUT_IMAGE"
    else
        log_info "Skipping QEMU test (use --test to enable)"
    fi

    # Step 6: Success summary
    echo ""
    echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}  Build Complete!${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Test in QEMU:"
    echo "     qemu-system-aarch64 -machine virt -cpu cortex-a53 -nographic -kernel $OUTPUT_IMAGE"
    echo ""
    echo "  2. Inspect build artifacts:"
    echo "     ls -lh $WORKSPACE_DIR/"
    echo ""

    if [ "$KEEP_WORKSPACE" = false ]; then
        echo "  Note: Workspace will be cleaned on exit (use --keep-workspace to preserve)"
    else
        echo "  Workspace preserved at: $WORKSPACE_DIR"
    fi
    echo ""
}

# Run main function
main
