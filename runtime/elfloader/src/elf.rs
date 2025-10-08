// ELF image parsing and loading

use goblin::elf::Elf;
use crate::uart_println;

pub struct ElfImage {
    pub entry: usize,
    pub load_addr: usize,
    pub size: usize,
}

pub fn parse_elf(data: &[u8]) -> Result<ElfImage, &'static str> {
    let elf = Elf::parse(data).map_err(|_| "Failed to parse ELF")?;

    uart_println!("ELF entry point: {:#x}", elf.entry);
    uart_println!("ELF program headers: {}", elf.program_headers.len());

    // Find the lowest and highest load addresses
    let mut min_addr = usize::MAX;
    let mut max_addr = 0;

    for ph in &elf.program_headers {
        if ph.p_type == goblin::elf::program_header::PT_LOAD {
            let start = ph.p_paddr as usize;
            let end = start + ph.p_memsz as usize;

            if start < min_addr {
                min_addr = start;
            }
            if end > max_addr {
                max_addr = end;
            }

            uart_println!("  Load segment: {:#x} - {:#x}", start, end);
        }
    }

    Ok(ElfImage {
        entry: elf.entry as usize,
        load_addr: min_addr,
        size: max_addr - min_addr,
    })
}

pub fn load_elf_segments(elf_data: &[u8], base_addr: usize) -> Result<(), &'static str> {
    let elf = Elf::parse(elf_data).map_err(|_| "Failed to parse ELF")?;

    for ph in &elf.program_headers {
        if ph.p_type == goblin::elf::program_header::PT_LOAD {
            let dest = (base_addr + ph.p_paddr as usize) as *mut u8;
            let src = &elf_data[ph.p_offset as usize..(ph.p_offset + ph.p_filesz) as usize];

            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dest, src.len());

                // Zero out BSS if memsz > filesz
                if ph.p_memsz > ph.p_filesz {
                    let bss_start = dest.add(ph.p_filesz as usize);
                    let bss_size = (ph.p_memsz - ph.p_filesz) as usize;
                    core::ptr::write_bytes(bss_start, 0, bss_size);
                }
            }
        }
    }

    Ok(())
}
