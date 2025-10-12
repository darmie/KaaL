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
    crate::kprintln!("DTB parse: reading header at {:#x}", dtb_addr);
    let header = unsafe { &*(dtb_addr as *const FdtHeader) };

    // Verify magic number
    let magic = u32::from_be(header.magic);
    crate::kprintln!("DTB magic: {:#x} (expected {:#x})", magic, FDT_MAGIC);
    if magic != FDT_MAGIC {
        return Err(DtbError::InvalidMagic);
    }
    crate::kprintln!("DTB magic OK");

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
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 200; // Much smaller limit for faster failure

    crate::kprintln!("Parsing DTB structure at {:#x}", struct_base);

    // Track if we're in a memory node
    let mut in_memory_node = false;

    loop {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            crate::kprintln!("ERROR: DTB parser exceeded {} iterations", MAX_ITERATIONS);
            crate::kprintln!("  Last offset: {:#x}", offset);
            crate::kprintln!("  Found model: {}", model.is_some());
            crate::kprintln!("  Found memory: {}", memory_start.is_some());
            // Return what we found so far if we have at least memory info
            if let (Some(start), Some(end)) = (memory_start, memory_end) {
                return Ok(DtbInfo {
                    model: model.unwrap_or("Unknown (DTB parse incomplete)"),
                    memory_start: start,
                    memory_end: end,
                });
            }
            return Err(DtbError::InvalidStructure);
        }

        let token = read_u32(struct_base + offset);
        offset += 4;

        if iterations <= 20 || iterations % 50 == 0 {
            crate::kprintln!("  [{}] Token {:#x} at offset {:#x}", iterations, token, offset - 4);
        }

        match token {
            FDT_BEGIN_NODE => {
                // Read node name
                let node_name = read_string(struct_base + offset);

                // Check if this is a memory node
                if node_name.starts_with("memory@") || node_name == "memory" {
                    in_memory_node = true;
                    if iterations <= 20 {
                        crate::kprintln!("    -> Entering memory node: '{}'", node_name);
                    }
                }

                offset = align_up(offset + node_name.len() + 1, 4);
            }
            FDT_END_NODE => {
                in_memory_node = false;
            }
            FDT_PROP => {
                // Read property
                let len = read_u32(struct_base + offset) as usize;
                offset += 4;
                let nameoff = read_u32(struct_base + offset) as usize;
                offset += 4;

                let prop_name = read_string_from_table(strings_base, nameoff);
                let prop_data = struct_base + offset;

                if iterations <= 20 {
                    crate::kprintln!("    -> Prop '{}' (len={})", prop_name, len);
                }

                // Check if this is the model property
                if prop_name == "model" && model.is_none() {
                    model = Some(read_string(prop_data));
                    crate::kprintln!("  Found model: '{}'", model.unwrap());
                }

                // Check if this is a memory reg property
                if prop_name == "reg" && in_memory_node && memory_start.is_none() {
                    if len >= 16 {
                        crate::kprintln!("    -> Reading memory reg property (len={})", len);
                        // reg property contains: <address size> pairs (64-bit each on ARM64)
                        let start = read_u64(prop_data);
                        crate::kprintln!("    -> Got start: {:#x}", start);
                        let size = read_u64(prop_data + 8);
                        crate::kprintln!("    -> Got size: {:#x}", size);
                        memory_start = Some(start as usize);
                        memory_end = Some((start + size) as usize);

                        crate::kprintln!("  Found memory: {:#x} - {:#x}", start, start + size);
                    } else {
                        crate::kprintln!("    -> Memory reg property too short: {}", len);
                    }
                }

                offset = align_up(offset + len, 4);
            }
            FDT_END => {
                crate::kprintln!("  DTB parsing complete ({} tokens)", iterations);
                break;
            }
            _ => {
                crate::kprintln!("ERROR: Unknown DTB token {:#x} at offset {:#x}", token, offset - 4);
                return Err(DtbError::InvalidStructure);
            }
        }

        // Early exit if we have both model and memory
        if model.is_some() && memory_start.is_some() {
            crate::kprintln!("  Found all required info, stopping parse");
            break;
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
    // Read as two u32s to avoid alignment issues
    let hi = read_u32(addr);
    let lo = read_u32(addr + 4);
    ((hi as u64) << 32) | (lo as u64)
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
