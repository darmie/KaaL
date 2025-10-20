#!/usr/bin/env nu
# KaaL Build System - Modular Nushell Edition
#
# A modern, type-safe, cross-platform build system for KaaL microkernel.
#
# Features:
# - Modular architecture (build-system/ directory)
# - Automatic component discovery from components.toml
# - Config-driven platform support
# - Structured error handling
# - Progress reporting

use build-system/config/mod.nu *
use build-system/utils/mod.nu *
use build-system/builders/mod.nu *
use build-system/builders/codegen.nu *
use build-system/builders/components.nu *

# =============================================================================
# Main Build Function
# =============================================================================

# Build KaaL for the specified platform
#
# Examples:
#   ./build.nu                          # Build for default platform (qemu-virt)
#   ./build.nu --platform rpi4          # Build for Raspberry Pi 4
#   ./build.nu -p qemu-virt --verbose   # Verbose output
#   ./build.nu --clean                  # Clean before building
def main [
    --platform (-p): string = "qemu-virt"  # Platform to build for
    --arch (-a): string = "aarch64"       # Target architecture (aarch64, x86_64, riscv64)
    --verbose (-v)                         # Verbose output
    --clean (-c)                          # Clean before building
    --list-platforms (-l)                 # List available platforms
    --run (-r)                            # Run in QEMU after building
] {
    # Load configuration
    let config = (config load)

    # Handle --list-platforms
    if $list_platforms {
        print "Available platforms:"
        for platform in (config list-platforms $config) {
            let platform_cfg = (config get-platform $config $platform)
            print $"  • ($platform) - ($platform_cfg.name)"
        }
        return
    }

    # Validate platform exists
    config validate-platform $config $platform

    # Get platform configuration
    let platform_cfg = (config get-platform $config $platform)

    # Print header
    print header "KaaL Build System (Modular Nushell)"
    print $"Platform: ($platform_cfg.name)"
    print $"Target:   ($platform_cfg.arch)"
    print ""

    # Clean if requested
    if $clean {
        print "🧹 Cleaning build artifacts..."
        # Clean component targets
        let comp_tomls = (try { ls components/*/Cargo.toml } catch { [] })
        for comp_toml in $comp_tomls {
            let comp_dir = ($comp_toml.name | path dirname)
            let target_dir = $"($comp_dir)/target"
            if ($target_dir | path exists) {
                rm -rf $target_dir
                print $"  Cleaned ($comp_dir | path basename)"
            }
        }
        # Clean kernel target
        if ("kernel/target" | path exists) {
            rm -rf kernel/target
            print "  Cleaned kernel"
        }
        # Clean runtime targets
        let runtime_tomls = (try { ls runtime/*/Cargo.toml } catch { [] })
        for runtime_toml in $runtime_tomls {
            let runtime_dir = ($runtime_toml.name | path dirname)
            let target_dir = $"($runtime_dir)/target"
            if ($target_dir | path exists) {
                rm -rf $target_dir
                print $"  Cleaned ($runtime_dir | path basename)"
            }
        }
        # Clean generated files
        if ("runtime/root-task/src/generated" | path exists) {
            rm -rf runtime/root-task/src/generated
            print "  Cleaned root-task generated files"
        }
        if ("components/system-init/src/generated" | path exists) {
            rm -rf components/system-init/src/generated
            print "  Cleaned system-init generated files"
        }
        if ("kernel/src/generated" | path exists) {
            rm -rf kernel/src/generated
            print "  Cleaned kernel generated files"
        }
        print ""
    }

    # Discover and validate components
    let components = (components validate)

    # Generate component linker scripts and configs
    print ""
    codegen component-linkers --platform $platform

    # Build components (excluding system_init)
    build components $platform_cfg

    # Generate component registry
    print ""
    codegen component-registry

    # Build system_init (after registry is generated)
    build system-init

    # Calculate addresses
    let elfloader_addr = (config calc-addr $platform_cfg.ram_base $platform_cfg.elfloader_offset)
    let kernel_addr = (config calc-addr $platform_cfg.ram_base $platform_cfg.kernel_offset)
    let stack_top = (config calc-addr $platform_cfg.ram_base $platform_cfg.stack_top_offset)

    if $verbose {
        print $"Memory Layout:"
        print $"  RAM Base:     ($platform_cfg.ram_base)"
        print $"  Elfloader:    ($elfloader_addr)"
        print $"  Kernel:       ($kernel_addr)"
        print $"  Stack Top:    ($stack_top)"
        print ""
    }

    # Create build directory
    let build_dir = $config.build.output_dir
    ensure dir $build_dir

    # Generate platform-specific code
    codegen memory-config $platform_cfg

    # Build steps
    let kernel_elf = (build kernel $config $kernel_addr)
    let roottask_elf = (build roottask $platform $platform_cfg $config.build.root_task_stack_size)
    build embeddable $kernel_elf $roottask_elf $build_dir
    let bootimage = (build elfloader $platform_cfg $platform $elfloader_addr $stack_top $build_dir)

    # Print success
    print ""
    print header "✓ BUILD COMPLETE"
    print ""
    print $"Platform:  ($platform_cfg.name)"
    print $"Bootimage: ($bootimage)"
    print ""
    print success "Final Image" $bootimage
    print ""

    # Print QEMU command
    if ($platform_cfg.qemu_machine? != null) {
        print $"Run in QEMU:"
        print $"  qemu-system-aarch64 -machine ($platform_cfg.qemu_machine) -cpu ($platform_cfg.qemu_cpu) -m ($platform_cfg.qemu_memory) -nographic -kernel ($bootimage)"
        print ""
    }

    # Print component summary
    let autostart_components = (components autostart)
    print $"📦 Autostart Components: ($autostart_components | length)"
    for component in $autostart_components {
        let comp_type = ($component | get type)
        let comp_priority = ($component | get priority)
        print $"  • ($component.name) \(($comp_type), priority: ($comp_priority)\)"
    }

    # Run in QEMU if requested
    if $run {
        if ($platform_cfg.qemu_machine? == null) {
            print $"(ansi red)Error: Platform ($platform) does not support QEMU execution(ansi reset)"
            return
        }

        print ""
        print header "🚀 LAUNCHING QEMU"
        print ""
        print $"Machine: ($platform_cfg.qemu_machine)"
        print $"CPU:     ($platform_cfg.qemu_cpu)"
        print $"Memory:  ($platform_cfg.qemu_memory)"
        print $"Image:   ($bootimage)"
        print ""
        print $"(ansi yellow)Press Ctrl+A then X to exit QEMU(ansi reset)"
        print ""

        # Give user time to read the message
        sleep 2sec

        # Launch QEMU
        ^qemu-system-aarch64 -machine $platform_cfg.qemu_machine -cpu $platform_cfg.qemu_cpu -m $platform_cfg.qemu_memory -nographic -kernel $bootimage
    }
}
