# Builders Module
# High-level build functions for each component

use ../utils/mod.nu *
use ../config/mod.nu *
use codegen.nu *

# Build kernel
export def "build kernel" [config: record, kernel_addr: string] {
    print step 1 4 "Building kernel"

    # Generate linker script
    codegen kernel-linker $kernel_addr $config.build.kernel_stack_size

    # Clean and build
    cargo clean --manifest-path kernel/Cargo.toml | ignore

    let rustflags = $"-C link-arg=-T($env.PWD)/kernel/kernel.ld"
    with-env { RUSTFLAGS: $rustflags } {
        cargo build-safe --manifest-path kernel/Cargo.toml --target aarch64-unknown-none --release --build-std [core alloc]
    }

    let kernel_elf = "kernel/target/aarch64-unknown-none/release/kaal-kernel"
    check exists $kernel_elf "Kernel ELF"

    print success "Kernel" $kernel_elf
    $kernel_elf
}

# Build root-task
export def "build roottask" [platform: string, platform_cfg: record, root_task_stack_size: string] {
    print step 2 4 "Building root-task"

    # Generate root-task memory configuration
    codegen roottask-memory-config $platform_cfg

    # Generate root-task linker script
    codegen roottask-linker $platform_cfg $root_task_stack_size

    # Build with linker script
    let rustflags = $"-C link-arg=-T($env.PWD)/runtime/root-task/root-task.ld"
    with-env { KAAL_PLATFORM: $platform, RUSTFLAGS: $rustflags } {
        cargo build-safe --manifest-path runtime/root-task/Cargo.toml --target aarch64-unknown-none --release --build-std [core alloc]
    }

    let roottask_elf = "runtime/root-task/target/aarch64-unknown-none/release/root-task"
    check exists $roottask_elf "Root-task ELF"

    print success "Root-task" $roottask_elf
    $roottask_elf
}

# Create embeddable objects
export def "build embeddable" [kernel_elf: string, roottask_elf: string, build_dir: string] {
    print step 3 4 "Creating embeddable objects"

    ensure dir $build_dir

    # Convert kernel to object
    llvm-objcopy -I binary -O elf64-littleaarch64 --rename-section .data=.kernel_elf $kernel_elf $"($build_dir)/kernel.o"

    # Convert root-task to object
    llvm-objcopy -I binary -O elf64-littleaarch64 --rename-section .data=.roottask_data $roottask_elf $"($build_dir)/roottask.o"

    print success "kernel.o" $"($build_dir)/kernel.o"
    print success "roottask.o" $"($build_dir)/roottask.o"
}

# Validate and auto-fix memory layout to ensure no overlaps
# Returns: updated platform_cfg if changes were made, or original if no changes needed
export def "validate memory-layout" [platform_cfg: record, kernel_size: int, roottask_size: int] {
    let ram_base = ($platform_cfg.ram_base | into int)
    let roottask_offset = ($platform_cfg.roottask_offset | into int)
    let elfloader_offset = ($platform_cfg.elfloader_offset | into int)
    let kernel_offset = ($platform_cfg.kernel_offset | into int)

    let roottask_start = $ram_base + $roottask_offset
    let roottask_end = $roottask_start + $roottask_size
    let elfloader_start = $ram_base + $elfloader_offset
    let kernel_start = $ram_base + $kernel_offset

    # Check root-task doesn't overlap with elfloader
    # Root-task is typically loaded BEFORE elfloader in memory layout
    if $roottask_offset < $elfloader_offset {
        # Root-task is before elfloader, check it doesn't extend into elfloader
        if $roottask_end > $elfloader_start {
            print ""
            print $"(ansi yellow_bold)⚠ Memory layout overlap detected!(ansi reset)"
            print $"(ansi yellow)Root-task has grown too large and overlaps with elfloader:(ansi reset)"
            let roottask_kb = ($roottask_size // 1024)
            print $"  Root-task:  (printf '0x%x' $roottask_start) - (printf '0x%x' $roottask_end) \(($roottask_kb) KB\)"
            print $"  Elfloader:  (printf '0x%x' $elfloader_start) - ..."
            print $"  Overlap:    (printf '0x%x' ($roottask_end - $elfloader_start)) bytes"
            print ""

            # Calculate new offset: place root-task after kernel, with 1MB alignment
            # Memory layout: DTB -> elfloader (embedded images) -> kernel -> root-task
            # Root-task must be after kernel_offset + some margin for kernel size
            let suggested_offset = (($kernel_offset + 0x200000) // 0x100000) * 0x100000  # kernel + 2MB margin, rounded up

            print $"(ansi cyan_bold)🔧 Auto-fixing: Adjusting roottask_offset(ansi reset)"
            print $"  Old: (printf '0x%x' $roottask_offset)"
            print $"  New: (printf '0x%x' $suggested_offset)"
            print ""

            # Update build-config.toml (both value and comment)
            let old_offset_hex = (printf '0x%x' $roottask_offset)
            let new_offset_hex = (printf '0x%x' $suggested_offset)
            let new_addr_hex = (printf '0x%x' ($ram_base + $suggested_offset))

            let old_line = $"roottask_offset = \"($old_offset_hex)\""
            let new_line = $"roottask_offset = \"($new_offset_hex)\"  # Root task at ram_base + ($new_offset_hex) = ($new_addr_hex)"

            # Read, update, and write config file as raw text
            let config_content = (open --raw build-config.toml)
            # Replace the entire line including any existing comment
            let updated_content = ($config_content | str replace -r $"roottask_offset = \"($old_offset_hex)\"[^\n]*" $new_line)
            $updated_content | save -f build-config.toml

            print $"(ansi green)✓ Updated build-config.toml(ansi reset)"
            print $"(ansi cyan)Rebuilding root-task with new memory layout...(ansi reset)"
            print ""

            # Return true to signal that we need to rebuild
            return true
        }
    }

    # Check elfloader doesn't overlap with kernel
    if $elfloader_start >= $kernel_start {
        print ""
        print $"(ansi red_bold)ERROR: Elfloader offset >= Kernel offset!(ansi reset)"
        print $"  Elfloader: (printf '0x%x' $elfloader_start)"
        print $"  Kernel:    (printf '0x%x' $kernel_start)"
        print ""
        error make { msg: "Memory layout configuration error - elfloader cannot be after kernel" }
    }

    print $"(ansi green)✓ Memory layout validated - no overlaps detected(ansi reset)"
    return false
}

# Build elfloader (final bootimage)
export def "build elfloader" [
    platform_cfg: record,
    platform: string,
    elfloader_addr: string,
    stack_top: string,
    build_dir: string
] {
    print step 4 4 "Building elfloader"

    # Generate linker script
    codegen elfloader-linker $elfloader_addr $stack_top $build_dir

    # Clean elfloader
    cd runtime/elfloader
    cargo clean | ignore
    cd ../..

    # Build elfloader with platform-specific feature
    let rustflags = $"-C link-arg=-T($env.PWD)/runtime/elfloader/linker.ld"
    with-env { RUSTFLAGS: $rustflags } {
        let target_json = $"runtime/elfloader/($platform_cfg.elfloader_target_json)"
        let features = $"platform-($platform)"
        cargo build-safe --manifest-path runtime/elfloader/Cargo.toml --target $target_json --features $features --release --build-std [core alloc]
    }

    let bootimage = "runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader"
    check exists $bootimage "Elfloader bootimage"

    $bootimage
}

# Build components (excluding system_init which is built last)
export def "build components" [platform_cfg: record] {
    print ""
    print "Building components (excluding system_init)..."

    # Get list of components from components.toml
    let components_data = (open components.toml)
    let components = ($components_data | get component)

    # Build ALL components EXCEPT system_init (not just autostart ones)
    # system_init is built last because it needs the registry
    for comp in $components {
        if $comp.name == "system_init" {
            continue  # Skip system_init, build it last
        }

        let comp_dir = $"components/($comp.binary)"
        let cargo_toml = $"($comp_dir)/Cargo.toml"
        if ($cargo_toml | path exists) {
            print $"  → Building ($comp.name)..."
            # Change to component directory so cargo finds .cargo/config.toml
            cd $comp_dir
            cargo build-safe --target aarch64-unknown-none --release
            cd ../..
        }
    }

    print "✓ Components built (except system_init)"
}

# Build system_init (must be called AFTER registry generation)
export def "build system-init" [] {
    print ""
    print "Building system_init (with generated registry)..."

    let comp_dir = "components/system-init"
    let cargo_toml = $"($comp_dir)/Cargo.toml"

    if ($cargo_toml | path exists) {
        cd $comp_dir
        cargo build-safe --target aarch64-unknown-none --release
        cd ../..
        print "✓ system_init built"
    } else {
        error make {
            msg: "system_init Cargo.toml not found"
        }
    }
}
