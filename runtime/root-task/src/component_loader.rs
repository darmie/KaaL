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
}

impl ComponentLoader {
    /// Create a new component loader
    pub const fn new(registry: &'static ComponentRegistry) -> Self {
        Self { registry }
    }

    /// Spawn a component by name
    ///
    /// Returns the process ID on success
    pub unsafe fn spawn(&self, name: &str) -> Result<usize, ComponentError> {
        let descriptor = self.registry
            .find(name)
            .ok_or(ComponentError::NotFound)?;

        self.spawn_component(descriptor)
    }

    /// Spawn all autostart components
    pub unsafe fn spawn_autostart(&self) -> Result<(), ComponentError> {
        for component in self.registry.autostart_components() {
            match self.spawn_component(component) {
                Ok(pid) => {
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
    unsafe fn spawn_component(&self, desc: &ComponentDescriptor) -> Result<usize, ComponentError> {
        // 1. Get binary data
        let binary_data = desc.binary_data.ok_or(ComponentError::NoBinary)?;

        // 2. Parse ELF (using existing root-task ELF parser)
        // TODO: Integrate with elf::parse_elf_header()

        // 3. Allocate process resources
        // - TCB (thread control block)
        // - Address space
        // - Stack

        // 4. Grant capabilities
        // TODO: Parse and grant capabilities from strings
        // For now, skip capability granting

        // 5. Load binary into address space
        // TODO: Use ELF loader to map segments

        // 6. Start component
        // TODO: Resume TCB

        // For now, return error indicating not yet implemented
        Err(ComponentError::NotImplemented)
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
