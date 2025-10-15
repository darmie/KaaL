//! Build script for root-task
//!
//! This script:
//! 1. Locates components.toml at the project root
//! 2. Embeds it as a compile-time constant for parsing
//! 3. Sets up rebuild triggers when components.toml changes

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the project root (two levels up from runtime/root-task)
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let project_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find project root");

    // Path to components.toml at project root
    let components_toml = project_root.join("components.toml");

    // Verify it exists
    if !components_toml.exists() {
        panic!(
            "components.toml not found at project root: {}",
            components_toml.display()
        );
    }

    // Tell cargo to rerun if components.toml changes
    println!("cargo:rerun-if-changed={}", components_toml.display());

    // Read and validate it's valid UTF-8
    let contents = fs::read_to_string(&components_toml)
        .expect("Failed to read components.toml");

    // Basic validation: check it contains at least one [[component]]
    if !contents.contains("[[component]]") {
        panic!("components.toml appears to be empty or invalid (no [[component]] sections found)");
    }

    println!("cargo:rustc-env=COMPONENTS_TOML_PATH={}", components_toml.display());
    println!("cargo:rustc-env=COMPONENTS_COUNT={}", contents.matches("[[component]]").count());

    // Output location for developer convenience
    eprintln!("🔍 Component manifest: {}", components_toml.display());
    eprintln!("📦 Found {} component(s)", contents.matches("[[component]]").count());
}
