# Configuration Module
# Handles loading and parsing of build-config.toml and components.toml

# Load build configuration from TOML
export def "config load" [] {
    open build-config.toml
}

# Load component manifest from project root
export def "config load-components" [] {
    open components.toml
}

# Get platform configuration
export def "config get-platform" [config: record, platform: string] {
    $config | get platform | get $platform
}

# Calculate hex address from base + offset
export def "config calc-addr" [base: string, offset: string] {
    let base_int = ($base | into int)
    let offset_int = ($offset | into int)
    let result = $base_int + $offset_int
    $"0x(printf '%x' $result)"
}

# Validate platform exists
export def "config validate-platform" [config: record, platform: string] {
    let platforms = ($config | get platform | columns)
    if not ($platform in $platforms) {
        error make {
            msg: $"Unknown platform: ($platform)"
            label: {
                text: $"Available platforms: ($platforms | str join ', ')"
                span: (metadata $platform).span
            }
        }
    }
}

# Get all available platforms
export def "config list-platforms" [config: record] {
    $config | get platform | columns
}
