//! Component Discovery and Loading
//!
//! This module handles:
//! - Parsing component manifests from PROJECT_ROOT/components.toml
//! - Loading component binaries
//! - Spawning components with proper capabilities
//! - Component lifecycle management
//!
//! The components.toml manifest is located at the project root for developer convenience,
//! and is embedded into the binary at build time via build.rs.

use core::str;

/// Components manifest embedded at build time from PROJECT_ROOT/components.toml
///
/// This allows developers to configure components at the project root without
/// dealing with runtime/ or kernel/ directories.
const COMPONENTS_TOML: &str = include_str!(env!("COMPONENTS_TOML_PATH"));

/// Component type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// Device driver (hardware access)
    Driver,
    /// System service (no hardware)
    Service,
    /// User application
    Application,
}

/// Component capability specification
#[derive(Debug, Clone, Copy)]
pub enum ComponentCapability {
    /// Memory mapping: (physical_addr, size)
    MemoryMap { phys_addr: usize, size: usize },
    /// Interrupt capability
    Interrupt { irq: u32 },
    /// IPC endpoint access
    IpcEndpoint { name: &'static str },
    /// Process management
    ProcessCreate,
    ProcessDestroy,
    /// General memory allocation
    MemoryAllocate,
}

/// Result of spawning a component
///
/// Contains all capabilities needed to manage the spawned component.
/// This enables the parent to:
/// - Map memory into child's VSpace (vspace_cap)
/// - Grant capabilities to child (cspace_cap)
/// - Control child's execution (tcb_cap)
/// - Identify child for logging (pid)
#[derive(Debug, Clone, Copy)]
pub struct SpawnResult {
    /// Capability slot number for child's TCB in parent's CSpace
    pub tcb_cap_slot: usize,
    /// Physical address of child's TCB (for direct kernel operations)
    pub tcb_phys: usize,
    /// Physical address of child's VSpace (page table root)
    pub vspace_cap: usize,
    /// Physical address of child's CSpace (capability space)
    pub cspace_cap: usize,
    /// Process ID (for debugging/logging)
    pub pid: usize,
}

/// Component descriptor from manifest
#[derive(Debug)]
pub struct ComponentDescriptor {
    /// Component name
    pub name: &'static str,
    /// Binary name (without extension)
    pub binary: &'static str,
    /// Component type
    pub component_type: ComponentType,
    /// Scheduling priority (0-255)
    pub priority: u8,
    /// Should spawn automatically at boot
    pub autostart: bool,
    /// Required capabilities (as strings)
    pub capabilities: &'static [&'static str],
    /// Required capabilities (as bitmask)
    /// Bit 0: CAP_MEMORY, Bit 1: CAP_PROCESS, Bit 2: CAP_IPC, Bit 3: CAP_CAPS
    pub capabilities_bitmask: u64,
    /// Embedded binary data (set at compile time)
    pub binary_data: Option<&'static [u8]>,
}

impl ComponentDescriptor {
    /// Create a new component descriptor
    pub const fn new(
        name: &'static str,
        binary: &'static str,
        component_type: ComponentType,
    ) -> Self {
        Self {
            name,
            binary,
            component_type,
            priority: 100,
            autostart: false,
            capabilities: &[],
            capabilities_bitmask: 0,
            binary_data: None,
        }
    }

    /// Set priority
    pub const fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set autostart
    pub const fn with_autostart(mut self, autostart: bool) -> Self {
        self.autostart = autostart;
        self
    }

    /// Set capabilities
    pub const fn with_capabilities(mut self, caps: &'static [&'static str]) -> Self {
        self.capabilities = caps;
        self
    }

    /// Set binary data
    pub const fn with_binary(mut self, data: &'static [u8]) -> Self {
        self.binary_data = Some(data);
        self
    }
}

/// Component registry - statically defined components
///
/// In a future version, this could be generated from components.toml at build time.
/// For now, we define components programmatically.
pub struct ComponentRegistry {
    components: &'static [ComponentDescriptor],
}

impl ComponentRegistry {
    /// Create a new registry with the given components
    pub const fn new(components: &'static [ComponentDescriptor]) -> Self {
        Self { components }
    }

    /// Get all components
    pub fn components(&self) -> &[ComponentDescriptor] {
        self.components
    }

    /// Get components that should autostart
    pub fn autostart_components(&self) -> impl Iterator<Item = &ComponentDescriptor> {
        self.components.iter().filter(|c| c.autostart)
    }

    /// Find a component by name
    pub fn find(&self, name: &str) -> Option<&ComponentDescriptor> {
        self.components.iter().find(|c| c.name == name)
    }
}

/// Component loader - handles spawning components
pub struct ComponentLoader {
    registry: &'static ComponentRegistry,
    irq_control_paddr: usize,
}

impl ComponentLoader {
    /// Create a new component loader
    pub const fn new(registry: &'static ComponentRegistry, irq_control_paddr: usize) -> Self {
        Self { registry, irq_control_paddr }
    }

    /// Spawn a component by name
    ///
    /// Returns SpawnResult with capabilities on success
    pub unsafe fn spawn(&self, name: &str) -> Result<SpawnResult, ComponentError> {
        let descriptor = self.registry
            .find(name)
            .ok_or(ComponentError::NotFound)?;

        self.spawn_component(descriptor)
    }

    /// Spawn all autostart components
    pub unsafe fn spawn_autostart(&self) -> Result<(), ComponentError> {
        for component in self.registry.autostart_components() {
            match self.spawn_component(component) {
                Ok(result) => {
                    crate::sys_print("[component_loader] Spawned: ");
                    crate::sys_print(component.name);
                    crate::sys_print(" (PID ");
                    // Print PID (simplified - would need proper number formatting)
                    crate::sys_print(")\n");
                }
                Err(e) => {
                    crate::sys_print("[component_loader] Failed to spawn: ");
                    crate::sys_print(component.name);
                    crate::sys_print("\n");
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// Internal: Spawn a single component
    unsafe fn spawn_component(&self, desc: &ComponentDescriptor) -> Result<SpawnResult, ComponentError> {
        // 1. Get binary data
        let binary_data = desc.binary_data.ok_or(ComponentError::NoBinary)?;

        // Debug: Check what binary we got
        crate::sys_print("[loader] Spawning component: ");
        crate::sys_print(desc.name);
        crate::sys_print(", binary_data len=");
        crate::print_number(binary_data.len());
        crate::sys_print(", contains: ");
        // Search for distinctive strings to identify the binary
        let has_producer = binary_data.windows(18).any(|w| w == b"IPC Producer v0.1.");
        let has_consumer = binary_data.windows(20).any(|w| w == b"THIS IS THE CONSUMER");

        if has_producer {
            crate::sys_print("PRODUCER ");
        }
        if has_consumer {
            crate::sys_print("CONSUMER ");
        }
        if !has_producer && !has_consumer {
            crate::sys_print("UNKNOWN");
        }
        crate::sys_print("\n");

        // 2. Parse ELF
        let elf_info = crate::elf::parse_elf(binary_data)
            .map_err(|_| ComponentError::InvalidElf)?;

        // Debug: Print ELF info
        crate::sys_print("[loader] ELF for ");
        crate::sys_print(desc.name);
        crate::sys_print(":\n");
        crate::sys_print("  Entry: 0x");
        crate::print_hex(elf_info.entry_point);
        crate::sys_print("\n");
        crate::sys_print("  Segments:\n");
        for i in 0..elf_info.num_segments {
            let (vaddr, filesz, memsz, _offset) = elf_info.segments[i];
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
        crate::sys_print("  Total range: 0x");
        crate::print_hex(elf_info.min_vaddr);
        crate::sys_print(" - 0x");
        crate::print_hex(elf_info.max_vaddr);
        crate::sys_print("\n");

        // 3. Allocate memory for process image
        // Future-proof: Always allocate an extra page beyond the highest address
        // This ensures entry stubs at the end of .text have room to execute
        let base_size = elf_info.memory_size();
        let extra_safety = 4096;  // One extra page for entry stub safety
        let process_size = ((base_size + extra_safety + 4095) & !4095);  // Round up to pages
        let process_mem = crate::sys_memory_allocate(process_size);
        if process_mem == usize::MAX {
            return Err(ComponentError::OutOfMemory);
        }

        // 4. Allocate stack (16KB)
        let stack_size = 16384;
        let stack_mem = crate::sys_memory_allocate(stack_size);
        if stack_mem == usize::MAX {
            return Err(ComponentError::OutOfMemory);
        }

        // 5. Allocate page table root (4KB)
        let pt_root = crate::sys_memory_allocate(4096);
        if pt_root == usize::MAX {
            return Err(ComponentError::OutOfMemory);
        }
        crate::sys_print("[loader] Allocated PT for ");
        crate::sys_print(desc.name);
        crate::sys_print(" at 0x");
        crate::print_hex(pt_root);
        crate::sys_print("\n");

        // 6. Allocate CNode for capability space
        // CNode needs:
        // - CNode struct (~24 bytes)
        // - Capability slots array (256 slots × 32 bytes = 8KB = 2 pages)
        // Total: 3 pages minimum (12KB) to avoid overlap with TCB
        let cspace_root = crate::sys_memory_allocate(12288); // 3 pages
        if cspace_root == usize::MAX {
            return Err(ComponentError::OutOfMemory);
        }

        // 7. Map the allocated physical memory so we can copy the ELF segments
        const RW_PERMS: usize = 0x3; // Read + Write
        crate::sys_print("[loader] Mapping phys 0x");
        crate::print_hex(process_mem);
        crate::sys_print(" size=0x");
        crate::print_hex(process_size);
        crate::sys_print(" for copying\n");

        let virt_mem = crate::sys_memory_map(process_mem, process_size, RW_PERMS);
        if virt_mem == usize::MAX {
            return Err(ComponentError::OutOfMemory);
        }

        crate::sys_print("[loader] Mapped to virt 0x");
        crate::print_hex(virt_mem);
        crate::sys_print(", will copy ");
        crate::print_number(binary_data.len());
        crate::sys_print(" bytes from binary_data\n");

        // 8. Copy each LOAD segment to the mapped memory
        let base_vaddr = elf_info.min_vaddr;

        // Debug: Show first few bytes of source binary
        if binary_data.len() >= 4 {
            crate::sys_print("[loader] Binary data first 4 bytes: ");
            let word = u32::from_le_bytes([binary_data[0], binary_data[1], binary_data[2], binary_data[3]]);
            crate::print_hex(word as usize);
            crate::sys_print("\n");
        }

        for i in 0..elf_info.num_segments {
            let (vaddr, filesz, memsz, offset) = elf_info.segments[i];

            // Calculate destination in mapped memory
            let segment_offset = vaddr - base_vaddr;
            let dest_ptr = (virt_mem + segment_offset) as *mut u8;
            let src_ptr = binary_data.as_ptr().add(offset);

            // Copy file data
            if filesz > 0 {
                crate::sys_print("[loader] Copying segment ");
                crate::print_number(i);
                crate::sys_print(": ");
                crate::print_number(filesz);
                crate::sys_print(" bytes from src=0x");
                crate::print_hex(src_ptr as usize);
                crate::sys_print(" to dest=0x");
                crate::print_hex(dest_ptr as usize);
                crate::sys_print("\n");
                core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, filesz);
            }

            // Zero BSS (memsz > filesz means there's BSS to zero)
            if memsz > filesz {
                let bss_ptr = dest_ptr.add(filesz);
                let bss_size = memsz - filesz;
                core::ptr::write_bytes(bss_ptr, 0, bss_size);
            }
        }

        // Debug: Show what's at the entry point
        let entry_offset = elf_info.entry_point - base_vaddr;
        let entry_ptr = (virt_mem + entry_offset) as *const u32;
        let entry_instr = unsafe { *entry_ptr };
        crate::sys_print("[loader] Entry point 0x");
        crate::print_hex(elf_info.entry_point);
        crate::sys_print(": first instruction = 0x");
        crate::print_hex(entry_instr as usize);
        crate::sys_print("\n");

        // 9. Unmap the memory (we're done writing to it)
        crate::sys_print("[loader] Unmapping virt 0x");
        crate::print_hex(virt_mem);
        crate::sys_print("\n");
        crate::sys_memory_unmap(virt_mem, process_size);
        crate::sys_print("[loader] Unmap complete\n");

        // 10. Map stack memory to get unique virtual address for this process
        // This ensures each process has its own stack and prevents stack collisions
        let stack_virt = crate::sys_memory_map(stack_mem, stack_size, 0x3);  // RW permissions
        if stack_virt == usize::MAX {
            crate::sys_print("[loader] ERROR: Failed to map stack memory\n");
            return Err(ComponentError::OutOfMemory);
        }
        // Stack grows DOWN, so SP starts at the TOP of the stack region
        let stack_top = stack_virt + stack_size;

        crate::sys_print("[loader] Stack mapped: virt=0x");
        crate::print_hex(stack_virt);
        crate::sys_print(", size=0x");
        crate::print_hex(stack_size);
        crate::sys_print(", stack_top=0x");
        crate::print_hex(stack_top);
        crate::sys_print("\n");

        crate::sys_print("[loader] Calling sys_process_create with code_phys=0x");
        crate::print_hex(process_mem);
        crate::sys_print(", code_size=0x");
        crate::print_hex(process_size);
        crate::sys_print(", pt_root=0x");
        crate::print_hex(pt_root);
        crate::sys_print("\n");

        // Use capabilities from component descriptor
        let capabilities = desc.capabilities_bitmask;

        let result = crate::sys_process_create(
            elf_info.entry_point,
            stack_top,
            pt_root,
            cspace_root,
            process_mem,
            elf_info.min_vaddr,  // Virtual address where code should be mapped
            process_size,
            stack_mem,
            desc.priority,  // Pass the component priority from manifest
            capabilities,  // Pass parsed capabilities from manifest
        );

        if result.pid == usize::MAX {
            return Err(ComponentError::OutOfMemory);
        }

        // Allocate capability slot for TCB in our CSpace
        // Start at slot 200 to avoid initial capabilities populated during boot
        static mut NEXT_CAP_SLOT: usize = 10;
        let tcb_cap_slot = NEXT_CAP_SLOT;
        NEXT_CAP_SLOT += 1;

        // Insert TCB capability into our CSpace at the allocated slot
        // CapType::Tcb = 4
        crate::sys_print("[loader] Inserting TCB cap: slot=");
        crate::print_number(tcb_cap_slot);
        crate::sys_print(", tcb_phys=0x");
        crate::print_hex(result.tcb_phys);
        crate::sys_print("\n");

        let insert_result = crate::sys_cap_insert_self(tcb_cap_slot, 4, result.tcb_phys);
        if insert_result != 0 {
            crate::sys_print("[loader] Warning: Failed to insert TCB capability\n");
        }

        // Check if component needs IRQControl and delegate it
        // IRQControl capability is at slot 0 in root-task's CSpace (from boot_info)
        // If component has irq:control capability, insert IRQControl into its CSpace at slot 0
        const IRQ_CONTROL_BIT: u64 = 1 << 10; // irq:control capability bit
        if (capabilities & IRQ_CONTROL_BIT) != 0 && self.irq_control_paddr != 0 {
            crate::sys_print("[loader] Delegating IRQControl to ");
            crate::sys_print(desc.name);
            crate::sys_print("\n");

            // Insert IRQControl into component's CSpace at slot 1 (slot 0 is reserved)
            // sys_cap_insert_into(target_tcb_cap, target_slot, cap_type, object_ptr)
            // CapType::IrqControl = 10 (from kernel)
            const IRQ_CONTROL_SLOT: usize = 1;
            const CAP_TYPE_IRQCONTROL: usize = 10;

            let insert_result = crate::sys_cap_insert_into(
                tcb_cap_slot,
                IRQ_CONTROL_SLOT,
                CAP_TYPE_IRQCONTROL,
                self.irq_control_paddr,
            );

            if insert_result == 0 {
                crate::sys_print("[loader] ✓ IRQControl delegated to slot 1\n");
            } else {
                crate::sys_print("[loader] ✗ Failed to delegate IRQControl\n");
            }
        }

        // Convert to SpawnResult with capability information
        Ok(SpawnResult {
            tcb_cap_slot,                   // Slot number for use with syscalls
            tcb_phys: result.tcb_phys,      // Physical address for reference
            vspace_cap: result.pt_phys,     // Page table root
            cspace_cap: result.cspace_phys, // CSpace root
            pid: result.pid,
        })
    }
}

/// Component loading errors
#[derive(Debug, Clone, Copy)]
pub enum ComponentError {
    /// Component not found in registry
    NotFound,
    /// No binary data embedded
    NoBinary,
    /// ELF parsing failed
    InvalidElf,
    /// Resource allocation failed
    OutOfMemory,
    /// Capability granting failed
    CapabilityError,
    /// Feature not yet implemented
    NotImplemented,
}

/// Get the embedded components manifest
///
/// This returns the contents of PROJECT_ROOT/components.toml that was
/// embedded at build time. Developers configure components at the project
/// root for easy discovery.
///
/// # Example
/// ```no_run
/// let manifest = get_components_manifest();
/// // Parse TOML and discover components
/// ```
pub fn get_components_manifest() -> &'static str {
    COMPONENTS_TOML
}

/// Print component manifest information
///
/// Helper for debugging - shows where manifest was loaded from
pub unsafe fn print_manifest_info() {
    crate::sys_print("═══════════════════════════════════════════════════════════\n");
    crate::sys_print("  Component Manifest\n");
    crate::sys_print("═══════════════════════════════════════════════════════════\n");
    crate::sys_print("  Location: PROJECT_ROOT/components.toml\n");
    crate::sys_print("  Components found: ");
    // Count [[component]] occurrences
    let count = COMPONENTS_TOML.matches("[[component]]").count();
    if count == 0 {
        crate::sys_print("0\n");
    } else {
        // Simple number printing (1-9)
        let digit = b'0' + (count as u8);
        let digit_byte = [digit];
        let s = core::str::from_utf8_unchecked(&digit_byte);
        crate::sys_print(s);
        crate::sys_print("\n");
    }
    crate::sys_print("  Status: Embedded at build time\n");
    crate::sys_print("═══════════════════════════════════════════════════════════\n");
    crate::sys_print("\n");
}
