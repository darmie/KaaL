#!/usr/bin/env nu

# Standalone QEMU runner for KaaL
# Run with: nu run-qemu.nu

def main [
    --timeout: int = 5  # Timeout in seconds (default 5)
    --debug             # Enable debug output
] {
    print "═══════════════════════════════════════════════════════════"
    print "  KaaL QEMU Runner"
    print "═══════════════════════════════════════════════════════════"
    print ""

    # Check if kernel exists
    let kernel_path = "runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader"

    if not ($kernel_path | path exists) {
        print $"Error: Kernel not found at ($kernel_path)"
        print "Please build first with: nu build.nu"
        exit 1
    }

    print $"Running kernel: ($kernel_path)"
    print $"Timeout: ($timeout) seconds"
    print ""
    print "Starting QEMU..."
    print "─────────────────────────────────────────────────────────────"

    # Build QEMU command
    let qemu_cmd = [
        "qemu-system-aarch64"
        "-machine" "virt"
        "-cpu" "cortex-a53"
        "-m" "128M"
        "-nographic"
        "-kernel" $kernel_path
    ]

    # Run with timeout
    let output = if $timeout > 0 {
        # Use timeout command
        do { timeout $"($timeout)s" ...$qemu_cmd } | complete
    } else {
        # Run without timeout (Ctrl+C to stop)
        do { ^$qemu_cmd.0 ...$qemu_cmd.1.. } | complete
    }

    # Display output
    if $output.exit_code == 124 {
        print ""
        print "─────────────────────────────────────────────────────────────"
        print $"[Timed out after ($timeout) seconds]"
    } else if $output.exit_code != 0 {
        print ""
        print "─────────────────────────────────────────────────────────────"
        print $"[QEMU exited with code: ($output.exit_code)]"
    }

    # Show the output
    print $output.stdout

    # Show specific sections if requested
    if $debug {
        print ""
        print "Debug Analysis:"
        print "─────────────────────────────────────────────────────────────"

        # Check for Chapter 9 Phase 5
        let phase5_output = $output.stdout | lines | where $it =~ "Phase 5|IPC|producer|consumer"
        if ($phase5_output | length) > 0 {
            print "Found IPC-related output:"
            $phase5_output | each { |line| print $"  ($line)" }
        } else {
            print "No IPC/Phase 5 output found"
        }

        # Check for errors
        let error_output = $output.stdout | lines | where $it =~ "ERROR|PANIC|fault"
        if ($error_output | length) > 0 {
            print ""
            print "Found errors:"
            $error_output | each { |line| print $"  ($line)" }
        }
    }
}

# Helper to extract specific phase output
def "main phase" [
    phase: int  # Phase number to extract
    --timeout: int = 5
] {
    print $"Extracting Phase ($phase) output..."

    let output = main --timeout $timeout | complete
    let phase_lines = $output.stdout | lines | where $it =~ $"Phase ($phase)"

    if ($phase_lines | length) > 0 {
        print $"Found Phase ($phase):"
        $phase_lines | each { |line| print $line }
    } else {
        print $"No Phase ($phase) output found"
    }
}

# Helper to find IPC communication
def "main ipc" [
    --timeout: int = 10
] {
    print "Looking for IPC communication..."

    # Run QEMU directly with timeout
    let qemu_cmd = [
        "timeout" $"($timeout)s"
        "qemu-system-aarch64"
        "-machine" "virt"
        "-cpu" "cortex-a53"
        "-m" "128M"
        "-nographic"
        "-kernel" "runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader"
    ]

    let output = do { ^$qemu_cmd.0 ...$qemu_cmd.1.. } | complete

    # Find IPC lines
    let ipc_lines = $output.stdout
        | lines
        | where $it =~ "producer|consumer|IPC|Channel|Phase 5"

    if ($ipc_lines | length) > 0 {
        print "IPC Communication Output:"
        print "─────────────────────────────────────────────────────────────"
        $ipc_lines | each { |line| print $line }
    } else {
        print "No IPC communication output found"
        print "Full output:"
        print $output.stdout | lines | last 50
    }
}