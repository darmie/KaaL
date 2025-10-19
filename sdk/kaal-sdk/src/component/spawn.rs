//! Component spawning from userspace
//!
//! Allows privileged components like system_init to spawn other components.
//! Uses existing syscalls - no kernel changes needed!

use crate::{Result, Error, elf, syscall};

/// Result of spawning a component
#[derive(Debug, Clone, Copy)]
pub struct SpawnResult {
    /// TCB capability slot in caller's CSpace
    pub tcb_cap_slot: usize,
    /// Process ID (TCB physical address)
    pub pid: usize,
}

/// Spawn a component from ELF binary data
///
/// This function demonstrates userspace component spawning using only existing syscalls.
/// No kernel changes needed!
///
/// # Process
/// 1. Parse ELF binary (userspace)
/// 2. Allocate memory for process image, stack, page table, CSpace
/// 3. Map memory and copy ELF segments
/// 4. Call SYS_PROCESS_CREATE to create TCB
/// 5. Insert TCB capability into caller's CSpace
///
/// # Arguments
/// * `binary_data` - ELF binary data
/// * `priority` - Scheduling priority (0-255)
/// * `capabilities` - Capability bitmask for the new process
///
/// # Returns
/// * `Ok(SpawnResult)` with TCB capability and PID
/// * `Err(Error)` if spawn fails
///
/// # Example
/// ```no_run
/// let binary_data = include_bytes!("../path/to/component");
/// // Grant IPC capabilities only (bit 2)
/// let caps = 1 << 2;
/// let result = spawn_from_elf(binary_data, 10, caps)?;
/// println!("Spawned with PID: {}", result.pid);
/// ```
pub fn spawn_from_elf(binary_data: &[u8], priority: u8, capabilities: u64) -> Result<SpawnResult> {
    unsafe {
        // Debug: log binary size
        use crate::printf;
        printf!("[spawn_from_elf] binary_data.len() = {} bytes\n", binary_data.len());

        // 1. Parse ELF
        let elf_info = elf::parse_elf(binary_data)
            .map_err(|_| Error::InvalidElf)?;

        // Debug: log parsed ELF info
        printf!("[spawn_from_elf] Parsed ELF: entry={:#x}, num_segments={}, memory_size={:#x}\n",
                elf_info.entry_point, elf_info.num_segments, elf_info.memory_size());

        // 2. Allocate memory
        // Process image (with extra page for safety)
        let base_size = elf_info.memory_size();
        let process_size = ((base_size + 8192 + 4095) & !4095); // Round up to pages
        let process_phys = syscall::memory_allocate(process_size)?;

        // Stack (16KB)
        let stack_size = 16384;
        let stack_phys = syscall::memory_allocate(stack_size)?;

        // Page table root (4KB)
        let pt_root = syscall::memory_allocate(4096)?;

        // CSpace root (4KB)
        let cspace_root = syscall::memory_allocate(4096)?;

        // 3. Map memory to copy segments
        const RW_PERMS: usize = 0x3;
        let virt_mem = syscall::memory_map(process_phys, process_size, RW_PERMS)?;

        // 4. Copy ELF segments
        for i in 0..elf_info.num_segments {
            let (vaddr, filesz, _memsz, file_offset) = elf_info.segments[i];
            let segment_offset = vaddr - elf_info.min_vaddr;
            let dest_ptr = (virt_mem + segment_offset) as *mut u8;
            let src_ptr = binary_data.as_ptr().add(file_offset);

            // Debug: show what we're copying
            printf!("[spawn_from_elf] Segment {}: vaddr={:#x}, filesz={:#x}, file_offset={:#x}\n",
                    i, vaddr, filesz, file_offset);

            // Show first 16 bytes of source
            printf!("  Source bytes: ");
            for j in 0..16.min(filesz) {
                printf!("{:02x} ", *src_ptr.add(j));
            }
            printf!("\n");

            // Copy segment data
            core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, filesz);
        }

        // 5. Unmap temporary mapping
        syscall::memory_unmap(virt_mem, process_size)?;

        // 6. Create process
        let stack_top = 0x80000000; // Standard user stack location
        let pid = syscall::process_create(
            elf_info.entry_point,
            stack_top,
            pt_root,
            cspace_root,
            process_phys,
            elf_info.min_vaddr, // code_vaddr from ELF
            process_size,
            stack_phys,
            priority,
            capabilities,  // Pass capabilities to new process
        )?;

        // 7. Get TCB capability
        let tcb_cap_slot = syscall::cap_allocate()?;
        syscall::cap_insert_self(tcb_cap_slot, 4 /* CAP_TCB */, pid)?;

        Ok(SpawnResult {
            tcb_cap_slot,
            pid,
        })
    }
}
