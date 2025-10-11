//! Device Tree Blob (DTB) parsing
//!
//! Parses the Flattened Device Tree (FDT) to extract hardware information.
//! This is a minimal implementation for Chapter 1 - just enough to get:
//! - Model name
//! - Memory regions

use core::str;

/// Device tree information extracted from DTB
pub struct DtbInfo {
    pub model: &'static str,
    pub memory_start: usize,
    pub memory_end: usize,
}

/// DTB parsing errors
#[derive(Debug)]
pub enum DtbError {
    InvalidMagic,
    InvalidStructure,
    MemoryNotFound,
    ModelNotFound,
}

/// FDT Header structure
#[repr(C)]
struct FdtHeader {
    magic: u32,          // Must be 0xd00dfeed
    totalsize: u32,      // Total size of DTB
    off_dt_struct: u32,  // Offset to structure block
    off_dt_strings: u32, // Offset to strings block
    off_mem_rsvmap: u32, // Offset to memory reserve map
    version: u32,        // DTB version
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

const FDT_MAGIC: u32 = 0xd00dfeed;
const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32 = 0x00000002;
const FDT_PROP: u32 = 0x00000003;
const FDT_END: u32 = 0x00000009;

/// Parse device tree at given physical address
pub fn parse(dtb_addr: usize) -> Result<DtbInfo, DtbError> {
    let header = unsafe { &*(dtb_addr as *const FdtHeader) };

    // Verify magic number
    if u32::from_be(header.magic) != FDT_MAGIC {
        return Err(DtbError::InvalidMagic);
    }

    // Get offsets
    let struct_offset = u32::from_be(header.off_dt_struct) as usize;
    let strings_offset = u32::from_be(header.off_dt_strings) as usize;

    let struct_base = dtb_addr + struct_offset;
    let strings_base = dtb_addr + strings_offset;

    // Parse structure to find model and memory
    let mut model: Option<&'static str> = None;
    let mut memory_start: Option<usize> = None;
    let mut memory_end: Option<usize> = None;

    let mut offset = 0;
    loop {
        let token = read_u32(struct_base + offset);
        offset += 4;

        match token {
            FDT_BEGIN_NODE => {
                // Read node name
                let node_name = read_string(struct_base + offset);
                offset = align_up(offset + node_name.len() + 1, 4);
            }
            FDT_END_NODE => {
                // End of node
            }
            FDT_PROP => {
                // Read property
                let len = read_u32(struct_base + offset) as usize;
                offset += 4;
                let nameoff = read_u32(struct_base + offset) as usize;
                offset += 4;

                let prop_name = read_string_from_table(strings_base, nameoff);
                let prop_data = struct_base + offset;

                // Check if this is the model property
                if prop_name == "model" && model.is_none() {
                    model = Some(read_string(prop_data));
                }

                // Check if this is a memory reg property
                if prop_name == "reg" && memory_start.is_none() {
                    // reg property contains: <address size> pairs (64-bit each on ARM64)
                    let start = read_u64(prop_data);
                    let size = read_u64(prop_data + 8);
                    memory_start = Some(start as usize);
                    memory_end = Some((start + size) as usize);
                }

                offset = align_up(offset + len, 4);
            }
            FDT_END => {
                break;
            }
            _ => {
                return Err(DtbError::InvalidStructure);
            }
        }
    }

    Ok(DtbInfo {
        model: model.ok_or(DtbError::ModelNotFound)?,
        memory_start: memory_start.ok_or(DtbError::MemoryNotFound)?,
        memory_end: memory_end.ok_or(DtbError::MemoryNotFound)?,
    })
}

/// Read big-endian u32
#[inline]
fn read_u32(addr: usize) -> u32 {
    let val = unsafe { core::ptr::read_volatile(addr as *const u32) };
    u32::from_be(val)
}

/// Read big-endian u64
#[inline]
fn read_u64(addr: usize) -> u64 {
    let val = unsafe { core::ptr::read_volatile(addr as *const u64) };
    u64::from_be(val)
}

/// Read null-terminated string
fn read_string(addr: usize) -> &'static str {
    let mut len = 0;
    while unsafe { *((addr + len) as *const u8) } != 0 {
        len += 1;
        if len > 256 {
            return "<invalid>";
        }
    }

    let bytes = unsafe { core::slice::from_raw_parts(addr as *const u8, len) };
    str::from_utf8(bytes).unwrap_or("<invalid>")
}

/// Read string from strings table at given offset
fn read_string_from_table(strings_base: usize, offset: usize) -> &'static str {
    read_string(strings_base + offset)
}

/// Align value up to alignment
#[inline]
fn align_up(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}
