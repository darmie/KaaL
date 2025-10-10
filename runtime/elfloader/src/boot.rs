//! Boot sequence management - loading kernel and root task

use crate::uart_println;
use crate::BootInfo;

/// Symbols provided by linker script for embedded kernel
extern "C" {
    static __kernel_image_start: u8;
    static __kernel_image_end: u8;
}

/// Symbols provided by linker script for embedded root task
extern "C" {
    static __user_image_start: u8;
    static __user_image_end: u8;
}

/// seL4 kernel's rootserver structure
///
/// This structure is defined in the seL4 kernel's BSS section at a fixed offset.
/// The elfloader must populate this structure with information about the root task
/// before jumping to the kernel. The kernel reads this structure during boot to
/// initialize the root task.
///
/// Based on seL4 v13.0.0 kernel source
#[repr(C)]
struct RootserverMem {
    /// Physical region containing root task image
    p_reg_start: usize,
    p_reg_end: usize,

    /// Virtual region for root task image
    v_reg_start: usize,
    v_reg_end: usize,

    /// Virtual entry point of root task
    v_entry: usize,

    /// Extra bootinfo pointer (DTB/ACPI)
    extra_bi: usize,

    /// Size of extra bootinfo
    extra_bi_size: usize,

    /// PV offset (physical-virtual offset)
    pv_offset: usize,

    /// Padding to 72 bytes
    _reserved: usize,
}

/// Offset of rootserver structure within the kernel binary
///
/// Found by: nm kernel.elf | grep rootserver
/// Address: 0xFFFFFF804002E8C8 (virtual)
/// Kernel loads at: 0x40000000 (physical)
/// Kernel virtual base: 0xFFFFFF8040000000
/// Offset = 0xFFFFFF804002E8C8 - 0xFFFFFF8040000000 = 0x2E8C8
const ROOTSERVER_OFFSET: usize = 0x2E8C8;

/// Load kernel and root task, return (kernel_entry, boot_info_for_root_task)
pub fn load_images() -> (usize, BootInfo) {
    uart_println!("Loading embedded images from ELF sections...");

    // Get kernel image from .kernel_elf section
    let (kernel_start, kernel_end) = unsafe {
        (
            &__kernel_image_start as *const u8 as usize,
            &__kernel_image_end as *const u8 as usize,
        )
    };

    let kernel_size = kernel_end - kernel_start;
    uart_println!("  Kernel: {:#x} - {:#x} ({} KB)", kernel_start, kernel_end, kernel_size / 1024);

    // Get root task from .roottask_data section
    let (user_start, user_end) = unsafe {
        (
            &__user_image_start as *const u8 as usize,
            &__user_image_end as *const u8 as usize,
        )
    };

    let user_size = user_end - user_start;
    uart_println!("  User:   {:#x} - {:#x} ({} KB)", user_start, user_end, user_size / 1024);

    // Load kernel at fixed physical address 0x40000000
    // This matches seL4's expected kernel load address for QEMU ARM virt
    let kernel_paddr = 0x40000000;

    uart_println!("Copying kernel to physical address {:#x}...", kernel_paddr);

    // Copy kernel to target physical address
    unsafe {
        let src = kernel_start as *const u8;
        let dst = kernel_paddr as *mut u8;
        core::ptr::copy_nonoverlapping(src, dst, kernel_size);
    }

    // Parse root task ELF to get entry point and load segments
    let user_entry = parse_elf_entry(user_start);
    uart_println!("Root task entry point: {:#x}", user_entry);

    uart_println!("Images loaded successfully!");

    // CRITICAL: Populate the kernel's rootserver structure
    // This tells the seL4 kernel where to find the root task
    let rootserver_addr = kernel_paddr + ROOTSERVER_OFFSET;
    uart_println!("Populating rootserver structure at {:#x}...", rootserver_addr);

    unsafe {
        let rootserver = rootserver_addr as *mut RootserverMem;
        core::ptr::write(rootserver, RootserverMem {
            // Physical region - where root task ELF is loaded in memory
            p_reg_start: user_entry,  // Root task loaded at its entry address
            p_reg_end: user_entry + (user_end - user_start),

            // Virtual region - same as physical for identity mapping
            v_reg_start: user_entry,
            v_reg_end: user_entry + (user_end - user_start),

            // Virtual entry point
            v_entry: user_entry,

            // Extra bootinfo (DTB) - will be set later
            extra_bi: 0,
            extra_bi_size: 0,

            // Physical-virtual offset (0 for identity mapping)
            pv_offset: 0,

            // Reserved/padding
            _reserved: 0,
        });
    }

    uart_println!("Rootserver structure populated:");
    uart_println!("  p_reg: {:#x} - {:#x}", user_entry, user_entry + (user_end - user_start));
    uart_println!("  v_entry: {:#x}", user_entry);

    // Return kernel entry and boot info
    // The kernel expects info about the root task in these parameters
    (
        kernel_paddr,  // Jump to kernel at this address
        BootInfo {
            user_img_start: user_start,    // Physical start of root task ELF
            user_img_end: user_end,          // Physical end of root task ELF
            pv_offset: 0,                    // Physical-virtual offset (identity mapped)
            user_entry,                      // Root task's entry point from its ELF header
            dtb_addr: 0,                     // Will be filled by caller
            dtb_size: 0,                     // Will be filled by caller
        },
    )
}

/// Parse ELF and load its segments into memory, return entry point
fn parse_elf_entry(elf_addr: usize) -> usize {
    // Read ELF header
    let elf_header = unsafe { core::slice::from_raw_parts(elf_addr as *const u8, 64) };

    // Check ELF magic number
    if &elf_header[0..4] != b"\x7FELF" {
        uart_println!("WARNING: Invalid ELF magic at {:#x}, using base address", elf_addr);
        return elf_addr;
    }

    // Read entry point from ELF64 header (offset 0x18, 8 bytes, little-endian)
    let entry_bytes = &elf_header[0x18..0x20];
    let entry = u64::from_le_bytes([
        entry_bytes[0], entry_bytes[1], entry_bytes[2], entry_bytes[3],
        entry_bytes[4], entry_bytes[5], entry_bytes[6], entry_bytes[7],
    ]) as usize;

    // Read program header offset and count
    let ph_off_bytes = &elf_header[0x20..0x28];
    let ph_off = u64::from_le_bytes([
        ph_off_bytes[0], ph_off_bytes[1], ph_off_bytes[2], ph_off_bytes[3],
        ph_off_bytes[4], ph_off_bytes[5], ph_off_bytes[6], ph_off_bytes[7],
    ]) as usize;

    let ph_num = u16::from_le_bytes([elf_header[0x38], elf_header[0x39]]) as usize;
    let ph_entsize = u16::from_le_bytes([elf_header[0x36], elf_header[0x37]]) as usize;

    uart_println!("ELF: entry={:#x}, {} program headers at offset {:#x}", entry, ph_num, ph_off);

    // Load each LOAD segment
    for i in 0..ph_num {
        let ph_addr = elf_addr + ph_off + (i * ph_entsize);
        let ph = unsafe { core::slice::from_raw_parts(ph_addr as *const u8, ph_entsize) };

        // Read program header type
        let p_type = u32::from_le_bytes([ph[0], ph[1], ph[2], ph[3]]);

        // PT_LOAD = 1
        if p_type == 1 {
            // Read segment information
            let p_offset_bytes = &ph[0x08..0x10];
            let p_offset = u64::from_le_bytes([
                p_offset_bytes[0], p_offset_bytes[1], p_offset_bytes[2], p_offset_bytes[3],
                p_offset_bytes[4], p_offset_bytes[5], p_offset_bytes[6], p_offset_bytes[7],
            ]) as usize;

            let p_vaddr_bytes = &ph[0x10..0x18];
            let p_vaddr = u64::from_le_bytes([
                p_vaddr_bytes[0], p_vaddr_bytes[1], p_vaddr_bytes[2], p_vaddr_bytes[3],
                p_vaddr_bytes[4], p_vaddr_bytes[5], p_vaddr_bytes[6], p_vaddr_bytes[7],
            ]) as usize;

            let p_filesz_bytes = &ph[0x20..0x28];
            let p_filesz = u64::from_le_bytes([
                p_filesz_bytes[0], p_filesz_bytes[1], p_filesz_bytes[2], p_filesz_bytes[3],
                p_filesz_bytes[4], p_filesz_bytes[5], p_filesz_bytes[6], p_filesz_bytes[7],
            ]) as usize;

            let p_memsz_bytes = &ph[0x28..0x30];
            let p_memsz = u64::from_le_bytes([
                p_memsz_bytes[0], p_memsz_bytes[1], p_memsz_bytes[2], p_memsz_bytes[3],
                p_memsz_bytes[4], p_memsz_bytes[5], p_memsz_bytes[6], p_memsz_bytes[7],
            ]) as usize;

            uart_println!("  LOAD segment {}: vaddr={:#x}, filesz={:#x}, memsz={:#x}",
                         i, p_vaddr, p_filesz, p_memsz);

            // Copy segment from ELF file to its load address
            // For identity mapping, physical address = virtual address
            unsafe {
                let src = (elf_addr + p_offset) as *const u8;
                let dst = p_vaddr as *mut u8;

                // Copy file contents
                if p_filesz > 0 {
                    core::ptr::copy_nonoverlapping(src, dst, p_filesz);
                }

                // Zero remaining memory (BSS section)
                if p_memsz > p_filesz {
                    let zero_start = (p_vaddr + p_filesz) as *mut u8;
                    let zero_len = p_memsz - p_filesz;
                    core::ptr::write_bytes(zero_start, 0, zero_len);
                }
            }
        }
    }

    entry
}

/// Update the rootserver structure with DTB information
///
/// This must be called after DTB is parsed to provide device tree info to kernel
pub fn update_rootserver_dtb(kernel_paddr: usize, dtb_addr: usize, dtb_size: usize) {
    let rootserver_addr = kernel_paddr + ROOTSERVER_OFFSET;

    uart_println!("Updating rootserver with DTB info...");
    uart_println!("  DTB: {:#x} (size: {})", dtb_addr, dtb_size);

    unsafe {
        let rootserver = rootserver_addr as *mut RootserverMem;
        // Update only the DTB fields, preserving other values
        (*rootserver).extra_bi = dtb_addr;
        (*rootserver).extra_bi_size = dtb_size;
    }
}
