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
/// This function demonstrates capability-based component spawning using sys_retype.
/// Memory is allocated from UntypedMemory capability delegated by root-task.
///
/// # Process
/// 1. Parse ELF binary (userspace)
/// 2. Allocate memory using sys_retype from UntypedMemory (capability-based!)
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
    spawn_from_elf_with_untyped(binary_data, priority, capabilities, 10)
}

/// Spawn a component using capability-based memory allocation
///
/// # Arguments
/// * `binary_data` - ELF binary data
/// * `priority` - Scheduling priority (0-255)
/// * `capabilities` - Capability bitmask for the new process
/// * `untyped_cap_slot` - Capability slot containing UntypedMemory capability
pub fn spawn_from_elf_with_untyped(
    binary_data: &[u8],
    priority: u8,
    capabilities: u64,
    untyped_cap_slot: usize,
) -> Result<SpawnResult> {
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

        // 2. Allocate memory using sys_retype from UntypedMemory capability
        // This is PROPER capability-based spawning - no direct kernel allocation!

        // Process image (with extra page for safety)
        let base_size = elf_info.memory_size();
        let process_size = ((base_size + 8192 + 4095) & !4095); // Round up to pages
        // Calculate log2 ceiling: round up to next power of 2, then take log2
        let process_size_pow2 = if process_size.is_power_of_two() {
            process_size
        } else {
            process_size.next_power_of_two()
        };
        let process_size_bits = process_size_pow2.trailing_zeros() as usize;

        // Sanity check: size_bits should be reasonable (12 to 25 = 4KB to 32MB)
        if process_size_bits < 12 || process_size_bits > 25 {
            printf!("[spawn_from_elf] ERROR: Invalid process_size_bits={} for size={}\n",
                    process_size_bits, process_size);
            return Err(Error::InvalidParameter);
        }

        // Allocate capability slots dynamically to avoid conflicts
        let process_cap_slot = syscall::cap_allocate()?;
        let stack_cap_slot = syscall::cap_allocate()?;

        // Allocate process memory from UntypedMemory (capability-based!)
        let process_phys = syscall::sys_retype(
            untyped_cap_slot,
            8, // CAP_TYPE_PAGE
            process_size_bits,
            0, // dest_cnode=0 means own CSpace
            process_cap_slot,
        )?;

        // Stack from UntypedMemory (16KB = 2^14)
        let stack_size = 16384;
        let stack_phys = syscall::sys_retype(
            untyped_cap_slot,
            8, // CAP_TYPE_PAGE
            14, // 2^14 = 16KB
            0,
            stack_cap_slot,
        )?;

        // Page table root - use traditional allocation for now
        // TODO: Implement PageTable initialization in sys_retype
        let pt_root = syscall::memory_allocate(4096)?;

        // CSpace root - use traditional allocation for now
        // TODO: Implement CNode initialization in sys_retype
        let cspace_root = syscall::memory_allocate(4096)?;

        printf!("[spawn_from_elf] Allocated from UntypedMemory: process={:#x}, stack={:#x}, pt={:#x}, cspace={:#x}\n",
                process_phys, stack_phys, pt_root, cspace_root);

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

        // 6. Map stack memory to get unique virtual address for this process
        // This ensures each process has its own stack and prevents stack collisions
        let stack_virt = syscall::memory_map(stack_phys, stack_size, RW_PERMS)?;
        // Stack grows DOWN, so SP starts at the TOP of the stack region
        let stack_top = stack_virt + stack_size;

        // Debug: log stack allocation
        printf!("[spawn_from_elf] Stack mapped: virt={:#x}, size={:#x}, stack_top={:#x}\n",
                stack_virt, stack_size, stack_top);

        // 7. Create process
        printf!("[spawn_from_elf] Calling process_create: entry={:#x}, stack={:#x}, pt={:#x}, cspace={:#x}\n",
                elf_info.entry_point, stack_top, pt_root, cspace_root);
        let pid = match syscall::process_create(
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
        ) {
            Ok(p) => {
                printf!("[spawn_from_elf] process_create succeeded, PID={:#x}\n", p);
                p
            }
            Err(e) => {
                printf!("[spawn_from_elf] process_create FAILED: {:?}\n", e);
                return Err(e);
            }
        };

        // 8. Get TCB capability
        let tcb_cap_slot = syscall::cap_allocate()?;
        syscall::cap_insert_self(tcb_cap_slot, 4 /* CAP_TCB */, pid)?;

        Ok(SpawnResult {
            tcb_cap_slot,
            pid,
        })
    }
}
