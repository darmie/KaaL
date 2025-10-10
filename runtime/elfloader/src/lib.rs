//! KaaL Elfloader - Rust-based bootloader for seL4
//!
//! This bootloader prepares the ARM64 system for running the seL4 microkernel
//! and KaaL root task. It handles:
//! - ARM64 boot initialization
//! - MMU and page table setup
//! - ELF image loading
//! - Device tree processing
//! - Kernel handoff

#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::panic::PanicInfo;
use core::alloc::{GlobalAlloc, Layout};

// Simple bump allocator for bootloader
struct BumpAllocator {
    heap: spin::Mutex<BumpAllocatorInner>,
}

struct BumpAllocatorInner {
    heap_start: usize,
    heap_end: usize,
    next: usize,
}

impl BumpAllocatorInner {
    const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: 0,
        }
    }

    fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut inner = self.heap.lock();

        if inner.heap_start == 0 {
            // Initialize heap on first allocation (8MB heap)
            const HEAP_SIZE: usize = 8 * 1024 * 1024;
            static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
            inner.init(HEAP.as_ptr() as usize, HEAP_SIZE);
        }

        let alloc_start = (inner.next + layout.align() - 1) & !(layout.align() - 1);
        let alloc_end = alloc_start + layout.size();

        if alloc_end > inner.heap_end {
            core::ptr::null_mut()
        } else {
            inner.next = alloc_end;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't support deallocation
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator {
    heap: spin::Mutex::new(BumpAllocatorInner::new()),
};

pub mod arch;
pub mod boot;
pub mod mmu;
pub mod payload;
pub mod uart;
pub mod utils;

/// Boot information passed to the kernel
#[repr(C)]
pub struct BootInfo {
    /// Physical address of user image start
    pub user_img_start: usize,
    /// Physical address of user image end
    pub user_img_end: usize,
    /// Physical-to-virtual offset
    pub pv_offset: usize,
    /// User image entry point
    pub user_entry: usize,
    /// Device tree physical address
    pub dtb_addr: usize,
    /// Device tree size
    pub dtb_size: usize,
}

/// Kernel entry function type
type KernelEntry = extern "C" fn(usize, usize, usize, usize, usize, usize) -> !;

/// Main elfloader entry point (called from assembly)
#[no_mangle]
pub extern "C" fn elfloader_main(dtb_addr: usize) -> ! {
    // Initialize UART for debug output
    uart::init();
    uart::println!("═══════════════════════════════════════════════════════════");
    uart::println!("  KaaL Elfloader v0.1.0 - Rust-based seL4 Boot Loader");
    uart::println!("═══════════════════════════════════════════════════════════");
    uart::println!();
    uart::println!("DTB address: {:#x}", dtb_addr);

    // Parse device tree
    let dtb = unsafe { fdt::Fdt::from_ptr(dtb_addr as *const u8) }
        .expect("Failed to parse device tree");

    uart::println!("Device tree parsed successfully");
    uart::println!("Model: {}", dtb.root().model());

    // Get memory info from device tree
    let memory = dtb.memory();
    for region in memory.regions() {
        let start = region.starting_address as usize;
        let size = region.size.unwrap_or(0);
        uart::println!("Memory region: {:#x} - {:#x} ({} MB)",
            start,
            start + size,
            size / (1024 * 1024));
    }

    uart::println!();
    uart::println!("Loading images...");

    // Load kernel and user images
    let (kernel_entry, mut boot_info) = boot::load_images();

    // Set DTB info in boot_info
    boot_info.dtb_addr = dtb_addr;
    boot_info.dtb_size = dtb.total_size();

    // Update rootserver structure with DTB information
    boot::update_rootserver_dtb(kernel_entry, dtb_addr, dtb.total_size());

    uart::println!("Kernel entry: {:#x}", kernel_entry);
    uart::println!("User image: {:#x} - {:#x}",
        boot_info.user_img_start, boot_info.user_img_end);
    uart::println!("User entry: {:#x}", boot_info.user_entry);

    uart::println!();
    uart::println!("Setting up page tables...");

    // Set up page tables for kernel
    let mut pt_mgr = mmu::PageTableManager::new();

    // Identity map elfloader memory
    extern "C" {
        static __elfloader_end: u8;
    }
    let elfloader_end = unsafe { &__elfloader_end as *const u8 as usize };
    pt_mgr.setup_identity_map(0x10000000, elfloader_end);

    uart::println!("Page tables configured");
    uart::println!("TTBR0: {:#x}", pt_mgr.get_ttbr0());

    uart::println!();
    uart::println!("Enabling MMU...");

    // Enable MMU
    arch::enable_mmu(
        pt_mgr.get_ttbr0(),
        0, // TTBR1 not used
        mmu::PageTableManager::get_mair(),
        mmu::PageTableManager::get_tcr(),
    );

    uart::println!("MMU enabled successfully");
    uart::println!();
    uart::println!("Jumping to seL4 kernel at {:#x}...", kernel_entry);
    uart::println!("  Passing root task info:");
    uart::println!("    user_img: {:#x} - {:#x}", boot_info.user_img_start, boot_info.user_img_end);
    uart::println!("    user_entry: {:#x}", boot_info.user_entry);
    uart::println!("    pv_offset: {:#x}", boot_info.pv_offset);
    uart::println!("    dtb: {:#x} (size: {})", boot_info.dtb_addr, boot_info.dtb_size);
    uart::println!("═══════════════════════════════════════════════════════════");
    uart::println!();

    // Jump to kernel with root task boot info
    let kernel_fn: KernelEntry = unsafe { core::mem::transmute(kernel_entry) };
    kernel_fn(
        boot_info.user_img_start,   // x0: user physical start
        boot_info.user_img_end,     // x1: user physical end
        boot_info.pv_offset,        // x2: physical-virtual offset
        boot_info.user_entry,       // x3: user entry point
        boot_info.dtb_addr,         // x4: DTB address
        boot_info.dtb_size,         // x5: DTB size
    )
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    uart::println!("PANIC: {}", info);
    loop {
        core::hint::spin_loop();
    }
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}
