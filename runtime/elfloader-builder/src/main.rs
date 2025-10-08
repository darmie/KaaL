//! KaaL Elfloader Builder
//!
//! This tool packages the seL4 kernel and KaaL root task into a bootable image.
//!
//! Usage:
//!   kaal-elfloader-builder \
//!     --loader path/to/elfloader.elf \
//!     --kernel path/to/kernel.elf \
//!     --app path/to/root-task.elf \
//!     --out path/to/bootimage.elf

mod elf_loader;
mod payload;

use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;

use crate::elf_loader::{calculate_paddr_range, parse_elf_file};
use crate::payload::Payload;

#[derive(Parser, Debug)]
#[command(name = "kaal-elfloader-builder")]
#[command(about = "Package seL4 kernel and KaaL root task into bootable image")]
struct Args {
    /// Path to elfloader ELF binary
    #[arg(long)]
    loader: PathBuf,

    /// Path to seL4 kernel ELF
    #[arg(long)]
    kernel: PathBuf,

    /// Path to root task (user app) ELF
    #[arg(long)]
    app: PathBuf,

    /// Output bootable image path
    #[arg(long)]
    out: PathBuf,

    /// Kernel physical load address (default: 0x40000000)
    #[arg(long, default_value = "0x40000000")]
    kernel_paddr: String,

    /// User app physical load address offset from kernel end (default: 0x200000 = 2MB)
    #[arg(long, default_value = "0x200000")]
    app_offset: String,
}

fn parse_hex_or_dec(s: &str) -> Result<usize> {
    if let Some(hex) = s.strip_prefix("0x") {
        usize::from_str_radix(hex, 16).context("Invalid hex number")
    } else {
        s.parse::<usize>().context("Invalid decimal number")
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    log::info!("═══════════════════════════════════════════════════════════");
    log::info!("  KaaL Elfloader Builder v0.1.0");
    log::info!("═══════════════════════════════════════════════════════════");
    log::info!("");

    // Parse addresses
    let kernel_base_paddr = parse_hex_or_dec(&args.kernel_paddr)?;
    let app_offset = parse_hex_or_dec(&args.app_offset)?;

    log::info!("Configuration:");
    log::info!("  Loader: {}", args.loader.display());
    log::info!("  Kernel: {}", args.kernel.display());
    log::info!("  App:    {}", args.app.display());
    log::info!("  Output: {}", args.out.display());
    log::info!("  Kernel paddr: {:#x}", kernel_base_paddr);
    log::info!("");

    // Parse kernel ELF
    let (kernel_regions, kernel_data, kernel_entry) =
        parse_elf_file(&args.kernel, kernel_base_paddr)?;

    // Calculate user app base address (after kernel)
    let kernel_size = calculate_paddr_range(&args.kernel)?;
    let user_base_paddr = kernel_base_paddr + kernel_size + app_offset;
    log::info!("User app paddr: {:#x}", user_base_paddr);

    // Parse user app ELF
    let (user_regions, user_data, user_entry) = parse_elf_file(&args.app, user_base_paddr)?;

    // Create payload
    let total_data_size = kernel_data.len() + user_data.len();
    let payload = Payload {
        kernel_regions,
        kernel_entry,
        user_regions,
        user_entry,
        total_data_size,
    };

    log::info!("");
    log::info!("Payload summary:");
    log::info!("  Kernel entry: {:#x}", payload.kernel_entry);
    log::info!("  User entry:   {:#x}", payload.user_entry);
    let (k_start, k_end) = payload.kernel_paddr_range();
    log::info!("  Kernel range: {:#x} - {:#x}", k_start, k_end);
    let (u_start, u_end) = payload.user_paddr_range();
    log::info!("  User range:   {:#x} - {:#x}", u_start, u_end);
    log::info!("  Total data:   {} bytes", total_data_size);
    log::info!("");

    // Serialize payload with postcard
    log::info!("Serializing payload...");
    let serialized_payload = postcard::to_allocvec(&payload).context("Failed to serialize")?;
    log::info!("  Metadata: {} bytes", serialized_payload.len());

    // Combine: serialized metadata + kernel data + user data
    let mut complete_payload = Vec::new();
    complete_payload.extend_from_slice(&serialized_payload);
    complete_payload.extend_from_slice(&kernel_data);
    complete_payload.extend_from_slice(&user_data);

    log::info!("  Complete payload: {} bytes", complete_payload.len());

    // For now, just save the payload to a file
    // TODO: Properly patch the elfloader binary
    let payload_path = args.out.with_extension("payload");
    fs::write(&payload_path, &complete_payload)
        .context("Failed to write payload")?;

    log::info!("");
    log::info!("✓ Payload written to: {}", payload_path.display());
    log::info!("");
    log::info!("Next steps:");
    log::info!("  1. Link payload into elfloader");
    log::info!("  2. Create final bootable ELF");
    log::info!("");
    log::info!("═══════════════════════════════════════════════════════════");

    Ok(())
}
