//! Root task creation and EL0 transition
//!
//! Creates the first userspace task (root task) and transitions to EL0.

use crate::arch::aarch64::page_table::PageTableFlags;
use crate::boot::{bootinfo, boot_info};
use crate::memory::{PhysAddr, VirtAddr, PAGE_SIZE, alloc_frame};
use crate::generated::memory_config;
use core::arch::naked_asm;

/// Root task creation error
#[derive(Debug)]
pub enum RootTaskError {
    /// Boot info not available
    BootInfoNotAvailable,
    /// Invalid boot info (zero addresses or sizes)
    InvalidBootInfo,
    /// Failed to allocate page table
    PageTableAllocation,
    /// Failed to map memory
    MemoryMapping,
    /// Failed to create userspace boot info
    BootInfoCreation,
}

/// Populate the boot info structure for userspace runtime services
///
/// This function creates and populates a BootInfo structure with:
/// - Untyped memory regions (available physical memory)
/// - Device MMIO regions (UART, RTC, Timer, etc.)
/// - Initial capability slots
/// - System configuration
unsafe fn populate_boot_info() -> Result<boot_info::BootInfo, RootTaskError> {
    let mut info = boot_info::BootInfo::new();

    // Set system configuration
    info.ram_size = memory_config::RAM_SIZE;
    info.kernel_virt_base = memory_config::KERNEL_BASE as u64;
    info.user_virt_start = memory_config::USER_VIRT_START;
    info.ipc_buffer_vaddr = 0x8000_0000; // Fixed IPC buffer location

    // TODO: Set capability slots when CSpace is implemented
    info.cspace_root_slot = 0; // Placeholder
    info.vspace_root_slot = 0; // Placeholder

    // Add device regions from platform configuration
    info.add_device_region(boot_info::DeviceRegion {
        paddr: memory_config::UART0_BASE,
        size: 0x1000,
        device_type: memory_config::DEVICE_UART0 as u32,
        irq: 33, // QEMU virt UART0 IRQ
    }).map_err(|_| RootTaskError::BootInfoCreation)?;

    info.add_device_region(boot_info::DeviceRegion {
        paddr: memory_config::UART1_BASE,
        size: 0x1000,
        device_type: memory_config::DEVICE_UART1 as u32,
        irq: 34, // QEMU virt UART1 IRQ
    }).map_err(|_| RootTaskError::BootInfoCreation)?;

    info.add_device_region(boot_info::DeviceRegion {
        paddr: memory_config::RTC_BASE,
        size: 0x1000,
        device_type: memory_config::DEVICE_RTC as u32,
        irq: 0xFFFFFFFF, // No IRQ
    }).map_err(|_| RootTaskError::BootInfoCreation)?;

    info.add_device_region(boot_info::DeviceRegion {
        paddr: memory_config::TIMER_BASE,
        size: 0x1000,
        device_type: memory_config::DEVICE_TIMER as u32,
        irq: 27, // QEMU virt timer IRQ
    }).map_err(|_| RootTaskError::BootInfoCreation)?;

    // Add untyped memory regions
    // TODO: Enumerate actual free memory regions from frame allocator
    // For now, add a large untyped region representing available RAM
    // This is a placeholder - proper implementation will query frame allocator

    // Example: 64MB of RAM starting at 64MB offset (after kernel)
    // Size bits: 26 = 64MB (2^26 = 67108864)
    info.add_untyped_region(boot_info::UntypedRegion::new(
        memory_config::RAM_BASE + 0x04000000, // 64MB offset
        26, // 64MB (2^26 bytes)
        false, // Not device memory
    )).map_err(|_| RootTaskError::BootInfoCreation)?;

    crate::kprintln!("[boot_info] Created userspace boot info:");
    crate::kprintln!("  Devices:  {} regions", info.num_device_regions);
    crate::kprintln!("  Untyped:  {} regions", info.num_untyped_regions);
    crate::kprintln!("  RAM size: {} MB", info.ram_size / (1024 * 1024));

    Ok(info)
}

/// Create and start the root task in EL0
///
/// This function:
/// 1. Creates a user page table with identity mapping
/// 2. Maps root task code/data into user address space
/// 3. Transitions to EL0 and jumps to root task entry point
///
/// # Safety
/// Must be called after kernel initialization with valid boot info.
pub unsafe fn create_and_start_root_task() -> ! {
    // Get boot info
    let boot_info = bootinfo::get_boot_info()
        .expect("[FATAL] Boot info not available");

    // Validate boot info
    if !boot_info.is_valid() {
        panic!("[FATAL] Invalid boot info");
    }

    crate::kprintln!("[root_task] Creating root task:");
    crate::kprintln!("  Root task image: {:#x} - {:#x} ({} KB)",
                   boot_info.root_task_start.as_usize(),
                   boot_info.root_task_end.as_usize(),
                   boot_info.root_task_size() / 1024);
    crate::kprintln!("  Entry point:     {:#x}", boot_info.root_task_entry);

    // Step 1: Create user page table
    // IMPORTANT: Since both kernel and user are at low addresses (0x40...), they both use TTBR0.
    // We MUST include kernel mappings in the user page table (with EL1-only permissions) so that
    // exception handlers can run when we're in EL0. The AP (access permission) bits in the page
    // table entries control whether EL0 can access these pages - kernel pages use AP_RW_EL1.
    let user_page_table_frame = crate::memory::alloc_frame()
        .expect("[FATAL] Failed to allocate user page table");

    let user_page_table_phys = user_page_table_frame.phys_addr();
    let user_page_table = &mut *(user_page_table_phys.as_usize()
        as *mut crate::arch::aarch64::page_table::PageTable);
    user_page_table.zero();

    crate::kprintln!("  User page table: {:#x}", user_page_table_phys.as_usize());

    // Create page mapper for user page table
    let mut mapper = crate::memory::PageMapper::new(user_page_table);

    // Map kernel regions with EL1-only permissions
    // This allows exception handlers to run while preventing user code from accessing kernel memory
    extern "C" {
        static _kernel_start: u8;
        static _kernel_end: u8;
    }
    let kernel_start = unsafe { &_kernel_start as *const u8 as usize };
    let kernel_end = unsafe { &_kernel_end as *const u8 as usize };

    crate::kprintln!("  Mapping kernel regions into user PT (EL1-only):");

    // Map kernel code/data
    crate::kprintln!("    Kernel: {:#x} - {:#x}", kernel_start, kernel_end);
    crate::memory::paging::identity_map_region(
        &mut mapper,
        kernel_start,
        kernel_end - kernel_start,
        PageTableFlags::KERNEL_RWX,
    ).expect("Failed to map kernel into user PT");

    // Map kernel stack/heap region (where kernel data structures live)
    let boot_info_ref = crate::boot::bootinfo::get_boot_info()
        .expect("[FATAL] Boot info not available");
    let memory_end = boot_info_ref.dtb_addr.as_usize() + 0x8000000; // RAM end (128MB)
    crate::kprintln!("    Kernel data: {:#x} - {:#x}", kernel_end, memory_end);
    crate::memory::paging::identity_map_region(
        &mut mapper,
        kernel_end,
        memory_end - kernel_end,
        PageTableFlags::KERNEL_DATA,
    ).expect("Failed to map kernel data into user PT");

    // Map UART device for syscall output
    crate::kprintln!("    UART: {:#x}", memory_config::UART0_BASE);
    crate::memory::paging::identity_map_region(
        &mut mapper,
        memory_config::UART0_BASE as usize,
        4096,
        PageTableFlags::KERNEL_DEVICE,
    ).expect("Failed to map UART into user PT");

    // Map GIC (interrupt controller) for IRQ handling in syscalls
    crate::kprintln!("    GIC Distributor: {:#x}", memory_config::GIC_DIST_BASE);
    crate::memory::paging::identity_map_region(
        &mut mapper,
        memory_config::GIC_DIST_BASE,
        memory_config::GIC_DIST_SIZE,
        PageTableFlags::KERNEL_DEVICE,
    ).expect("Failed to map GIC distributor into user PT");

    crate::kprintln!("    GIC CPU Interface: {:#x}", memory_config::GIC_CPU_BASE);
    crate::memory::paging::identity_map_region(
        &mut mapper,
        memory_config::GIC_CPU_BASE,
        memory_config::GIC_CPU_SIZE,
        PageTableFlags::KERNEL_DEVICE,
    ).expect("Failed to map GIC CPU interface into user PT");

    crate::kprintln!("  ✓ Kernel regions mapped");

    // Step 2: Map root task memory (code + data + rodata)
    // IMPORTANT: The elfloader embeds the entire ELF file (including headers),
    // but the entry point expects to start at the first LOAD segment.
    // ELF headers are typically 0x1000 bytes, so we need to skip them.

    let phys_file_start = boot_info.root_task_start.as_usize();
    let phys_end = boot_info.root_task_end.as_usize();
    let binary_size = phys_end - phys_file_start;

    // Virtual address: where the linker script expects it
    let entry_addr = boot_info.root_task_entry;

    // Parse ELF program headers to map each LOAD segment individually
    let elf_base = phys_file_start as *const u8;
    let e_phoff = unsafe { *(elf_base.add(32) as *const u64) }; // Program header offset
    let e_phnum = unsafe { *(elf_base.add(56) as *const u16) }; // Number of program headers
    let phdr_size = 56; // Size of ELF64 program header

    crate::kprintln!("  ELF header: {} program headers at offset {:#x}", e_phnum, e_phoff);

    // Iterate through all LOAD segments and map each one
    let mut total_pages = 0;
    for i in 0..e_phnum {
        let phdr_addr = (elf_base as usize + e_phoff as usize + (i as usize * phdr_size)) as *const u32;
        let p_type = unsafe { *phdr_addr };

        if p_type == 1 { // PT_LOAD
            // Read segment information
            let p_flags = unsafe { *((phdr_addr as usize + 4) as *const u32) };
            let p_offset = unsafe { *((phdr_addr as usize + 8) as *const u64) };
            let p_vaddr = unsafe { *((phdr_addr as usize + 16) as *const u64) };
            let p_filesz = unsafe { *((phdr_addr as usize + 32) as *const u64) };
            let p_memsz = unsafe { *((phdr_addr as usize + 40) as *const u64) };

            // Determine permissions from p_flags (PF_X=1, PF_W=2, PF_R=4)
            let is_writable = (p_flags & 2) != 0;
            let is_executable = (p_flags & 1) != 0;
            let flags = if is_executable && !is_writable {
                PageTableFlags::USER_RWX // Text segment (actually RX but we use RWX for simplicity)
            } else {
                PageTableFlags::USER_DATA // Data/rodata segment
            };

            crate::kprintln!("  LOAD segment {}:", i);
            crate::kprintln!("    vaddr:  {:#x}", p_vaddr);
            crate::kprintln!("    offset: {:#x}", p_offset);
            crate::kprintln!("    filesz: {:#x} ({} bytes)", p_filesz, p_filesz);
            crate::kprintln!("    memsz:  {:#x} ({} bytes)", p_memsz, p_memsz);
            crate::kprintln!("    flags:  {:#x} ({}{}{})", p_flags,
                if p_flags & 4 != 0 { "R" } else { "-" },
                if is_writable { "W" } else { "-" },
                if is_executable { "X" } else { "-" });

            // Calculate physical address for this segment
            let phys_addr = phys_file_start + p_offset as usize;

            // Map this segment (align to page boundaries)
            let virt_start = p_vaddr as usize & !(PAGE_SIZE - 1);
            let virt_end = ((p_vaddr as usize + p_memsz as usize) + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
            let seg_size = virt_end - virt_start;

            let mut current_virt = virt_start;
            let mut current_phys = phys_addr & !(PAGE_SIZE - 1);
            let mut page_count = 0;

            while current_virt < virt_end {
                if let Err(e) = mapper.map(
                    VirtAddr::new(current_virt),
                    PhysAddr::new(current_phys),
                    flags,
                    crate::memory::PageSize::Size4KB,
                ) {
                    panic!("[FATAL] Failed to map page at virt={:#x} phys={:#x}: {:?}",
                        current_virt, current_phys, e);
                }

                current_virt += PAGE_SIZE;
                current_phys += PAGE_SIZE;
                page_count += 1;
            }

            crate::kprintln!("    Mapped {} pages ({} KB)", page_count, seg_size / 1024);
            total_pages += page_count;
        }
    }
    crate::kprintln!("  Total: {} pages mapped for all LOAD segments", total_pages);

    // Map stack (1MB below entry point, 256KB size)
    // Ensure stack_top is page-aligned by rounding entry_addr down to page boundary, then subtracting
    let stack_top = (entry_addr & !(PAGE_SIZE - 1)) - PAGE_SIZE;
    let stack_size = 256 * 1024; // 256KB stack
    let stack_bottom = stack_top - stack_size;

    crate::kprintln!("  Mapping stack: {:#x} - {:#x}", stack_bottom, stack_top);

    // Allocate physical frames for the stack and map them
    let num_stack_pages = stack_size / PAGE_SIZE;
    for i in 0..num_stack_pages {
        let virt_addr = stack_bottom + (i * PAGE_SIZE);
        let phys_frame = alloc_frame().expect("Failed to allocate stack frame");
        let phys_addr = phys_frame.phys_addr();

        if let Err(e) = mapper.map(
            VirtAddr::new(virt_addr),
            phys_addr,
            PageTableFlags::USER_DATA,
            crate::memory::PageSize::Size4KB,
        ) {
            panic!("[FATAL] Failed to map stack page at {:#x}: {:?}", virt_addr, e);
        }
    }

    // Map heap for root-task allocator (256KB at 32MB mark)
    // This enables use of alloc-based collections (Vec, BTreeMap) for IPC broker
    const HEAP_START: usize = 0x200_0000; // 32MB
    const HEAP_SIZE: usize = 0x40000;     // 256KB
    crate::kprintln!("  Mapping heap: {:#x} - {:#x} (256 KB)", HEAP_START, HEAP_START + HEAP_SIZE);

    // Allocate physical frames for the heap and map them
    let num_heap_pages = HEAP_SIZE / PAGE_SIZE;
    for i in 0..num_heap_pages {
        let virt_addr = HEAP_START + (i * PAGE_SIZE);
        let phys_frame = alloc_frame().expect("Failed to allocate heap frame");
        let phys_addr = phys_frame.phys_addr();

        if let Err(e) = mapper.map(
            VirtAddr::new(virt_addr),
            phys_addr,
            PageTableFlags::USER_DATA,
            crate::memory::PageSize::Size4KB,
        ) {
            panic!("[FATAL] Failed to map heap page at {:#x}: {:?}", virt_addr, e);
        }
    }

    // UART already mapped with kernel permissions earlier, no need to map again

    crate::kprintln!("  Entry point:     {:#x}", entry_addr);
    crate::kprintln!("  Stack:           {:#x} - {:#x} (256 KB)", stack_bottom, stack_top);
    crate::kprintln!("  Heap:            {:#x} - {:#x} (256 KB)", HEAP_START, HEAP_START + HEAP_SIZE);
    crate::kprintln!("  ✓ Root task ready for EL0 transition");

    // Step 2b: Create and map boot info for userspace runtime services
    crate::kprintln!("");
    crate::kprintln!("[root_task] Creating boot info for runtime services...");

    // Create boot info structure
    let userspace_boot_info = populate_boot_info().expect("[FATAL] Failed to populate boot info");

    // Allocate physical frame for boot info
    let boot_info_frame = crate::memory::alloc_frame()
        .expect("[FATAL] Failed to allocate boot info frame");
    let boot_info_phys = boot_info_frame.phys_addr();

    // Write boot info to physical frame
    let boot_info_ptr = boot_info_phys.as_usize() as *mut boot_info::BootInfo;
    core::ptr::write(boot_info_ptr, userspace_boot_info);

    // Map boot info at fixed virtual address (0x7FFF_F000 - just below IPC buffer)
    const BOOT_INFO_VADDR: usize = 0x7FFF_F000;
    mapper.map(
        VirtAddr::new(BOOT_INFO_VADDR),
        boot_info_phys,
        PageTableFlags::USER_DATA, // Read-only would be better, but use RW for now
        crate::memory::PageSize::Size4KB,
    ).expect("[FATAL] Failed to map boot info");

    crate::kprintln!("  Boot info phys:  {:#x}", boot_info_phys.as_usize());
    crate::kprintln!("  Boot info virt:  {:#x}", BOOT_INFO_VADDR);
    crate::kprintln!("  Boot info size:  {} bytes", boot_info::BootInfo::size());
    crate::kprintln!("  ✓ Boot info mapped for userspace");

    // Step 3: Create CNode for root task capability space
    crate::kprintln!("  Creating CNode for capability space...");
    let cnode_frame = crate::memory::alloc_frame()
        .expect("[FATAL] Failed to allocate CNode frame");
    let cnode_phys = cnode_frame.phys_addr();
    let cnode_ptr = cnode_phys.as_usize() as *mut crate::objects::CNode;
    crate::kprintln!("  CNode frame allocated at: {:#x}", cnode_ptr as usize);

    // Allocate slots array for CNode (256 slots * 16 bytes/cap = 4096 bytes = 1 frame)
    let slots_frame = crate::memory::alloc_frame()
        .expect("[FATAL] Failed to allocate CNode slots frame");
    let slots_phys = slots_frame.phys_addr();
    crate::kprintln!("  CNode slots allocated at: {:#x}", slots_phys.as_usize());

    // Create CNode with 256 slots (2^8 = 256 capabilities)
    let cnode = crate::objects::CNode::new(8, slots_phys)
        .expect("[FATAL] Failed to create CNode");
    core::ptr::write(cnode_ptr, cnode);
    crate::kprintln!("  CNode:           {:#x} (256 slots, slots at {:#x})", cnode_ptr as usize, slots_phys.as_usize());

    // Step 3b: Create IRQControl capability for root-task
    crate::kprintln!("  Creating IRQControl capability...");

    // Allocate frame for IRQControl object
    let irq_control_frame = crate::memory::alloc_frame()
        .expect("[FATAL] Failed to allocate IRQControl frame");
    let irq_control_phys = irq_control_frame.phys_addr();
    let irq_control_ptr = irq_control_phys.as_usize() as *mut crate::objects::IRQControl;

    // Initialize IRQControl object
    let irq_control = crate::objects::IRQControl::new();
    core::ptr::write(irq_control_ptr, irq_control);

    // Create IRQControl capability
    let irq_control_cap = crate::objects::Capability::new(
        crate::objects::CapType::IrqControl,
        irq_control_ptr as usize,
    );

    // Insert IRQControl capability into slot 0 of root-task's CSpace
    const IRQ_CONTROL_SLOT: usize = 0;
    (*cnode_ptr).insert(IRQ_CONTROL_SLOT, irq_control_cap)
        .expect("[FATAL] Failed to insert IRQControl capability");

    crate::kprintln!("  IRQControl:      slot {} → {:#x}", IRQ_CONTROL_SLOT, irq_control_ptr as usize);

    // Update boot_info with IRQControl physical address (for delegation to drivers)
    (*boot_info_ptr).irq_control_paddr = irq_control_phys.as_usize() as u64;

    // Step 4: Create TCB for root task
    crate::kprintln!("  Creating root TCB...");
    let root_tcb_frame = crate::memory::alloc_frame()
        .expect("[FATAL] Failed to allocate TCB frame");
    let root_tcb_ptr = root_tcb_frame.phys_addr().as_usize() as *mut crate::objects::TCB;
    crate::kprintln!("  Root TCB frame:  {:#x}", root_tcb_ptr as usize);

    // Create TCB with TID 1 (0 is idle thread)
    crate::kprintln!("  Initializing TCB...");
    let root_tcb = crate::objects::TCB::new(
        1,                                     // TID = 1 for root-task
        cnode_ptr,                             // CSpace root (CNode)
        user_page_table_phys.as_usize(),       // VSpace root (page table)
        VirtAddr::new(0x8000_0000),            // IPC buffer (not used yet)
        entry_addr as u64,                     // Entry point
        stack_top as u64,                      // Stack pointer
        crate::objects::TCB::CAP_ALL,          // Root-task gets ALL capabilities
    );
    crate::kprintln!("  Writing TCB...");
    core::ptr::write(root_tcb_ptr, root_tcb);

    crate::kprintln!("  Setting state to Running...");
    // Set state to Running (root-task will start executing immediately)
    (*root_tcb_ptr).set_state(crate::objects::ThreadState::Running);

    // Initialize next_virt_addr to start after root-task's mapped regions
    // Root-task ELF segments: USER_VIRT_START - ~1MB
    // Loader temp mappings: LOADER_VIRT_START - LOADER_VIRT_END (reserved for component loading)
    // IPC shared memory: IPC_VIRT_START - IPC_VIRT_END
    // Stack: 0x7ffbf000 - 0x7ffff000
    // Heap:  0x2000000 - 0x2040000
    // Start allocating from USER_DYNAMIC_VIRT_START (configured in build-config.toml)
    const ROOT_TASK_VIRT_ALLOC_START: u64 = memory_config::USER_DYNAMIC_VIRT_START;
    (*root_tcb_ptr).alloc_virt_range(0); // Initialize allocator
    // Manually set it to the correct start value
    unsafe {
        // Direct access to update next_virt_addr
        let tcb_mut = &mut *root_tcb_ptr;
        let start_offset = ROOT_TASK_VIRT_ALLOC_START - memory_config::USER_VIRT_START;
        tcb_mut.alloc_virt_range(start_offset); // Advance allocator to start position
    }

    crate::kprintln!("  Setting priority to 255 (lowest)...");
    // Root-task should have lowest priority (255) so it only runs when nothing else can run
    // This ensures spawned components at priority 200 (test) or 100 (IPC) run before root-task
    (*root_tcb_ptr).set_priority(255);

    crate::kprintln!("  Setting saved_ttbr0...");
    // Set saved_ttbr0 for context switching
    (*root_tcb_ptr).context_mut().saved_ttbr0 = user_page_table_phys.as_usize() as u64;

    crate::kprintln!("  Registering with scheduler...");
    // Register with scheduler as current thread
    crate::scheduler::test_set_current_thread(root_tcb_ptr);

    crate::kprintln!("  Root TCB:        {:#x} ✓", root_tcb_ptr as usize);

    // Step 5: Transition to EL0
    crate::kprintln!("");
    crate::kprintln!("[root_task] Transitioning to EL0...");
    crate::kprintln!("  Entry:    {:#x}", entry_addr);
    crate::kprintln!("  Stack:    {:#x}", stack_top);
    crate::kprintln!("  TTBR0:    {:#x}", user_page_table_phys.as_usize());
    crate::kprintln!("  About to call transition_to_el0...");
    crate::kprintln!("  VBAR_EL1: {:#x}", unsafe {
        let vbar: usize;
        core::arch::asm!("mrs {}, vbar_el1", out(reg) vbar);
        vbar
    });
    crate::kprintln!("  CurrentEL before: {:#x}", unsafe {
        let el: usize;
        core::arch::asm!("mrs {}, currentel", out(reg) el);
        el
    });

    // Clean data cache and invalidate instruction cache for the mapped user code
    // This ensures the CPU sees the code we just mapped
    unsafe {
        core::arch::asm!(
            "dc civac, {addr}",  // Clean and invalidate data cache by VA
            "ic ivau, {addr}",   // Invalidate instruction cache by VA
            "dsb ish",           // Data synchronization barrier
            "isb",               // Instruction synchronization barrier
            addr = in(reg) entry_addr,
        );
    }
    crate::kprintln!("  Cache flushed");

    // Verify the code is actually at the physical address
    let code_phys_offset = 0x10000; // First LOAD segment offset from ELF
    let code_phys = (phys_file_start + code_phys_offset) & !(PAGE_SIZE - 1);
    let first_instruction = unsafe { *(code_phys as *const u32) };
    crate::kprintln!("  First instruction at phys {:#x}: {:#x}", code_phys, first_instruction);

    // Expected first instruction from objdump
    crate::kprintln!("  Expected: 0xa9ba7bfd (stp x29, x30, [sp, #-0x60]!)");

    // Also verify we can read from the mapped virtual address using the user page table
    // We need to temporarily switch to the user page table to test this
    crate::kprintln!("  User PT is at phys {:#x}", user_page_table_phys.as_usize());


    // Directly transition using inline assembly to avoid any compiler-generated cleanup code
    core::arch::asm!(
        // Set up TTBR0_EL1 (user page table)
        // In ARM64, TTBR0_EL1 is used for both EL1 and EL0 translations
        // The TCR_EL1.EPD0 bit controls whether TTBR0 is used at EL0
        "msr ttbr0_el1, {ttbr0}",
        "isb",
        // Invalidate TLB for TTBR0
        "tlbi vmalle1",
        "dsb ish",
        "isb",
        // Set up ELR_EL1 (return address = user entry point)
        "msr elr_el1, {entry}",
        // Set up SP_EL0 (user stack pointer)
        "msr sp_el0, {sp}",
        // Set up SPSR_EL1 for EL0
        // M[3:0] = 0b0000 (EL0t - EL0 with SP_EL0)
        // F = 0 (FIQ not masked)
        // I = 0 (IRQ not masked)
        // A = 0 (SError not masked)
        // D = 1 (Debug exceptions masked)
        "mov x3, #0x0",  // All zeros = EL0t with interrupts enabled
        "msr spsr_el1, x3",
        // Perform exception return to EL0
        "eret",
        entry = in(reg) entry_addr,
        sp = in(reg) stack_top,
        ttbr0 = in(reg) user_page_table_phys.as_usize(),
        options(noreturn)
    );
}

/// Transition to EL0 and jump to user code
///
/// # Safety
/// Must be called with valid entry point, stack, and page table.
#[unsafe(naked)]
unsafe extern "C" fn transition_to_el0(entry: usize, sp: usize, ttbr0: usize) -> ! {
    naked_asm!(
        // Set up TTBR0_EL1 (user page table)
        // In ARM64, TTBR0_EL1 is used for user space translations at EL0
        "msr ttbr0_el1, x2",
        "isb",

        // Set up ELR_EL1 (return address = user entry point)
        "msr elr_el1, x0",

        // Set up SP_EL0 (user stack pointer)
        "msr sp_el0, x1",

        // Set up SPSR_EL1 for EL0
        // - Mode: EL0t (0b0000)
        // - All interrupt bits cleared (D=0, A=0, I=0, F=0) - interrupts enabled
        "mov x3, #0",
        "msr spsr_el1, x3",

        // Perform exception return to EL0
        "eret",
    )
}
