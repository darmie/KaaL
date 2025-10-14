use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Read build-config.toml from workspace root
    let workspace_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let config_path = workspace_root.join("build-config.toml");
    println!("cargo:rerun-if-changed={}", config_path.display());

    let config_contents =
        fs::read_to_string(&config_path).expect("Failed to read build-config.toml");

    let config: toml::Value =
        toml::from_str(&config_contents).expect("Failed to parse build-config.toml");

    // Get platform (default to qemu-virt)
    let platform = env::var("KAAL_PLATFORM").unwrap_or_else(|_| "qemu-virt".to_string());

    // Read root task configuration
    let root_task_stack_size = config["build"]["root_task_stack_size"]
        .as_str()
        .expect("Missing root_task_stack_size in build-config.toml");

    let page_size = config["platform"][&platform]["page_size"]
        .as_str()
        .unwrap_or("0x1000");

    // Calculate root task load address from platform-specific offset
    let ram_base_str = config["platform"][&platform]["ram_base"]
        .as_str()
        .expect("Missing ram_base in platform config");
    let roottask_offset_str = config["platform"][&platform]["roottask_offset"]
        .as_str()
        .expect("Missing roottask_offset in platform config");

    // Parse hex strings to calculate actual address
    let ram_base = u64::from_str_radix(ram_base_str.trim_start_matches("0x"), 16)
        .expect("Invalid ram_base format");
    let roottask_offset = u64::from_str_radix(roottask_offset_str.trim_start_matches("0x"), 16)
        .expect("Invalid roottask_offset format");
    let root_task_load_base = format!("0x{:x}", ram_base + roottask_offset);

    // Generate linker script
    let linker_script = format!(
        r#"/*
 * KaaL Root Task Linker Script (ARM64)
 * AUTO-GENERATED - DO NOT EDIT
 * Generated from build-config.toml
 */

ENTRY(_start)

SECTIONS
{{
    /*
     * Placeholder load address for linker
     * Actual address determined by elfloader at runtime and passed via boot info
     */
    . = {root_task_load_base};

    /* Text section (code) */
    .text : ALIGN({page_size}) {{
        *(.text._start)    /* Entry point first */
        *(.text .text.*)   /* All other code */
    }}

    /* Read-only data */
    .rodata : ALIGN({page_size}) {{
        *(.rodata .rodata.*)
    }}

    /* Data section */
    .data : ALIGN({page_size}) {{
        *(.data .data.*)
    }}

    /* BSS (zero-initialized data) */
    .bss : ALIGN({page_size}) {{
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }}

    /* Stack */
    . = ALIGN({page_size});
    __stack_start = .;
    . += {root_task_stack_size};
    __stack_end = .;

    /* End marker */
    . = ALIGN({page_size});
    __root_task_end = .;

    /* Discard unwanted sections */
    /DISCARD/ : {{
        *(.comment)
        *(.gnu*)
        *(.note*)
        *(.eh_frame*)
    }}
}}
"#,
        root_task_load_base = root_task_load_base,
        root_task_stack_size = root_task_stack_size,
        page_size = page_size,
    );

    // Write generated linker script
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let linker_script_path = out_dir.join("root-task.ld");
    fs::write(&linker_script_path, linker_script).expect("Failed to write linker script");

    // Also write to source directory for reference (optional)
    let src_linker_path =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("root-task.ld.generated");
    fs::write(&src_linker_path, format!("/* This is a reference copy. Actual linker script is in target/*/build/*/out/ */\n\n{}",
        fs::read_to_string(&linker_script_path).unwrap()))
        .ok(); // Ignore errors

    // Tell cargo to use the generated linker script
    println!("cargo:rustc-link-arg=-T{}", linker_script_path.display());
    println!("cargo:rustc-link-search={}", out_dir.display());
}
