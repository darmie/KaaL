#!/bin/bash
#
# KaaL QEMU Test Script
# Tests bootimage.elf in QEMU ARM virt platform
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BOOTIMAGE="${1:-$PROJECT_ROOT/bootimage.elf}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  KaaL QEMU Test${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""

# Check bootimage exists
if [ ! -f "$BOOTIMAGE" ]; then
    echo -e "${RED}✗${NC} Bootimage not found: $BOOTIMAGE"
    echo ""
    echo "Run ./tools/build-bootimage.sh first to create the bootimage"
    exit 1
fi

echo -e "${GREEN}✓${NC} Found bootimage: $BOOTIMAGE"
file "$BOOTIMAGE"
ls -lh "$BOOTIMAGE"
echo ""

# Check QEMU
if ! command -v qemu-system-aarch64 &> /dev/null; then
    echo -e "${RED}✗${NC} qemu-system-aarch64 not found"
    echo "Install with: brew install qemu"
    exit 1
fi

echo -e "${GREEN}✓${NC} QEMU: $(qemu-system-aarch64 --version | head -1)"
echo ""

echo -e "${BLUE}Starting QEMU...${NC}"
echo "The system will run for 5 seconds to capture boot output"
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""

# Use serial output to file for proper capture on macOS
OUTPUT_FILE="/tmp/kaal-qemu-output.txt"
rm -f "$OUTPUT_FILE"

# Run QEMU in background with output to file
qemu-system-aarch64 \
    -machine virt,virtualization=on \
    -cpu cortex-a53 \
    -m 512M \
    -nographic \
    -serial file:"$OUTPUT_FILE" \
    -kernel "$BOOTIMAGE" &

QEMU_PID=$!

# Wait for output
sleep 5

# Kill QEMU
kill $QEMU_PID 2>/dev/null || true
wait $QEMU_PID 2>/dev/null || true

echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  QEMU Output${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"

if [ -f "$OUTPUT_FILE" ] && [ -s "$OUTPUT_FILE" ]; then
    cat "$OUTPUT_FILE"
    echo ""
    echo -e "${GREEN}✓${NC} Output captured successfully"
else
    echo -e "${RED}✗${NC} No output captured"
    echo ""
    echo "This suggests the elfloader is not executing properly."
    echo "Possible issues:"
    echo "  - Entry point not correctly set"
    echo "  - UART not initialized"
    echo "  - Code crashing before UART output"
fi

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
