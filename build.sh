#!/bin/bash
# KaaL Build System - Config-driven multi-platform build

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$PROJECT_ROOT"

# =============================================================================
# Configuration Parser (TOML-lite for bash)
# =============================================================================

get_config() {
    local section="$1"
    local key="$2"
    local config_file="$PROJECT_ROOT/build-config.toml"

    # Parse TOML: find [section], then find key = "value" (BSD awk compatible)
    awk -v section="$section" -v key="$key" '
        /^\[/ {
            gsub(/^\[|\].*$/, "")
            current_section = $0
        }
        current_section == section && $0 ~ "^"key" " {
            # Extract value between quotes
            if (match($0, /"[^"]*"/)) {
                val = substr($0, RSTART+1, RLENGTH-2)
                print val
            }
        }
    ' "$config_file"
}

# Parse command-line arguments
PLATFORM=""
VERBOSE=0

while [[ $# -gt 0 ]]; do
    case $1 in
        --platform)
            PLATFORM="$2"
            shift 2
            ;;
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--platform <platform>] [-v|--verbose]"
            exit 1
            ;;
    esac
done

# Get default platform if not specified
if [ -z "$PLATFORM" ]; then
    PLATFORM=$(get_config "build" "default_platform")
fi

echo "══════════════════════════════════════════════════════════"
echo "  KaaL Build System"
echo "  Platform: $PLATFORM"
echo "══════════════════════════════════════════════════════════"

# =============================================================================
# Load platform configuration
# =============================================================================

PLATFORM_NAME=$(get_config "platform.$PLATFORM" "name")
RAM_BASE=$(get_config "platform.$PLATFORM" "ram_base")
RAM_SIZE=$(get_config "platform.$PLATFORM" "ram_size")
DTB_OFFSET=$(get_config "platform.$PLATFORM" "dtb_offset")
ELFLOADER_OFFSET=$(get_config "platform.$PLATFORM" "elfloader_offset")
KERNEL_OFFSET=$(get_config "platform.$PLATFORM" "kernel_offset")
STACK_TOP_OFFSET=$(get_config "platform.$PLATFORM" "stack_top_offset")
KERNEL_STACK_SIZE=$(get_config "build" "kernel_stack_size")

# Calculate absolute addresses
ELFLOADER_ADDR=$(printf "0x%x" $((RAM_BASE + ELFLOADER_OFFSET)))
KERNEL_ADDR=$(printf "0x%x" $((RAM_BASE + KERNEL_OFFSET)))
STACK_TOP=$(printf "0x%x" $((RAM_BASE + STACK_TOP_OFFSET)))

[ $VERBOSE -eq 1 ] && {
    echo "Configuration:"
    echo "  RAM Base:       $RAM_BASE"
    echo "  Elfloader:      $ELFLOADER_ADDR"
    echo "  Kernel:         $KERNEL_ADDR"
    echo "  Stack Top:      $STACK_TOP"
    echo ""
}

# Load build configuration
BUILD_DIR="$PROJECT_ROOT/$(get_config "build" "output_dir")"
KERNEL_SOURCE_DIR="$(get_config "build" "kernel_source_dir")"
ELFLOADER_SOURCE_DIR="$(get_config "build" "elfloader_source_dir")"
ROOTTASK_SOURCE_DIR="$(get_config "build" "roottask_source_dir")"

mkdir -p "$BUILD_DIR"

# =============================================================================
# Generate kernel linker script
# =============================================================================

echo "Generating kernel linker script..."
rm -f kernel/kernel.ld
cat > kernel/kernel.ld << KERNELLD
OUTPUT_FORMAT("elf64-littleaarch64")
OUTPUT_ARCH(aarch64)
ENTRY(_start)

SECTIONS
{
    . = $KERNEL_ADDR;
    _kernel_start = .;

    .text : {
        KEEP(*(.text._start))
        *(.text .text.*)
    }

    .rodata : ALIGN(4096) { *(.rodata .rodata.*) }
    .data : ALIGN(4096) { *(.data .data.*) }

    .bss : ALIGN(4096) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    .stack (NOLOAD) : ALIGN(4096) {
        . = . + $KERNEL_STACK_SIZE;
        __stack_top = .;
    }

    _kernel_end = .;
}
KERNELLD

# =============================================================================
# Step 1: Build kernel
# =============================================================================

echo ""
echo "[1/4] Building kernel..."
cargo clean --manifest-path $KERNEL_SOURCE_DIR/Cargo.toml 2>&1 | tail -1
RUSTFLAGS="-C link-arg=-T$PROJECT_ROOT/$KERNEL_SOURCE_DIR/kernel.ld" \
cargo build \
    --manifest-path $KERNEL_SOURCE_DIR/Cargo.toml \
    --release \
    --target aarch64-unknown-none \
    -Z build-std=core,alloc 2>&1 | grep -E "(Compiling|Finished|error)" || true

KERNEL_ELF="$KERNEL_SOURCE_DIR/target/aarch64-unknown-none/release/kaal-kernel"
[ -f "$KERNEL_ELF" ] || { echo "ERROR: Kernel not built"; exit 1; }
echo "✓ Kernel: $(ls -lh $KERNEL_ELF | awk '{print $5}')"

# =============================================================================
# Step 2: Build root task
# =============================================================================

echo ""
echo "[2/4] Building root task..."
if [ -d "$ROOTTASK_SOURCE_DIR" ]; then
    cargo build \
        --manifest-path $ROOTTASK_SOURCE_DIR/Cargo.toml \
        --release \
        --target aarch64-unknown-none \
        -Z build-std=core 2>&1 | grep -E "(Compiling|Finished|error)" || true
    ROOTTASK_ELF="$ROOTTASK_SOURCE_DIR/target/aarch64-unknown-none/release/dummy-roottask"
else
    touch "$BUILD_DIR/roottask.bin"
    ROOTTASK_ELF="$BUILD_DIR/roottask.bin"
fi
[ -f "$ROOTTASK_ELF" ] || { echo "ERROR: Root task not built"; exit 1; }
echo "✓ Root task: $(ls -lh $ROOTTASK_ELF | awk '{print $5}')"

# =============================================================================
# Step 3: Create embeddable objects
# =============================================================================

echo ""
echo "[3/4] Creating embeddable objects..."
llvm-objcopy -I binary -O elf64-littleaarch64 \
    --rename-section .data=.kernel_elf \
    "$KERNEL_ELF" "$BUILD_DIR/kernel.o"
llvm-objcopy -I binary -O elf64-littleaarch64 \
    --rename-section .data=.roottask_data \
    "$ROOTTASK_ELF" "$BUILD_DIR/roottask.o"
echo "✓ kernel.o: $(ls -lh $BUILD_DIR/kernel.o | awk '{print $5}')"
echo "✓ roottask.o: $(ls -lh $BUILD_DIR/roottask.o | awk '{print $5}')"

# =============================================================================
# Step 4: Build elfloader
# =============================================================================

echo ""
echo "[4/4] Building elfloader..."

# Delete old linker script and generate new one
rm -f $ELFLOADER_SOURCE_DIR/linker.ld
cat > $ELFLOADER_SOURCE_DIR/linker.ld << ELFLD
OUTPUT_FORMAT("elf64-littleaarch64")
OUTPUT_ARCH(aarch64)
ENTRY(_start)

INPUT($BUILD_DIR/kernel.o)
INPUT($BUILD_DIR/roottask.o)

SECTIONS
{
    . = $ELFLOADER_ADDR;
    .text : { *(.text._start) *(.text .text.*) }
    .rodata : { *(.rodata .rodata.*) }

    .kernel_elf ALIGN(4096) : {
        __kernel_image_start = .;
        KEEP(*(.kernel_elf))
        __kernel_image_end = .;
    }

    .roottask_data ALIGN(4096) : {
        __user_image_start = .;
        KEEP(*(.roottask_data))
        __user_image_end = .;
    }

    .data : { *(.data .data.*) }
    .bss : {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    __elfloader_end = .;
    . = $STACK_TOP;
    __stack_top = .;
}
ELFLD

# Force elfloader rebuild by cleaning
cd $ELFLOADER_SOURCE_DIR
cargo clean 2>&1 | tail -1

# Get elfloader target json
ELFLOADER_TARGET_JSON=$(get_config "platform.$PLATFORM" "elfloader_target_json")

# Build elfloader (only pass feature if it exists)
CARGO_FEATURES=""
if cargo read-manifest --manifest-path "$PROJECT_ROOT/$ELFLOADER_SOURCE_DIR/Cargo.toml" 2>/dev/null | grep -q "platform-$PLATFORM"; then
    CARGO_FEATURES="--features platform-$PLATFORM"
fi

RUSTFLAGS="-C link-arg=-T$PROJECT_ROOT/$ELFLOADER_SOURCE_DIR/linker.ld" \
cargo build \
    --target "$PROJECT_ROOT/$ELFLOADER_SOURCE_DIR/$ELFLOADER_TARGET_JSON" \
    --bin elfloader \
    --release \
    $CARGO_FEATURES \
    -Z build-std=core,alloc 2>&1 | grep -E "(Compiling|Finished|error)" | head -20

BOOTIMAGE="target/aarch64-unknown-none-elf/release/elfloader"
[ -f "$BOOTIMAGE" ] || { echo "ERROR: Bootimage not built"; exit 1; }

# Get QEMU launch parameters
QEMU_MACHINE=$(get_config "platform.$PLATFORM" "qemu_machine")
QEMU_CPU=$(get_config "platform.$PLATFORM" "qemu_cpu")
QEMU_MEMORY=$(get_config "platform.$PLATFORM" "qemu_memory")

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  ✓ BUILD COMPLETE"
echo "══════════════════════════════════════════════════════════"
echo ""
echo "Platform:  $PLATFORM_NAME"
echo "Bootimage: $PROJECT_ROOT/$ELFLOADER_SOURCE_DIR/$BOOTIMAGE"
ls -lh "$BOOTIMAGE"
echo ""
if [ -n "$QEMU_MACHINE" ]; then
    echo "Run: qemu-system-aarch64 -machine $QEMU_MACHINE -cpu $QEMU_CPU -m $QEMU_MEMORY -nographic -kernel $PROJECT_ROOT/$ELFLOADER_SOURCE_DIR/$BOOTIMAGE"
else
    echo "Deploy bootimage to target platform: $PLATFORM_NAME"
fi
echo ""
