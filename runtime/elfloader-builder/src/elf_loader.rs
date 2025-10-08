//! ELF file parsing and region extraction

use anyhow::{Context, Result};
use goblin::elf::{Elf, ProgramHeader};
use std::fs;
use std::path::Path;

use crate::payload::Region;

const PAGE_SIZE: usize = 4096;

/// Align address up to page boundary
fn page_align_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Align address down to page boundary
fn page_align_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

/// Parse an ELF file and extract loadable regions
pub fn parse_elf_file(path: &Path, base_paddr: usize) -> Result<(Vec<Region>, Vec<u8>, usize)> {
    log::info!("Parsing ELF: {}", path.display());

    let data = fs::read(path).context("Failed to read ELF file")?;
    let elf = Elf::parse(&data).context("Failed to parse ELF")?;

    log::info!("  Entry point: {:#x}", elf.entry);
    log::info!("  Program headers: {}", elf.program_headers.len());

    let mut regions = Vec::new();
    let mut region_data = Vec::new();
    let mut current_offset = 0;

    // Process loadable segments
    for ph in &elf.program_headers {
        if ph.p_type == goblin::elf::program_header::PT_LOAD {
            process_load_segment(
                &ph,
                &data,
                base_paddr,
                &mut regions,
                &mut region_data,
                &mut current_offset,
            )?;
        }
    }

    log::info!("  Extracted {} regions, {} bytes total", regions.len(), region_data.len());

    Ok((regions, region_data, elf.entry as usize))
}

fn process_load_segment(
    ph: &ProgramHeader,
    elf_data: &[u8],
    base_paddr: usize,
    regions: &mut Vec<Region>,
    region_data: &mut Vec<u8>,
    current_offset: &mut usize,
) -> Result<()> {
    let segment_paddr = base_paddr + ph.p_paddr as usize;
    let segment_vaddr = ph.p_vaddr as usize;
    let segment_size = ph.p_memsz as usize;
    let file_size = ph.p_filesz as usize;

    log::info!("  Load segment:");
    log::info!("    Phys: {:#x} - {:#x}", segment_paddr, segment_paddr + segment_size);
    log::info!("    Virt: {:#x} - {:#x}", segment_vaddr, segment_vaddr + segment_size);
    log::info!("    File size: {:#x}, Mem size: {:#x}", file_size, segment_size);

    // Extract segment data from ELF
    let file_offset = ph.p_offset as usize;
    let segment_bytes = &elf_data[file_offset..file_offset + file_size];

    let region = Region {
        paddr: segment_paddr,
        vaddr: segment_vaddr,
        size: segment_size,
        data_offset: *current_offset,
        data_size: file_size,
    };

    // Append segment data
    region_data.extend_from_slice(segment_bytes);

    // If memsz > filesz, we have BSS that will be zeroed
    if segment_size > file_size {
        let bss_size = segment_size - file_size;
        log::info!("    BSS: {} bytes to be zeroed", bss_size);
        // Pad with zeros
        region_data.extend(std::iter::repeat(0).take(bss_size));
    }

    *current_offset += segment_size;
    regions.push(region);

    Ok(())
}

/// Calculate the physical address range needed for an ELF
pub fn calculate_paddr_range(path: &Path) -> Result<usize> {
    let data = fs::read(path)?;
    let elf = Elf::parse(&data)?;

    let mut max_end = 0;
    for ph in &elf.program_headers {
        if ph.p_type == goblin::elf::program_header::PT_LOAD {
            let end = ph.p_paddr + ph.p_memsz;
            if end > max_end {
                max_end = end;
            }
        }
    }

    Ok(page_align_up(max_end as usize))
}
