//! ARM64 MMU (Memory Management Unit) setup and control
//!
//! This module handles:
//! - MMU initialization and configuration
//! - TTBR0/TTBR1 setup (translation table base registers)
//! - TCR_EL1 configuration (translation control register)
//! - SCTLR_EL1 configuration (system control register)
//! - MAIR_EL1 setup (memory attribute indirection register)
//!
//! # Design
//! - TTBR0_EL1: User space page tables (0x0000_xxxx_xxxx_xxxx)
//! - TTBR1_EL1: Kernel space page tables (0xFFFF_xxxx_xxxx_xxxx)
//! - 4KB granule, 48-bit address space

use core::arch::asm;
use crate::memory::PhysAddr;

/// Memory Attribute Indirection Register (MAIR_EL1) configuration
///
/// Defines memory types for use in page table entries (AttrIndx field)
#[repr(u64)]
pub enum MemoryAttribute {
    /// Normal memory, inner/outer write-back cacheable
    Normal = 0xFF,
    /// Device memory, non-gathering, non-reordering, non-early write acknowledgement
    Device = 0x00,
}

/// Translation Control Register (TCR_EL1) flags
pub struct TcrFlags {
    /// T0SZ: Size offset for TTBR0 (64 - 48 = 16 for 48-bit)
    pub t0sz: u64,
    /// T1SZ: Size offset for TTBR1 (64 - 48 = 16 for 48-bit)
    pub t1sz: u64,
    /// TG0: Granule size for TTBR0 (0 = 4KB)
    pub tg0: u64,
    /// TG1: Granule size for TTBR1 (2 = 4KB)
    pub tg1: u64,
    /// IPS: Intermediate physical address size (0 = 32-bit, 5 = 48-bit)
    pub ips: u64,
}

impl TcrFlags {
    /// Create default TCR flags for 48-bit VA, 4KB granule
    pub const fn default() -> Self {
        Self {
            t0sz: 16,  // 48-bit VA for TTBR0
            t1sz: 16,  // 48-bit VA for TTBR1
            tg0: 0,    // 4KB granule for TTBR0
            tg1: 2,    // 4KB granule for TTBR1 (value 2)
            ips: 5,    // 48-bit IPA
        }
    }

    /// Convert to TCR_EL1 register value
    pub const fn to_register(&self) -> u64 {
        (self.t0sz << 0)       // T0SZ [5:0]
            | (self.t1sz << 16)    // T1SZ [21:16]
            | (self.tg0 << 14)     // TG0 [15:14]
            | (self.tg1 << 30)     // TG1 [31:30]
            | (self.ips << 32)     // IPS [34:32]
            | (1 << 8)             // IRGN0 = 1 (inner write-back cacheable)
            | (1 << 10)            // ORGN0 = 1 (outer write-back cacheable)
            | (3 << 12)            // SH0 = 3 (inner shareable)
            | (1 << 24)            // IRGN1 = 1 (inner write-back cacheable)
            | (1 << 26)            // ORGN1 = 1 (outer write-back cacheable)
            | (3 << 28)            // SH1 = 3 (inner shareable)
    }
}

/// MMU configuration
pub struct MmuConfig {
    /// Physical address of TTBR0 page table (user space)
    pub ttbr0: Option<PhysAddr>,
    /// Physical address of TTBR1 page table (kernel space)
    pub ttbr1: PhysAddr,
}

/// Initialize the MMU with the given configuration
///
/// # Safety
/// - Must be called with valid page table addresses
/// - Page tables must be properly initialized
/// - Must be called from EL1
pub unsafe fn init_mmu(config: MmuConfig) {
    // Setup MAIR_EL1 (Memory Attribute Indirection Register)
    let mair_value =
        (MemoryAttribute::Normal as u64) << 0 |   // Attr0: Normal memory
        (MemoryAttribute::Device as u64) << 8;    // Attr1: Device memory

    asm!(
        "msr mair_el1, {mair}",
        mair = in(reg) mair_value,
        options(nomem, nostack),
    );

    // Setup TCR_EL1 (Translation Control Register)
    let tcr = TcrFlags::default();
    asm!(
        "msr tcr_el1, {tcr}",
        tcr = in(reg) tcr.to_register(),
        options(nomem, nostack),
    );

    // Setup TTBR1_EL1 (kernel space page table)
    asm!(
        "msr ttbr1_el1, {ttbr1}",
        ttbr1 = in(reg) config.ttbr1.as_usize(),
        options(nomem, nostack),
    );

    // Setup TTBR0_EL1 (user space page table) if provided
    if let Some(ttbr0) = config.ttbr0 {
        asm!(
            "msr ttbr0_el1, {ttbr0}",
            ttbr0 = in(reg) ttbr0.as_usize(),
            options(nomem, nostack),
        );
    }

    // Ensure all page table writes are visible before MMU enable
    // This is CRITICAL - page tables must be in memory, not stuck in caches
    asm!(
        "dsb sy",                  // Full system data sync barrier
        options(nomem, nostack),
    );

    // Invalidate TLB (crucial before MMU enable!)
    // ARM TF does this: "Ensure translation table writes have drained"
    asm!(
        "tlbi vmalle1",           // Invalidate all TLB entries for EL1
        "dsb sy",                 // Full system data sync barrier
        "isb",                    // Instruction sync barrier
        options(nomem, nostack),
    );

    // Enable MMU ONLY (following seL4 pattern: enable MMU first, caches later)
    let mut sctlr: u64;
    asm!(
        "mrs {sctlr}, sctlr_el1",
        sctlr = out(reg) sctlr,
        options(nomem, nostack),
    );

    // First, ensure caches are disabled (seL4 requirement)
    sctlr &= !(1 << 2);   // Clear C bit (data cache)
    sctlr &= !(1 << 12);  // Clear I bit (instruction cache)

    // Now enable ONLY the MMU
    sctlr |= (1 << 0);    // Set M bit (MMU enable)

    asm!(
        "msr sctlr_el1, {sctlr}",
        "dsb sy",                 // Full system barrier
        "isb",                    // Synchronize instruction fetch
        sctlr = in(reg) sctlr,
        options(nomem, nostack),
    );

    // TODO: Enable caches after MMU is verified working
    // sctlr |= (1 << 2) | (1 << 12);  // Enable D-cache and I-cache
}

/// Check if MMU is currently enabled
pub fn is_mmu_enabled() -> bool {
    let sctlr: u64;
    unsafe {
        asm!(
            "mrs {sctlr}, sctlr_el1",
            sctlr = out(reg) sctlr,
            options(nomem, nostack),
        );
    }
    (sctlr & 1) != 0
}

/// Get current TTBR0_EL1 value
pub fn get_ttbr0() -> u64 {
    let ttbr0: u64;
    unsafe {
        asm!(
            "mrs {ttbr0}, ttbr0_el1",
            ttbr0 = out(reg) ttbr0,
            options(nomem, nostack),
        );
    }
    ttbr0
}

/// Get current TTBR1_EL1 value
pub fn get_ttbr1() -> u64 {
    let ttbr1: u64;
    unsafe {
        asm!(
            "mrs {ttbr1}, ttbr1_el1",
            ttbr1 = out(reg) ttbr1,
            options(nomem, nostack),
        );
    }
    ttbr1
}

/// Invalidate TLB entry for a virtual address
///
/// # Safety
/// - Should be called after changing page table entries
pub unsafe fn invalidate_tlb(vaddr: usize) {
    asm!(
        "dsb ishst",         // Ensure writes complete
        "tlbi vaae1is, {va}", // Invalidate by VA, all ASID, inner shareable
        "dsb ish",           // Ensure TLB invalidation completes
        "isb",               // Synchronize context
        va = in(reg) vaddr >> 12, // VA needs to be shifted right by 12
        options(nomem, nostack),
    );
}

/// Invalidate entire TLB
///
/// # Safety
/// - Should be called after major page table changes
pub unsafe fn invalidate_tlb_all() {
    asm!(
        "dsb ishst",
        "tlbi vmalle1is",    // Invalidate all, EL1, inner shareable
        "dsb ish",
        "isb",
        options(nomem, nostack),
    );
}
