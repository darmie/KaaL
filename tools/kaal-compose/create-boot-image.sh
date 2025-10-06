#!/bin/bash
# KaaL Boot Image Creator
# Replicates seL4's DeclareRootserver process to create bootable images
#
# Usage: create-boot-image.sh <kernel.elf> <roottask.elf> <output-image>

set -e

if [ "$#" -ne 3 ]; then
    echo "Usage: $0 <kernel.elf> <roottask.elf> <output-image>"
    exit 1
fi

KERNEL_ELF="$1"
ROOTTASK_ELF="$2"
OUTPUT_IMAGE="$3"

WORK_DIR=$(mktemp -d)
trap "rm -rf $WORK_DIR" EXIT

echo "🔧 KaaL Boot Image Creator"
echo "   Kernel: $KERNEL_ELF"
echo "   Root Task: $ROOTTASK_ELF"
echo "   Output: $OUTPUT_IMAGE"
echo ""

# Step 1: Strip binaries
echo "📦 Step 1: Stripping binaries..."
aarch64-linux-gnu-strip "$KERNEL_ELF" -o "$WORK_DIR/kernel.elf"
aarch64-linux-gnu-strip "$ROOTTASK_ELF" -o "$WORK_DIR/roottask"

# Step 2: Create CPIO archive
echo "📦 Step 2: Creating CPIO archive..."
cd "$WORK_DIR"
echo "kernel.elf" > cpio_files.txt
echo "roottask" >> cpio_files.txt

# Create CPIO archive (newc format, no compression)
cpio -o -H newc < cpio_files.txt > archive.cpio 2>/dev/null <<EOF
kernel.elf
roottask
EOF

# Convert CPIO to object file that can be linked
echo "📦 Step 3: Converting CPIO to linkable object..."
aarch64-linux-gnu-objcopy -I binary -O elf64-littleaarch64 -B aarch64 \
    --rename-section .data=.archive \
    --redefine-sym _binary_archive_cpio_start=_archive_start \
    --redefine-sym _binary_archive_cpio_end=_archive_end \
    --redefine-sym _binary_archive_cpio_size=_archive_size \
    archive.cpio archive.o

# Step 4: Get elfloader source (if not already present)
if [ ! -d "/tmp/seL4_tools" ]; then
    echo "📦 Step 4: Cloning elfloader source..."
    git clone --depth 1 https://github.com/seL4/seL4_tools.git /tmp/seL4_tools
fi

echo "✅ CPIO archive created successfully"
echo "   Archive object: $WORK_DIR/archive.o"
echo ""
echo "⚠️  Next step: Build elfloader with this archive embedded"
echo "   This requires the elfloader source and proper toolchain setup"
