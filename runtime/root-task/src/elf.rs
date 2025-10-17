//! Minimal ELF64 parser for userspace process spawning
//!
//! This is a simple, no_std ELF parser that extracts just enough information
//! to load a process into memory and create its execution environment.
//!
//! Unlike the bootloader's ELF parser, this:
//! - Runs in userspace (EL0)
//! - Uses syscalls for memory allocation
//! - Creates page tables and CSpace for new processes
//! - No external dependencies (no goblin crate)

#![allow(dead_code)]

/// ELF64 magic bytes
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// ELF64 class (64-bit)
const ELFCLASS64: u8 = 2;

/// Little endian
const ELFDATA2LSB: u8 = 1;

/// PT_LOAD segment type
const PT_LOAD: u32 = 1;

/// ELF64 header (simplified - only fields we need)
#[repr(C)]
struct Elf64Header {
    e_ident: [u8; 16],      // Magic number and other info
    e_type: u16,            // Object file type
    e_machine: u16,         // Architecture
    e_version: u32,         // Object file version
    e_entry: u64,           // Entry point virtual address
    e_phoff: u64,           // Program header table file offset
    e_shoff: u64,           // Section header table file offset
    e_flags: u32,           // Processor-specific flags
    e_ehsize: u16,          // ELF header size in bytes
    e_phentsize: u16,       // Program header table entry size
    e_phnum: u16,           // Program header table entry count
    e_shentsize: u16,       // Section header table entry size
    e_shnum: u16,           // Section header table entry count
    e_shstrndx: u16,        // Section header string table index
}

/// ELF64 program header (simplified)
#[repr(C)]
struct Elf64ProgramHeader {
    p_type: u32,            // Segment type
    p_flags: u32,           // Segment flags
    p_offset: u64,          // Segment file offset
    p_vaddr: u64,           // Segment virtual address
    p_paddr: u64,           // Segment physical address
    p_filesz: u64,          // Segment size in file
    p_memsz: u64,           // Segment size in memory
    p_align: u64,           // Segment alignment
}

/// Parsed ELF information needed for process creation
pub struct ElfInfo {
    /// Entry point (initial PC)
    pub entry_point: usize,
    /// Load segments (vaddr, filesz, memsz, file_offset)
    pub segments: [(usize, usize, usize, usize); 8],
    /// Number of load segments
    pub num_segments: usize,
    /// Minimum virtual address
    pub min_vaddr: usize,
    /// Maximum virtual address (for total size calculation)
    pub max_vaddr: usize,
}

impl ElfInfo {
    /// Get the total memory size needed for the process
    pub fn memory_size(&self) -> usize {
        self.max_vaddr - self.min_vaddr
    }
}

/// Parse an ELF64 binary
///
/// # Arguments
/// * `elf_data` - Raw ELF binary data
///
/// # Returns
/// * `Ok(ElfInfo)` - Parsed ELF information
/// * `Err(&str)` - Error message
pub fn parse_elf(elf_data: &[u8]) -> Result<ElfInfo, &'static str> {
    // Validate minimum size
    if elf_data.len() < core::mem::size_of::<Elf64Header>() {
        return Err("ELF too small");
    }

    // Cast to ELF header
    let header = unsafe { &*(elf_data.as_ptr() as *const Elf64Header) };

    // Validate magic
    if header.e_ident[0..4] != ELF_MAGIC {
        return Err("Invalid ELF magic");
    }

    // Validate class (64-bit)
    if header.e_ident[4] != ELFCLASS64 {
        return Err("Not 64-bit ELF");
    }

    // Validate endianness (little endian)
    if header.e_ident[5] != ELFDATA2LSB {
        return Err("Not little endian");
    }

    // Parse program headers
    let phoff = header.e_phoff as usize;
    let phnum = header.e_phnum as usize;
    let phentsize = header.e_phentsize as usize;

    if phnum > 8 {
        return Err("Too many program headers (max 8)");
    }

    let mut info = ElfInfo {
        entry_point: header.e_entry as usize,
        segments: [(0, 0, 0, 0); 8],
        num_segments: 0,
        min_vaddr: usize::MAX,
        max_vaddr: 0,
    };

    // Parse LOAD segments
    for i in 0..phnum {
        let ph_offset = phoff + (i * phentsize);
        if ph_offset + phentsize > elf_data.len() {
            return Err("Program header out of bounds");
        }

        let ph = unsafe {
            &*(elf_data.as_ptr().add(ph_offset) as *const Elf64ProgramHeader)
        };

        if ph.p_type == PT_LOAD {
            let vaddr = ph.p_vaddr as usize;
            let filesz = ph.p_filesz as usize;
            let memsz = ph.p_memsz as usize;
            let offset = ph.p_offset as usize;

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

    // Debug: Print extracted ELF info
    // (need to import the print functions)
    #[cfg(debug_assertions)]
    {
        // For now, just return the info - we can add debug later
    }

    Ok(info)
}
