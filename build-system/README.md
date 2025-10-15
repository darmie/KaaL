# Build System Modules

This directory contains the modular Nushell build system for KaaL.

## Structure

```
build-system/
├── config/
│   └── mod.nu          # Configuration management
├── utils/
│   └── mod.nu          # Utility functions
└── builders/
    ├── mod.nu          # Main build orchestration
    ├── codegen.nu      # Code generation
    └── components.nu   # Component discovery
```

## Modules

### config/mod.nu

Configuration loading and management:

- `config load` - Load build-config.toml
- `config load-components` - Load components.toml
- `config get-platform` - Get platform configuration
- `config calc-addr` - Calculate memory addresses
- `config validate-platform` - Validate platform exists
- `config list-platforms` - List available platforms

### utils/mod.nu

Common utilities:

- `print header` - Print section header
- `print step` - Print build step
- `print success` - Print success with file size
- `check exists` - Verify file exists
- `ensure dir` - Create directory if needed
- `cargo build-safe` - Safe cargo build wrapper

### builders/mod.nu

High-level build functions:

- `build kernel` - Build kernel ELF
- `build roottask` - Build root-task ELF
- `build embeddable` - Create embeddable objects
- `build elfloader` - Build final bootimage

### builders/codegen.nu

Code generation:

- `codegen memory-config` - Generate memory_config.rs
- `codegen kernel-linker` - Generate kernel linker script
- `codegen elfloader-linker` - Generate elfloader linker script

### builders/components.nu

Component management:

- `components discover` - Discover components from manifest
- `components validate` - Validate component manifest
- `components autostart` - Get autostart components
- `components get` - Get component by name

## Usage

Modules are imported in build.nu:

```nu
use build-system/config/mod.nu *
use build-system/utils/mod.nu *
use build-system/builders/mod.nu *
use build-system/builders/codegen.nu *
use build-system/builders/components.nu *
```

## Adding New Functionality

### Add a New Platform

1. Edit `build-config.toml`:
   ```toml
   [platform.my-board]
   name = "My Board"
   # ... configuration
   ```

2. Build system automatically discovers it:
   ```bash
   ./build.nu --list-platforms
   ./build.nu --platform my-board
   ```

### Add a New Build Step

1. Create function in appropriate module:
   ```nu
   # In builders/mod.nu
   export def "build my-component" [] {
       print step 5 5 "Building my component"
       # ... build logic
   }
   ```

2. Call from main build.nu:
   ```nu
   build my-component
   ```

### Add a New Code Generator

1. Create function in `builders/codegen.nu`:
   ```nu
   export def "codegen my-config" [config: record] {
       let output = $"// Generated config\n..."
       $output | save --force path/to/output.rs
   }
   ```

2. Call during build:
   ```nu
   codegen my-config $platform_cfg
   ```

## Benefits of Modular Architecture

### ✅ Maintainability
- Each module has a single responsibility
- Easy to find and modify specific functionality
- Clear separation of concerns

### ✅ Reusability
- Functions can be reused across modules
- Common utilities in one place
- DRY (Don't Repeat Yourself)

### ✅ Testability
- Modules can be tested independently
- Easy to mock dependencies
- Clear interfaces

### ✅ Extensibility
- Add new modules without modifying existing code
- Platform-specific logic can be isolated
- Component builders can be added incrementally

## Examples

### Use Config Module Standalone

```nu
# Load configuration
use build-system/config/mod.nu *
let config = (config load)

# List platforms
config list-platforms $config

# Get platform details
let platform = (config get-platform $config "qemu-virt")
print $platform.name
```

### Use Component Discovery Standalone

```nu
# Discover components
use build-system/builders/components.nu *
let components = (components discover)

# Get autostart components
let autostart = (components autostart)
print $"Found ($autostart | length) autostart components"
```

### Use Utils Standalone

```nu
# Pretty printing
use build-system/utils/mod.nu *

print header "My Build Step"
print step 1 3 "Compiling"
print success "Output" "path/to/file"
```

## Design Principles

1. **Single Responsibility**: Each module does one thing well
2. **Explicit Exports**: Only export what's needed (`export def`)
3. **Type Annotations**: Use types where helpful (`: record`, `: string`)
4. **Error Handling**: Use `error make` for structured errors
5. **Documentation**: Document each exported function

## Future Enhancements

- [ ] Add parallel builds (build kernel + roottask simultaneously)
- [ ] Add build caching (skip unchanged components)
- [ ] Add incremental builds (only rebuild changed files)
- [ ] Add build profiling (measure build times)
- [ ] Add test runner module
- [ ] Add deployment module (flash to hardware)

## References

- [Nushell Modules](https://www.nushell.sh/book/modules.html)
- [build.nu](../build.nu) - Main build script
- [build-config.toml](../build-config.toml) - Platform configurations
- [components.toml](../components.toml) - Component manifest
