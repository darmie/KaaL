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
cap-broker = {{ git = "https://github.com/darmie/kaal", default-features = false, features = ["runtime"] }}
sel4-platform = {{ git = "https://github.com/darmie/kaal", default-features = false, features = ["runtime"] }}

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

    // Create Dockerfile
    let dockerfile = format!(
        r#"FROM ghcr.io/darmie/kaal-builder:latest
COPY . /project
WORKDIR /project
RUN cargo build --release \
    --target aarch64-unknown-none \
    --no-default-features \
    --features runtime \
    -Z unstable-options \
    -Zbuild-std=core,alloc,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem
CMD ["cat", "/project/target/aarch64-unknown-none/release/{name}"]
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

        // Build Docker image
        let status = Command::new("docker")
            .args(["build", "-t", "kaal-app", "."])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            anyhow::bail!("Docker build failed");
        }

        // Extract binary
        println!("\n{} Extracting binary...", "📦".green());
        let output = Command::new("docker")
            .args(["run", "--rm", "kaal-app"])
            .output()?;

        fs::create_dir_all("build")?;
        fs::write("build/system.elf", output.stdout)?;

        println!("{} Build complete: build/system.elf", "✅".green());
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

    if !Path::new("build/system.elf").exists() {
        anyhow::bail!("No binary found. Run 'kaal build' first.");
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
        "build/system.elf",
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
