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

    # Generate root-task linker script
    codegen roottask-linker $platform_cfg $root_task_stack_size

    # Build with linker script
    let rustflags = $"-C link-arg=-T($env.PWD)/runtime/root-task/root-task.ld"
    with-env { KAAL_PLATFORM: $platform, RUSTFLAGS: $rustflags } {
        cargo build-safe --manifest-path runtime/root-task/Cargo.toml --target aarch64-unknown-none --release --build-std [core]
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

# Build components
export def "build components" [platform_cfg: record] {
    print ""
    print "Building components..."

    # Get list of components from components.toml
    let components_data = (open components.toml)
    let components = ($components_data | get component)

    # Build ALL components (not just autostart ones)
    # This ensures components can be spawned on-demand later
    for comp in $components {
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

    print "✓ Components built"
}
