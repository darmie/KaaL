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

    // Create Dockerfile with full build environment
    let dockerfile = format!(
        r#"# Multi-stage build for KaaL project
# Stage 1: Build seL4 kernel and set up environment
FROM trustworthysystems/sel4:latest AS builder

# Install Rust and build dependencies
RUN apt-get update && apt-get install -y \
    curl build-essential qemu-system-aarch64 \
    device-tree-compiler wget git python3 python3-pip \
    ninja-build tree && \
    rm -rf /var/lib/apt/lists/*

# Install Rust nightly
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${{PATH}}"
RUN rustup component add rust-src

# Build seL4 kernel for ARM64/QEMU
WORKDIR /opt
RUN git clone --depth 1 https://github.com/seL4/seL4.git && \
    cd seL4 && mkdir build && cd build && \
    cmake -G Ninja \
        -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
        -DKernelPlatform=qemu-arm-virt \
        -DKernelSel4Arch=aarch64 \
        -DKernelDebugBuild=TRUE \
        -DKernelPrinting=TRUE \
        .. && \
    ninja && \
    mkdir -p gen_headers/kernel gen_headers/sel4 gen_headers/interfaces gen_headers/api && \
    cp gen_config/kernel/gen_config.json gen_headers/kernel/ && \
    cp gen_config/kernel/gen_config.h gen_headers/kernel/ && \
    cp libsel4/gen_config/sel4/gen_config.json gen_headers/sel4/ && \
    cp libsel4/gen_config/sel4/gen_config.h gen_headers/sel4/ && \
    cp /opt/seL4/libsel4/include/interfaces/object-api.xml gen_headers/interfaces/ && \
    cp /opt/seL4/libsel4/sel4_arch_include/aarch64/interfaces/object-api-sel4-arch.xml gen_headers/interfaces/ && \
    cp /opt/seL4/libsel4/arch_include/arm/interfaces/object-api-arch.xml gen_headers/interfaces/ && \
    cp /opt/seL4/libsel4/include/api/syscall.xml gen_headers/api/

# Set environment for rust-sel4 bindings
ENV SEL4_DIR=/opt/seL4
ENV SEL4_BUILD_DIR=/opt/seL4/build
ENV SEL4_PLATFORM=qemu-arm-virt
ENV SEL4_PREFIX=/opt/seL4/build
ENV SEL4_INCLUDE_DIRS=/opt/seL4/build/gen_headers:/opt/seL4/build/libsel4/include:/opt/seL4/build/libsel4/autoconf:/opt/seL4/build/libsel4/sel4_arch_include/aarch64:/opt/seL4/build/libsel4/arch_include/arm:/opt/seL4/libsel4/include:/opt/seL4/libsel4/sel4_arch_include/aarch64:/opt/seL4/libsel4/arch_include/arm:/opt/seL4/libsel4/mode_include/64:/opt/seL4/libsel4/sel4_plat_include/qemu-arm-virt

# Stage 2: Build KaaL root task
COPY . /kaal
WORKDIR /kaal/examples/{name}
RUN cargo build --release \
    --target aarch64-unknown-none \
    --no-default-features \
    -Z unstable-options \
    -Zbuild-std=core,alloc,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem

# Stage 3: Rebuild seL4 kernel with our root task
WORKDIR /opt/seL4/build-final
RUN cmake -G Ninja \
        -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
        -DKernelPlatform=qemu-arm-virt \
        -DKernelSel4Arch=aarch64 \
        -DKernelDebugBuild=TRUE \
        -DKernelPrinting=TRUE \
        -DElfloaderImage=/kaal/examples/{name}/target/aarch64-unknown-none/release/{name} \
        .. && \
    ninja

# Extract the final bootable image
CMD ["cat", "/opt/seL4/build-final/images/sel4-image"]
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
