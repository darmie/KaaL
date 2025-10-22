//! ARM Generic Interrupt Controller (GICv2) Driver
//!
//! The GIC manages hardware interrupts for ARM systems. It consists of two main components:
//!
//! ## GIC Distributor (GICD)
//! - Base address: 0x08000000 (QEMU virt)
//! - Manages interrupt prioritization, routing, and distribution to CPU interfaces
//! - Handles up to 1020 interrupts (32 SGI/PPI + 988 SPI)
//!
//! ## GIC CPU Interface (GICC)
//! - Base address: 0x08010000 (QEMU virt)
//! - Per-CPU interface for acknowledging and completing interrupts
//! - Provides interrupt masking and priority filtering
//!
//! ## Interrupt Types
//! - **SGI (0-15)**: Software Generated Interrupts (inter-processor interrupts)
//! - **PPI (16-31)**: Private Peripheral Interrupts (per-CPU, e.g., local timer)
//! - **SPI (32-1019)**: Shared Peripheral Interrupts (hardware devices)
//!
//! ## Platform-Specific IRQ Mapping
//! IRQ numbers are defined in build-config.toml and generated at build time.
//! See `kernel/src/generated/memory_config.rs` for actual values.

use core::ptr::{read_volatile, write_volatile};
use crate::generated::memory_config::{GIC_DIST_BASE, GIC_CPU_BASE};

/// GIC Distributor base address (from platform configuration)
const GICD_BASE: usize = GIC_DIST_BASE;

/// GIC CPU Interface base address (from platform configuration)
const GICC_BASE: usize = GIC_CPU_BASE;

/// Maximum number of interrupts supported (32 SGI/PPI + 988 SPI)
pub const MAX_IRQS: usize = 1020;

// =============================================================================
// GIC Distributor Registers (GICD_*)
// =============================================================================

/// GICD_CTLR - Distributor Control Register
/// Bit 0: Enable Group 0 interrupts
/// Bit 1: Enable Group 1 interrupts
const GICD_CTLR: usize = GICD_BASE + 0x000;

/// GICD_TYPER - Interrupt Controller Type Register
/// Bits 0-4: Number of implemented ITLinesNumber (N)
///           Total SPIs = 32 * (N + 1)
const GICD_TYPER: usize = GICD_BASE + 0x004;

/// GICD_ISENABLERn - Interrupt Set-Enable Registers
/// Each bit enables one interrupt (write 1 to enable)
/// Register n controls interrupts [32n : 32n+31]
const GICD_ISENABLER: usize = GICD_BASE + 0x100;

/// GICD_ICENABLERn - Interrupt Clear-Enable Registers
/// Each bit disables one interrupt (write 1 to disable)
const GICD_ICENABLER: usize = GICD_BASE + 0x180;

/// GICD_ISPENDRn - Interrupt Set-Pending Registers
/// Each bit sets interrupt to pending state (for testing)
const GICD_ISPENDR: usize = GICD_BASE + 0x200;

/// GICD_ICPENDRn - Interrupt Clear-Pending Registers
/// Each bit clears pending state
const GICD_ICPENDR: usize = GICD_BASE + 0x280;

/// GICD_IPRIORITYRn - Interrupt Priority Registers
/// 8 bits per interrupt (0 = highest priority, 255 = lowest)
const GICD_IPRIORITYR: usize = GICD_BASE + 0x400;

/// GICD_ITARGETSRn - Interrupt Processor Targets Registers
/// 8 bits per interrupt, each bit represents a CPU core
/// Bit 0 = CPU0, Bit 1 = CPU1, etc.
const GICD_ITARGETSR: usize = GICD_BASE + 0x800;

/// GICD_ICFGRn - Interrupt Configuration Registers
/// 2 bits per interrupt:
/// - Bit 0: Reserved (should be 0)
/// - Bit 1: 0 = level-sensitive, 1 = edge-triggered
const GICD_ICFGR: usize = GICD_BASE + 0xC00;

// =============================================================================
// GIC CPU Interface Registers (GICC_*)
// =============================================================================

/// GICC_CTLR - CPU Interface Control Register
/// Bit 0: Enable Group 0 interrupts
/// Bit 1: Enable Group 1 interrupts
/// Bit 9: EOImode (0 = priority drop and deactivate, 1 = priority drop only)
const GICC_CTLR: usize = GICC_BASE + 0x000;

/// GICC_PMR - Interrupt Priority Mask Register
/// Interrupts with priority lower than this value are masked
/// 0 = all masked, 255 = all enabled
const GICC_PMR: usize = GICC_BASE + 0x004;

/// GICC_BPR - Binary Point Register
/// Controls priority grouping for preemption
const GICC_BPR: usize = GICC_BASE + 0x008;

/// GICC_IAR - Interrupt Acknowledge Register
/// Read to acknowledge an interrupt and get its ID
/// Bits 0-9: Interrupt ID
/// Bits 10-12: CPU ID (for SGIs)
const GICC_IAR: usize = GICC_BASE + 0x00C;

/// GICC_EOIR - End of Interrupt Register
/// Write interrupt ID here to signal completion
const GICC_EOIR: usize = GICC_BASE + 0x010;

/// GICC_RPR - Running Priority Register (read-only)
/// Shows current running priority
const GICC_RPR: usize = GICC_BASE + 0x014;

/// GICC_HPPIR - Highest Priority Pending Interrupt Register (read-only)
/// Shows ID of highest priority pending interrupt
const GICC_HPPIR: usize = GICC_BASE + 0x018;

/// Special interrupt ID returned when no interrupt is pending
const SPURIOUS_IRQ: u32 = 1023;

// =============================================================================
// GIC Driver Implementation
// =============================================================================

/// Initialize the GIC
///
/// This function must be called during kernel boot to:
/// 1. Discover the number of supported interrupts
/// 2. Disable all interrupts
/// 3. Configure default priorities and targets
/// 4. Enable the GIC distributor and CPU interface
pub unsafe fn init() {
    crate::kprintln!("[GIC] Initializing GICv2...");

    // Read GIC type to discover number of interrupt lines
    let typer = read_volatile(GICD_TYPER as *const u32);
    let itlines = (typer & 0x1F) as usize; // Bits 0-4
    let max_irqs = 32 * (itlines + 1);
    crate::kprintln!("[GIC] ITLinesNumber: {}, Max IRQs: {}", itlines, max_irqs);

    // Disable distributor while configuring
    write_volatile(GICD_CTLR as *mut u32, 0);

    // Disable all interrupts
    for i in 0..((max_irqs + 31) / 32) {
        write_volatile((GICD_ICENABLER + i * 4) as *mut u32, 0xFFFFFFFF);
    }

    // Clear all pending interrupts
    for i in 0..((max_irqs + 31) / 32) {
        write_volatile((GICD_ICPENDR + i * 4) as *mut u32, 0xFFFFFFFF);
    }

    // Set default priority (0xA0 = 160) for all interrupts
    // Priority range: 0 (highest) - 255 (lowest)
    for i in 0..(max_irqs / 4) {
        write_volatile((GICD_IPRIORITYR + i * 4) as *mut u32, 0xA0A0A0A0);
    }

    // Route all SPIs to CPU 0
    // First 32 interrupts (SGI/PPI) are banked per-CPU, so start from 32
    for i in (32 / 4)..(max_irqs / 4) {
        write_volatile((GICD_ITARGETSR + i * 4) as *mut u32, 0x01010101); // Target CPU 0
    }

    // Configure all SPIs as level-sensitive (default)
    // Bits [1:0] are read-only (SGI), start from register 1
    for i in 1..((max_irqs + 15) / 16) {
        write_volatile((GICD_ICFGR + i * 4) as *mut u32, 0x00000000);
    }

    // Enable distributor (Group 0 interrupts)
    write_volatile(GICD_CTLR as *mut u32, 0x1);

    // Initialize CPU interface
    init_cpu_interface();

    crate::kprintln!("[GIC] GICv2 initialized successfully");
}

/// Initialize the GIC CPU interface for the current CPU
///
/// This must be called on each CPU core to enable interrupt delivery.
/// For now, we only support single-core (CPU 0).
unsafe fn init_cpu_interface() {
    // Disable CPU interface while configuring
    write_volatile(GICC_CTLR as *mut u32, 0);

    // Set priority mask to allow all interrupts (255 = lowest priority)
    write_volatile(GICC_PMR as *mut u32, 0xFF);

    // Set binary point to 0 (no grouping, all 8 bits for priority)
    write_volatile(GICC_BPR as *mut u32, 0);

    // Enable CPU interface (Group 0 interrupts)
    write_volatile(GICC_CTLR as *mut u32, 0x1);

    crate::kprintln!("[GIC] CPU interface initialized");
}

/// Enable a specific interrupt
///
/// # Arguments
/// * `irq` - Interrupt number (0-1019)
///
/// # Safety
/// Must be called with a valid IRQ number
pub unsafe fn enable_irq(irq: u32) {
    if irq >= MAX_IRQS as u32 {
        crate::kprintln!("[GIC] ERROR: Invalid IRQ {}", irq);
        return;
    }

    let reg = (irq / 32) as usize;
    let bit = irq % 32;

    // Write 1 to the corresponding bit to enable
    write_volatile(
        (GICD_ISENABLER + reg * 4) as *mut u32,
        1 << bit,
    );

    crate::kprintln!("[GIC] Enabled IRQ {}", irq);
}

/// Disable a specific interrupt
///
/// # Arguments
/// * `irq` - Interrupt number (0-1019)
///
/// # Safety
/// Must be called with a valid IRQ number
pub unsafe fn disable_irq(irq: u32) {
    if irq >= MAX_IRQS as u32 {
        return;
    }

    let reg = (irq / 32) as usize;
    let bit = irq % 32;

    // Write 1 to the corresponding bit to disable
    write_volatile(
        (GICD_ICENABLER + reg * 4) as *mut u32,
        1 << bit,
    );

    crate::kprintln!("[GIC] Disabled IRQ {}", irq);
}

/// Set interrupt priority
///
/// # Arguments
/// * `irq` - Interrupt number
/// * `priority` - Priority value (0 = highest, 255 = lowest)
///
/// # Safety
/// Must be called with a valid IRQ number
pub unsafe fn set_priority(irq: u32, priority: u8) {
    if irq >= MAX_IRQS as u32 {
        return;
    }

    let reg = (irq / 4) as usize;
    let offset = (irq % 4) * 8;

    // Read-modify-write to set priority for this IRQ
    let addr = (GICD_IPRIORITYR + reg * 4) as *mut u32;
    let mut val = read_volatile(addr);
    val &= !(0xFF << offset);
    val |= (priority as u32) << offset;
    write_volatile(addr, val);
}

/// Acknowledge an interrupt and return its ID
///
/// This function must be called at the start of the IRQ handler.
/// Returns the interrupt ID, or None if spurious.
///
/// # Safety
/// Must be called from IRQ context
pub unsafe fn acknowledge_irq() -> Option<u32> {
    let iar = read_volatile(GICC_IAR as *const u32);
    let irq_id = iar & 0x3FF; // Bits 0-9

    if irq_id == SPURIOUS_IRQ {
        None
    } else {
        Some(irq_id)
    }
}

/// Signal end of interrupt processing
///
/// This function must be called at the end of the IRQ handler to:
/// 1. Drop the interrupt priority
/// 2. Deactivate the interrupt (allow it to be triggered again)
///
/// # Arguments
/// * `irq` - The interrupt ID returned by `acknowledge_irq()`
///
/// # Safety
/// Must be called from IRQ context with the correct IRQ ID
pub unsafe fn end_of_interrupt(irq: u32) {
    write_volatile(GICC_EOIR as *mut u32, irq);
}

/// Get the highest priority pending interrupt (without acknowledging)
///
/// # Safety
/// Safe to call from any context
pub unsafe fn get_highest_pending() -> Option<u32> {
    let hppir = read_volatile(GICC_HPPIR as *const u32);
    let irq_id = hppir & 0x3FF;

    if irq_id == SPURIOUS_IRQ {
        None
    } else {
        Some(irq_id)
    }
}
