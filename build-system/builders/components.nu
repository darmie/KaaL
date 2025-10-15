# Component Discovery Module
# Discovers and validates components from components.toml

use ../config/mod.nu *

# Discover and validate components
export def "components discover" [] {
    let components_data = (config load-components)
    let component_list = ($components_data | get component)

    print $"🔍 Discovered ($component_list | length) component(s) from components.toml:"

    for component in $component_list {
        let autostart_mark = if $component.autostart { "✓" } else { " " }
        let caps_count = ($component.capabilities | length)
        print $"  [($autostart_mark)] ($component.name)"
        print $"      Type: ($component.type), Priority: ($component.priority), Caps: ($caps_count)"
    }

    print ""
    $component_list
}

# Validate component manifest
export def "components validate" [] {
    let components = (components discover)

    # Check for duplicate names
    let names = ($components | get name)
    let unique_names = ($names | uniq)

    if ($names | length) != ($unique_names | length) {
        error make {
            msg: "Duplicate component names found in components.toml"
        }
    }

    # Check system_init exists and is first
    let first_component = ($components | first | get name)
    if $first_component != "system_init" {
        print "⚠️  Warning: system_init should be the first component"
    }

    # Validate priority ranges (0-255)
    for component in $components {
        let priority = $component.priority
        if $priority < 0 or $priority > 255 {
            error make {
                msg: $"Invalid priority ($priority) for component ($component.name). Must be 0-255."
            }
        }
    }

    print "✓ Component manifest validation passed"
    $components
}

# Get autostart components
export def "components autostart" [] {
    let components = (config load-components | get component)
    $components | where autostart == true
}

# Get component by name
export def "components get" [name: string] {
    let components = (config load-components | get component)
    $components | where name == $name | first
}
