use std::env;
use std::path::PathBuf;

fn main() {
    // Pass linker script location
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let linker_script = PathBuf::from("linker.ld");

    println!("cargo:rustc-link-arg=-T{}", linker_script.display());
    println!("cargo:rerun-if-changed=linker.ld");

    // Set default paths for kernel and user images (will be overridden at build time)
    if env::var("KERNEL_IMAGE_PATH").is_err() {
        println!("cargo:rustc-env=KERNEL_IMAGE_PATH=/dev/null");
    }
    if env::var("USER_IMAGE_PATH").is_err() {
        println!("cargo:rustc-env=USER_IMAGE_PATH=/dev/null");
    }
}
