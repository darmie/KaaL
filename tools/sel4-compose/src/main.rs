//! sel4-compose - CLI tool for KaaL project management
//!
//! # Purpose
//! Provides command-line interface for:
//! - Creating new KaaL projects
//! - Managing components
//! - Building and running systems
//! - Debugging and profiling
//!
//! # Commands
//! - `new` - Create a new project
//! - `build` - Build the system
//! - `run` - Run in QEMU
//! - `debug` - Start debugging session
//! - `component` - Manage components

use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "sel4-compose")]
#[command(author = "KaaL Team")]
#[command(version)]
#[command(about = "KaaL project management tool", long_about = None)]
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

        /// Project template
        #[arg(short, long, default_value = "minimal")]
        template: String,
    },

    /// Build the project
    Build {
        /// Target architecture
        #[arg(short, long, default_value = "aarch64")]
        target: String,

        /// Release build
        #[arg(short, long)]
        release: bool,
    },

    /// Run in QEMU
    Run {
        /// Target architecture
        #[arg(short, long, default_value = "aarch64")]
        target: String,
    },

    /// Show project info
    Info,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, template } => {
            println!("{} Creating project '{}'...", "📦".green(), name);
            println!("  Template: {}", template);
            println!("\n{} Project creation not yet implemented", "⚠️".yellow());
            println!("  See: docs/GETTING_STARTED.md for manual setup");
        }

        Commands::Build { target, release } => {
            println!("{} Building for {}...", "🔨".green(), target);
            let mode = if release { "release" } else { "debug" };
            println!("  Mode: {}", mode);
            println!("\n{} Build integration not yet implemented", "⚠️".yellow());
            println!("  Use: cargo build --target {}-unknown-none {}",
                     target,
                     if release { "--release" } else { "" });
        }

        Commands::Run { target } => {
            println!("{} Running on {} QEMU...", "🚀".green(), target);
            println!("\n{} QEMU integration not yet implemented", "⚠️".yellow());
            println!("  See: docs/GETTING_STARTED.md for QEMU commands");
        }

        Commands::Info => {
            println!("{}", "KaaL - seL4 Kernel-as-a-Library".bold().green());
            println!("Version: {}", env!("CARGO_PKG_VERSION"));
            println!("Status: Phase 1 - Foundation (In Progress)");
            println!("\nFor more info, see: README.md");
        }
    }

    Ok(())
}
