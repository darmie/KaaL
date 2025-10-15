# Utilities Module
# Common helper functions for the build system

# Print a section header
export def "print header" [title: string] {
    print $"══════════════════════════════════════════════════════════"
    print $"  ($title)"
    print $"══════════════════════════════════════════════════════════"
}

# Print a step header
export def "print step" [step: int, total: int, title: string] {
    print ""
    print $"[($step)/($total)] ($title)..."
}

# Print success message with file size
export def "print success" [label: string, path: string] {
    let size = (ls $path | get 0.size | into string)
    print $"✓ ($label): ($size)"
}

# Check if file exists, error if not
export def "check exists" [path: string, description: string] {
    if not ($path | path exists) {
        error make {
            msg: $"($description) not found"
            label: {
                text: $"Expected at: ($path)"
            }
        }
    }
}

# Create directory if it doesn't exist
export def "ensure dir" [path: string] {
    if not ($path | path exists) {
        mkdir $path
    }
}

# Run cargo build with proper error handling
export def "cargo build-safe" [
    --manifest-path (-m): string
    --target (-t): string
    --features (-f): string = ""
    --release (-r)
    --build-std (-z): list<string> = []
] {
    mut args = [build]

    if $manifest_path != null {
        $args = ($args | append [--manifest-path $manifest_path])
    }

    if $target != null {
        $args = ($args | append [--target $target])
    }

    if $release {
        $args = ($args | append "--release")
    }

    if ($features | str length) > 0 {
        $args = ($args | append [--features $features])
    }

    if ($build_std | length) > 0 {
        $args = ($args | append ["-Z" $"build-std=($build_std | str join ',')"])
    }

    let result = (cargo ...$args | complete)

    # Check if build succeeded
    if $result.exit_code != 0 {
        print $result.stderr
        error make {
            msg: "Cargo build failed"
            label: {
                text: $"Exit code: ($result.exit_code)"
            }
        }
    }

    $result
}
