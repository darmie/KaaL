//! ELF parsing for userspace component loading
//!
//! Minimal ELF64 parser for loading component binaries.
//! Based on kernel/src/boot/elf.rs but adapted for userspace.

/// ELF parsing errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    /// Invalid ELF magic number
    InvalidMagic,
    /// Not a 64-bit ELF
    Not64Bit,
    /// Not little-endian
    NotLittleEndian,
    /// Invalid program header
    InvalidProgramHeader,
    /// Too many segments
    TooManySegments,
}

/// Maximum number of loadable segments
const MAX_SEGMENTS: usize = 8;

/// Parsed ELF information
#[derive(Debug, Clone, Copy)]
pub struct ElfInfo {
    /// Entry point address
    pub entry_point: usize,
    /// Number of loadable segments
    pub num_segments: usize,
    /// Segment data: (vaddr, file_size, mem_size, file_offset)
    pub segments: [(usize, usize, usize, usize); MAX_SEGMENTS],
    /// Minimum virtual address (for base calculation)
    pub min_vaddr: usize,
    /// Maximum virtual address (for size calculation)
    pub max_vaddr: usize,
}

impl ElfInfo {
    /// Calculate total memory size needed for process
    pub fn memory_size(&self) -> usize {
        if self.max_vaddr >= self.min_vaddr {
            self.max_vaddr - self.min_vaddr
        } else {
            0
        }
    }
}

/// Parse ELF64 binary
pub fn parse_elf(data: &[u8]) -> Result<ElfInfo, ElfError> {
    // Check minimum size
    if data.len() < 64 {
        return Err(ElfError::InvalidMagic);
    }

    // Check ELF magic: 0x7F 'E' 'L' 'F'
    if data[0] != 0x7F || data[1] != b'E' || data[2] != b'L' || data[3] != b'F' {
        return Err(ElfError::InvalidMagic);
    }

    // Check 64-bit (EI_CLASS = 2)
    if data[4] != 2 {
        return Err(ElfError::Not64Bit);
    }

    // Check little-endian (EI_DATA = 1)
    if data[5] != 1 {
        return Err(ElfError::NotLittleEndian);
    }

    // Parse entry point (offset 0x18, 8 bytes)
    let entry_point = read_u64(data, 0x18);

    // Parse program header offset (offset 0x20, 8 bytes)
    let phoff = read_u64(data, 0x20);

    // Parse program header entry size (offset 0x36, 2 bytes)
    let phentsize = read_u16(data, 0x36) as usize;

    // Parse number of program headers (offset 0x38, 2 bytes)
    let phnum = read_u16(data, 0x38) as usize;

    // Parse program headers
    let mut segments = [(0, 0, 0, 0); MAX_SEGMENTS];
    let mut num_segments = 0;
    let mut min_vaddr = usize::MAX;
    let mut max_vaddr = 0;

    for i in 0..phnum {
        let ph_offset = phoff + (i * phentsize);
        if ph_offset + phentsize > data.len() {
            return Err(ElfError::InvalidProgramHeader);
        }

        // Read p_type (offset 0x00, 4 bytes)
        let p_type = read_u32(data, ph_offset);

        // Only process PT_LOAD segments (type = 1)
        if p_type == 1 {
            if num_segments >= MAX_SEGMENTS {
                return Err(ElfError::TooManySegments);
            }

            // Read segment fields
            let vaddr = read_u64(data, ph_offset + 0x10);      // p_vaddr
            let filesz = read_u64(data, ph_offset + 0x20);     // p_filesz
            let memsz = read_u64(data, ph_offset + 0x28);      // p_memsz
            let offset = read_u64(data, ph_offset + 0x08);     // p_offset

            segments[num_segments] = (vaddr, filesz, memsz, offset);
            num_segments += 1;

            // Track address range
            if vaddr < min_vaddr {
                min_vaddr = vaddr;
            }
            let end_addr = vaddr + memsz;
            if end_addr > max_vaddr {
                max_vaddr = end_addr;
            }
        }
    }

    Ok(ElfInfo {
        entry_point,
        num_segments,
        segments,
        min_vaddr,
        max_vaddr,
    })
}

/// Read u64 from little-endian bytes
fn read_u64(data: &[u8], offset: usize) -> usize {
    let bytes = &data[offset..offset + 8];
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
    ]) as usize
}

/// Read u32 from little-endian bytes
fn read_u32(data: &[u8], offset: usize) -> u32 {
    let bytes = &data[offset..offset + 4];
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Read u16 from little-endian bytes
fn read_u16(data: &[u8], offset: usize) -> u16 {
    let bytes = &data[offset..offset + 2];
    u16::from_le_bytes([bytes[0], bytes[1]])
}
