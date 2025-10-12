fn main() {
    // Tell Cargo to rerun if the generated memory config changes
    println!("cargo:rerun-if-changed=src/generated/memory_config.rs");

    // Tell Cargo to rerun if the linker script changes
    println!("cargo:rerun-if-changed=kernel.ld");
}
