// Utility functions

/// Align address up to the nearest multiple of align
pub const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Align address down to the nearest multiple of align
pub const fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}

/// Check if address is aligned
pub const fn is_aligned(addr: usize, align: usize) -> bool {
    (addr & (align - 1)) == 0
}

/// Page size (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Align to page boundary (up)
pub const fn page_align_up(addr: usize) -> usize {
    align_up(addr, PAGE_SIZE)
}

/// Align to page boundary (down)
pub const fn page_align_down(addr: usize) -> usize {
    align_down(addr, PAGE_SIZE)
}
