//! kaal - CLI tool for KaaL project management
//!
//! Commands:
//! - `kaal new <name>` - Create a new project
//! - `kaal build` - Build the system (uses Docker on macOS)
//! - `kaal run` - Run in QEMU
//! - `kaal info` - Show project info

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Parser)]
#[command(name = "kaal")]
#[command(author = "KaaL Team")]
#[command(version)]
#[command(about = "KaaL - seL4 microkernel framework", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new KaaL project
    New {
        /// Project name
        name: String,
    },

    /// Build the project
    Build {
        /// Release build
        #[arg(short, long)]
        release: bool,
    },

    /// Run in QEMU
    Run {
        /// Debug mode (wait for GDB)
        #[arg(short, long)]
        debug: bool,
    },

    /// Show project info
    Info,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            create_project(&name)?;
        }

        Commands::Build { release } => {
            build_project(release)?;
        }

        Commands::Run { debug } => {
            run_qemu(debug)?;
        }

        Commands::Info => {
            show_info();
        }
    }

    Ok(())
}

fn create_project(name: &str) -> anyhow::Result<()> {
    println!("{} Creating KaaL project '{}'...", "📦".green(), name.bold());

    let project_path = PathBuf::from(name);
    if project_path.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    // Create project structure
    fs::create_dir_all(&project_path)?;
    fs::create_dir_all(project_path.join("src"))?;
    fs::create_dir_all(project_path.join(".kaal"))?;
    fs::create_dir_all(project_path.join("wrapper"))?;

    // Create Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

# This is a standalone project, not part of a workspace
[workspace]

[dependencies]
# TODO: Update these paths to point to your KaaL installation
# For now, using relative paths (works if generated in KaaL examples/ directory)
kaal-root-task = {{ path = "../../runtime/root-task" }}
cap-broker = {{ path = "../../runtime/cap_broker", default-features = false, features = ["runtime"] }}
sel4-platform = {{ path = "../../runtime/sel4-platform", default-features = false, features = ["runtime"] }}

# When you copy this project elsewhere, use git dependencies:
# kaal-root-task = {{ git = "https://github.com/YOUR_ORG/kaal" }}
# cap-broker = {{ git = "https://github.com/YOUR_ORG/kaal", default-features = false, features = ["runtime"] }}
# sel4-platform = {{ git = "https://github.com/YOUR_ORG/kaal", default-features = false, features = ["runtime"] }}

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
"#,
        name = name
    );
    fs::write(project_path.join("Cargo.toml"), cargo_toml)?;

    // Create main.rs from system template (KaaL convention)
    let main_rs = include_str!("../templates/system.rs");
    fs::write(project_path.join("src/main.rs"), main_rs)?;

    // Create Dockerfile using sel4test infrastructure for bootable images
    let dockerfile = format!(
        r#"# KaaL Bootable Image Build
# Uses sel4test infrastructure to create bootable seL4 images with KaaL root task
FROM trustworthysystems/sel4:latest

# Install Rust and build dependencies
RUN apt-get update && apt-get install -y \
    curl build-essential qemu-system-aarch64 \
    device-tree-compiler git python3 python3-pip \
    ninja-build repo xmllint && \
    rm -rf /var/lib/apt/lists/*

# Install Rust nightly
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${{PATH}}"
RUN rustup component add rust-src

# ===================================================================
# Stage 1: Set up sel4test build infrastructure
# ===================================================================
WORKDIR /build
RUN repo init -u https://github.com/seL4/sel4test-manifest.git -b master && \
    repo sync

# ===================================================================
# Stage 2: Build KaaL Rust library
# ===================================================================
COPY . /kaal-source
WORKDIR /kaal-source/examples/{name}

# Build KaaL as a static library (lib{name}.a)
RUN cargo build --release \
    --target aarch64-unknown-none \
    --no-default-features \
    -Z unstable-options \
    -Zbuild-std=core,alloc,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem && \
    ls -lh target/aarch64-unknown-none/release/

# ===================================================================
# Stage 3: Replace sel4test-driver with KaaL wrapper
# ===================================================================

# Copy wrapper files to sel4test-driver location
WORKDIR /build/projects/sel4test/apps/sel4test-driver/src
RUN rm -f *.c *.h
COPY /kaal-source/examples/{name}/wrapper/main.c .

# Update sel4test-driver CMakeLists.txt to use our wrapper
WORKDIR /build/projects/sel4test/apps/sel4test-driver
RUN rm -f CMakeLists.txt
COPY /kaal-source/examples/{name}/wrapper/CMakeLists.txt .

# Create symlink to KaaL Rust library so CMake can find it
RUN mkdir -p /build/projects/sel4test/apps/sel4test-driver/target/aarch64-unknown-none/release && \
    ln -sf /kaal-source/examples/{name}/target/aarch64-unknown-none/release/lib{name}.a \
           /build/projects/sel4test/apps/sel4test-driver/target/aarch64-unknown-none/release/lib{name}.a

# ===================================================================
# Stage 4: Build bootable image with KaaL
# ===================================================================
WORKDIR /build
RUN mkdir -p build && cd build && \
    ../init-build.sh -DPLATFORM=qemu-arm-virt -DAARCH64=1 && \
    ninja

# Extract bootable image
RUN cp /build/build/images/sel4test-driver-image-arm-qemu-arm-virt /boot-image.elf && \
    echo "=== Bootable KaaL image created ===" && \
    ls -lh /boot-image.elf && \
    file /boot-image.elf

# Default command: output the bootable image
CMD ["cat", "/boot-image.elf"]
"#,
        name = name
    );
    fs::write(project_path.join("Dockerfile"), dockerfile)?;

    // Create .kaal/config.toml
    let config_toml = r#"[build]
platform = "qemu-arm-virt"
arch = "aarch64"

[kernel]
debug_build = true
printing = true

[memory]
heap_size = "8MB"
vspace_base = "0x40000000"
vspace_size = "512MB"
"#;
    fs::write(project_path.join(".kaal/config.toml"), config_toml)?;

    // Create README.md
    let readme = format!(
        r#"# {}

KaaL-based seL4 system

## Build

```bash
kaal build
```

## Run

```bash
kaal run
```

## Customize

Edit `src/main.rs` to implement your system logic.
"#,
        name
    );
    fs::write(project_path.join("README.md"), readme)?;

    // Create wrapper/main.c - C entry point that calls into Rust
    let wrapper_main_c = r#"/*
 * KaaL seL4 Wrapper - C Entry Point
 *
 * This wrapper integrates KaaL with seL4's boot infrastructure.
 * It receives boot info from seL4 and passes it to the Rust runtime.
 */

#include <stdio.h>
#include <sel4/sel4.h>
#include <sel4platsupport/bootinfo.h>

/* Rust entry point */
extern void kaal_main(seL4_BootInfo *bootinfo);

int main(void) {
    seL4_BootInfo *info = platsupport_get_bootinfo();

    printf("\n");
    printf("===========================================\n");
    printf("  KaaL Root Task Starting\n");
    printf("===========================================\n");
    printf("  Boot Info:\n");
    printf("    IPC Buffer:      %p\n", (void*)info->ipcBuffer);
    printf("    Empty Slots:     [%lu-%lu)\n",
           (unsigned long)info->empty.start,
           (unsigned long)info->empty.end);
    printf("    User Image:      [%p-%p)\n",
           (void*)info->userImageFrames.start,
           (void*)info->userImageFrames.end);
    printf("===========================================\n\n");

    /* Transfer control to Rust KaaL runtime */
    kaal_main(info);

    /* Should never return */
    printf("ERROR: kaal_main returned!\n");
    return 1;
}
"#;
    fs::write(project_path.join("wrapper/main.c"), wrapper_main_c)?;

    // Create wrapper/kaal_entry.rs - Rust FFI entry point
    let wrapper_kaal_entry_rs = format!(
        r#"//! KaaL Entry Point - Rust FFI Bridge
//!
//! This module provides the C-callable entry point that receives
//! seL4 boot info and initializes the KaaL runtime.

#![no_std]

use kaal_root_task::{{RootTask, RootTaskConfig}};

/// C-callable entry point from wrapper
///
/// SAFETY: This function receives a pointer to seL4_BootInfo from the C wrapper.
/// The pointer must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn kaal_main(bootinfo: *const u8) -> ! {{
    // Initialize KaaL with default configuration
    let config = RootTaskConfig::default();
    let mut root = match RootTask::init(config) {{
        Ok(r) => r,
        Err(_) => halt("Failed to initialize RootTask"),
    }};

    // Run KaaL system with component spawning
    root.run_with(|broker| {{
        // TODO: Spawn your components here
        // See examples/my-kaal-system/src/main.rs for patterns

        // For now, just entering idle loop
        let _ = broker;
    }});
}}

/// Halt system on critical error
fn halt(msg: &str) -> ! {{
    // In a real system, you'd log this error
    let _ = msg;
    loop {{
        unsafe {{
            core::arch::asm!("wfi");
        }}
    }}
}}
"#
    );
    fs::write(project_path.join("wrapper/kaal_entry.rs"), wrapper_kaal_entry_rs)?;

    // Create wrapper/CMakeLists.txt - Build configuration
    let wrapper_cmake = format!(
        r#"cmake_minimum_required(VERSION 3.7.2)

# Include seL4 test infrastructure (provides build targets)
include(${{CMAKE_SOURCE_DIR}}/projects/sel4test/settings.cmake)

# Set project name
project(kaal-wrapper C ASM)

# Find required seL4 packages
find_package(seL4 REQUIRED)
find_package(elfloader-tool REQUIRED)
find_package(musllibc REQUIRED)
find_package(sel4muslcsys REQUIRED)
find_package(sel4platsupport REQUIRED)
find_package(sel4utils REQUIRED)
find_package(sel4debug REQUIRED)

# Build KaaL Rust library
set(KAAL_RUST_BINARY "${{CMAKE_CURRENT_SOURCE_DIR}}/../target/aarch64-unknown-none/release/lib{name}.a")

# Create wrapper executable (replaces sel4test-driver)
add_executable(sel4test-driver
    main.c
)

target_link_libraries(sel4test-driver
    PRIVATE
    ${{KAAL_RUST_BINARY}}
    sel4
    muslc
    sel4muslcsys
    sel4platsupport
    sel4utils
    sel4debug
)

# Declare as root server
include(rootserver)
DeclareRootserver(sel4test-driver)

# Generate final boot image
include(simulation)
GenerateSimulateScript()
"#,
        name = name
    );
    fs::write(project_path.join("wrapper/CMakeLists.txt"), wrapper_cmake)?;

    println!("{} Project created successfully!\n", "✅".green());
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  kaal build");
    println!("  kaal run");

    Ok(())
}

fn build_project(release: bool) -> anyhow::Result<()> {
    println!("{} Building KaaL project...", "🔨".green());

    // Check if we're on macOS (need Docker)
    let use_docker = cfg!(target_os = "macos") || std::env::var("KAAL_USE_DOCKER").is_ok();

    if use_docker {
        println!("  Using Docker build (macOS detected)\n");

        // Determine build context - should be KaaL repo root (../../ from examples/project)
        let build_context = Path::new("../..").canonicalize()
            .unwrap_or_else(|_| PathBuf::from("../.."));

        println!("  Build context: {}", build_context.display());

        // Build Docker image with KaaL repo root as context
        let status = Command::new("docker")
            .args(["build", "-f", "Dockerfile", "-t", "kaal-app"])
            .arg(&build_context)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            anyhow::bail!("Docker build failed");
        }

        // Extract bootable image
        println!("\n{} Extracting bootable seL4 image...", "📦".green());
        let output = Command::new("docker")
            .args(["run", "--rm", "kaal-app"])
            .output()?;

        fs::create_dir_all("build")?;
        fs::write("build/sel4-image.elf", output.stdout)?;

        println!("{} Build complete: build/sel4-image.elf", "✅".green());
    } else {
        // Native build
        let mode = if release { "--release" } else { "" };
        let status = Command::new("cargo")
            .args([
                "build",
                mode,
                "--target",
                "aarch64-unknown-none",
                "--no-default-features",
                "--features",
                "runtime",
            ])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            anyhow::bail!("Cargo build failed");
        }

        println!("{} Build complete", "✅".green());
    }

    Ok(())
}

fn run_qemu(debug: bool) -> anyhow::Result<()> {
    println!("{} Starting QEMU...", "🚀".green());

    if !Path::new("build/sel4-image.elf").exists() {
        anyhow::bail!("No image found. Run 'kaal build' first.");
    }

    let mut args = vec![
        "-machine",
        "virt",
        "-cpu",
        "cortex-a53",
        "-nographic",
        "-m",
        "512M",
        "-kernel",
        "build/sel4-image.elf",
    ];

    if debug {
        args.extend(&["-s", "-S"]);
        println!("  Debug mode: waiting for GDB on :1234");
    }

    println!("  Press Ctrl+A then X to exit\n");

    let status = Command::new("qemu-system-aarch64")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .status()?;

    if !status.success() {
        anyhow::bail!("QEMU failed");
    }

    Ok(())
}

fn show_info() {
    println!("{}", "KaaL - seL4 Kernel-as-a-Library".bold().green());
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("\n{}", "Framework for building verified seL4 systems".italic());
    println!("\nCommands:");
    println!("  kaal new <name>    Create a new project");
    println!("  kaal build         Build the system");
    println!("  kaal run           Run in QEMU");
    println!("\nDocumentation: https://github.com/darmie/kaal");
}
