# Build System Fix - Component Registry Generation

**Date**: 2025-10-19
**Issue**: Component spawning fails with "Failed: no binary" errors
**Commit**: c9dac54

## Problem

After cleaning and rebuilding, components fail to spawn because the component registry contains stale `binary_data: None` entries instead of actual binary data.

### Symptoms

```
[root_task] Spawning autostart components...
  → system_init - Failed: no binary
  → serial_driver - Failed: no binary
  → timer_driver - Failed: no binary
  → process_manager - Failed: no binary
```

### Root Cause

The `--clean` flag was incomplete:
1. ✅ Cleaned `target/` directories (removes built binaries)
2. ❌ Did NOT clean `generated/` directories (registry files persist)

**Build sequence**:
1. Run `nu build.nu --clean`
2. Target directories deleted
3. Components rebuilt → binaries exist
4. Registry generation runs → checks for binaries
5. **BUT** old registry files still present with `binary_data: None`
6. Root-task built using stale registry
7. Components fail to spawn

### Why Binary Detection Failed

The codegen script checks for binaries:

```nu
let binary_path = $"components/($comp.binary)/target/aarch64-unknown-none/release/($comp.binary)"
let binary_exists = ($binary_path | path exists)
let binary_data = if $binary_exists {
    $"Some\(include_bytes!\(\"($rel_path)\"\)\)"
} else {
    "None"
}
```

If an old registry exists and hasn't been regenerated, it keeps the old `None` values.

## Solution

Extended `--clean` to remove generated files:

```nu
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
```

## How to Fix

If you're experiencing this issue:

```bash
# Clean and rebuild
nu build.nu --clean

# The registry will now be properly regenerated with binary data
```

## Verification

After running `--clean`, check the generated registry:

```bash
# Should show include_bytes!(...) instead of None
grep "binary_data:" runtime/root-task/src/generated/component_registry.rs
```

Expected output (after fix):
```rust
binary_data: Some(include_bytes!("../../../../components/system-init/target/aarch64-unknown-none/release/system-init")),
binary_data: None,  // For components without Rust binaries (serial_driver, etc.)
binary_data: None,  // For components without Rust binaries (timer_driver, etc.)
binary_data: None,  // For components without Rust binaries (process_manager, etc.)
binary_data: Some(include_bytes!("../../../../components/ipc-producer/target/aarch64-unknown-none/release/ipc-producer")),
binary_data: Some(include_bytes!("../../../../components/ipc-consumer/target/aarch64-unknown-none/release/ipc-consumer")),
binary_data: Some(include_bytes!("../../../../components/test-minimal/target/aarch64-unknown-none/release/test-minimal")),
```

Note: Some components legitimately have `None` because they're not Rust binaries (serial_driver, timer_driver, process_manager are placeholders).

## Files Modified

- [build.nu](../build.nu) - Added generated file cleanup to --clean flag

## Related

- Component registry generation: [build-system/builders/codegen.nu](../build-system/builders/codegen.nu) (line 566)
- Component loader: [runtime/root-task/src/component_loader.rs](../runtime/root-task/src/component_loader.rs)
- Generated registry: `runtime/root-task/src/generated/component_registry.rs` (auto-generated)
- Generated registry: `components/system-init/src/generated/registry.rs` (auto-generated)

## Future Improvements

Consider adding a check in the build script to warn if registry has all `None` values:

```nu
# After generating registry, validate it
let none_count = (open runtime/root-task/src/generated/component_registry.rs | str contains "binary_data: None" | length)
if $none_count == $total_components {
    print warning "Registry appears to have no binary data - run with --clean?"
}
```

---

**Last Updated**: 2025-10-19
**Status**: ✅ Fixed in commit c9dac54
