//! ELF parser using xmas-elf crate
//!
//! This replaces our custom ELF parser with the well-tested xmas-elf crate.

use xmas_elf::ElfFile;
use xmas_elf::program::Type;

/// Information extracted from ELF for process creation
pub struct ElfInfo {
    /// Entry point address
    pub entry_point: usize,
    /// Segments: (vaddr, filesz, memsz, offset)
    pub segments: [(usize, usize, usize, usize); 8],
    /// Number of LOAD segments found
    pub num_segments: usize,
    /// Minimum virtual address
    pub min_vaddr: usize,
    /// Maximum virtual address (for total size calculation)
    pub max_vaddr: usize,
}

impl ElfInfo {
    /// Calculate total memory size needed
    pub fn memory_size(&self) -> usize {
        self.max_vaddr - self.min_vaddr
    }
}

/// Parse ELF binary using xmas-elf
pub fn parse_elf(elf_data: &[u8]) -> Result<ElfInfo, &'static str> {
    // Debug: Print that we're using xmas-elf
    unsafe {
        crate::sys_print("[xmas-elf] Starting ELF parse...\n");
    }

    // Parse ELF file
    let elf_file = match ElfFile::new(elf_data) {
        Ok(f) => f,
        Err(e) => {
            unsafe {
                crate::sys_print("[xmas-elf] Failed to parse ELF: ");
                crate::sys_print(e);
                crate::sys_print("\n");
            }
            return Err("Failed to parse ELF");
        }
    };

    // Get entry point
    unsafe {
        crate::sys_print("[xmas-elf] Getting entry point...\n");
    }
    let entry_point = elf_file.header.pt2.entry_point() as usize;
    unsafe {
        crate::sys_print("[xmas-elf] Entry point retrieved\n");
    }

    let mut info = ElfInfo {
        entry_point,
        segments: [(0, 0, 0, 0); 8],
        num_segments: 0,
        min_vaddr: usize::MAX,
        max_vaddr: 0,
    };

    // Process program headers
    unsafe {
        crate::sys_print("[xmas-elf] Processing program headers...\n");
    }
    for program_header in elf_file.program_iter() {
        unsafe {
            crate::sys_print("[xmas-elf] Checking program header...\n");
        }
        if program_header.get_type() == Ok(Type::Load) {
            if info.num_segments >= 8 {
                return Err("Too many LOAD segments");
            }

            let vaddr = program_header.virtual_addr() as usize;
            let filesz = program_header.file_size() as usize;
            let memsz = program_header.mem_size() as usize;
            let offset = program_header.offset() as usize;

            // Track address range
            if vaddr < info.min_vaddr {
                info.min_vaddr = vaddr;
            }
            let segment_end = vaddr + memsz;
            if segment_end > info.max_vaddr {
                info.max_vaddr = segment_end;
            }

            // Store segment info
            info.segments[info.num_segments] = (vaddr, filesz, memsz, offset);
            info.num_segments += 1;
        }
    }

    if info.num_segments == 0 {
        return Err("No LOAD segments found");
    }

    // Debug output
    unsafe {
        crate::sys_print("[xmas-elf] Parsed ELF:\n");
        crate::sys_print("  Entry: 0x");
        crate::print_hex(entry_point);
        crate::sys_print("\n");
        crate::sys_print("  Segments: ");
        crate::print_number(info.num_segments);
        crate::sys_print("\n");
        for i in 0..info.num_segments {
            let (vaddr, filesz, memsz, _) = info.segments[i];
            crate::sys_print("    [");
            crate::print_number(i);
            crate::sys_print("] vaddr=0x");
            crate::print_hex(vaddr);
            crate::sys_print(" filesz=0x");
            crate::print_hex(filesz);
            crate::sys_print(" memsz=0x");
            crate::print_hex(memsz);
            crate::sys_print("\n");
        }
        crate::sys_print("  Range: 0x");
        crate::print_hex(info.min_vaddr);
        crate::sys_print(" - 0x");
        crate::print_hex(info.max_vaddr);
        crate::sys_print("\n");
    }

    Ok(info)
}