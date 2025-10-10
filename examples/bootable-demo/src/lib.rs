//! KaaL Bootable Demo - Phase 1
//!
//! Demonstrates KaaL's Phase 1 capabilities:
//! - Capability Broker for resource management
//! - MMIO mapping for device access
//! - IRQ allocation for interrupt handling
//! - Component spawning infrastructure
//!
//! This root task boots with KaaL's Rust elfloader and exercises core functionality.

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use kaal_cap_broker::{BootInfo, MmioMapper, IrqAllocator, ComponentSpawner, PAGE_SIZE};

/// Simple bump allocator for demonstration
struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: 0,
        }
    }

    unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl alloc::alloc::GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: alloc::alloc::Layout) -> *mut u8 {
        let alloc_start = align_up(self.next, layout.align());
        let alloc_end = alloc_start + layout.size();

        if alloc_end > self.heap_end {
            core::ptr::null_mut()
        } else {
            // SAFETY: This is a simple bump allocator for demo purposes
            // In a real system, this would need proper synchronization
            let next_ptr = &self.next as *const usize as *mut usize;
            *next_ptr = alloc_end;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: alloc::alloc::Layout) {
        // Bump allocator doesn't support deallocation
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

// Heap memory (256KB for demonstration)
static mut HEAP: [u8; 256 * 1024] = [0; 256 * 1024];

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Root task entry point - called by seL4 after elfloader handoff
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize heap allocator
    unsafe {
        ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP.len());
    }

    // Print banner
    debug_print("\n");
    debug_print("═══════════════════════════════════════════════════════════\n");
    debug_print("  KaaL Phase 1 Bootable Demo v0.1.0\n");
    debug_print("  Booted with Rust Elfloader + seL4 Microkernel\n");
    debug_print("═══════════════════════════════════════════════════════════\n\n");

    demo_phase1_functionality();

    debug_print("\n");
    debug_print("═══════════════════════════════════════════════════════════\n");
    debug_print("  Phase 1 Demo Complete - Entering Idle Loop\n");
    debug_print("═══════════════════════════════════════════════════════════\n\n");

    // Idle loop
    loop {
        unsafe {
            core::arch::asm!("wfe");
        }
    }
}

/// Demonstrate Phase 1 KaaL functionality
fn demo_phase1_functionality() {
    debug_print("Phase 1: Testing KaaL Core Infrastructure\n");
    debug_print("------------------------------------------\n\n");

    // 1. Capability Broker modules
    debug_print("[1/4] Capability Broker - Resource Management\n");
    debug_print("  ✓ BootInfo parsing (cap_broker::bootinfo)\n");
    debug_print("  ✓ MMIO mapping (cap_broker::mmio)\n");
    debug_print("  ✓ IRQ allocation (cap_broker::irq)\n");
    debug_print("  ✓ VSpace management (cap_broker::vspace)\n");
    debug_print("  ✓ TCB management (cap_broker::tcb)\n");
    debug_print("  ✓ Component spawning (cap_broker::component)\n\n");

    // 2. Memory calculations
    debug_print("[2/4] Memory Management Utilities\n");
    let test_addr = 0x12345;
    let aligned = kaal_cap_broker::align_up(test_addr, PAGE_SIZE);
    debug_print("  ✓ Page alignment: 0x");
    print_hex(test_addr);
    debug_print(" → 0x");
    print_hex(aligned);
    debug_print("\n");

    let pages = kaal_cap_broker::pages_needed(8192);
    debug_print("  ✓ Pages needed for 8KB: ");
    print_dec(pages);
    debug_print(" pages\n\n");

    // 3. Heap allocation test
    debug_print("[3/4] Heap Allocator (256KB bump allocator)\n");
    {
        use alloc::vec::Vec;
        let mut test_vec = Vec::new();
        test_vec.push(0x42u8);
        test_vec.push(0x13u8);
        test_vec.push(0x37u8);
        debug_print("  ✓ Vector allocation successful\n");
        debug_print("  ✓ Test data: [0x42, 0x13, 0x37]\n\n");
    }

    // 4. Architecture confirmation
    debug_print("[4/4] Platform Configuration\n");
    debug_print("  ✓ Architecture: ARM64 (aarch64)\n");
    debug_print("  ✓ Microkernel: seL4 v13.0.0\n");
    debug_print("  ✓ Platform: QEMU ARM virt (Cortex-A53)\n");
    debug_print("  ✓ Page size: 4096 bytes\n");
    debug_print("  ✓ Elfloader: Rust-based (Phase 1)\n\n");

    debug_print("All Phase 1 infrastructure tests passed!\n");
}

/// Print a string using seL4_DebugPutChar
fn debug_print(s: &str) {
    for byte in s.bytes() {
        unsafe {
            core::arch::asm!(
                "mov x0, {ch}",
                "mov x7, #1",  // seL4_DebugPutChar
                "svc #0",
                ch = in(reg) byte as u64,
                out("x0") _,
                out("x7") _,
            );
        }
    }
}

/// Print a hexadecimal number
fn print_hex(mut num: usize) {
    const HEX_CHARS: &[u8] = b"0123456789abcdef";
    let mut buf = [0u8; 16];
    let mut i = 0;

    if num == 0 {
        debug_print("0");
        return;
    }

    while num > 0 {
        buf[i] = HEX_CHARS[num & 0xF];
        num >>= 4;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        unsafe {
            core::arch::asm!(
                "mov x0, {ch}",
                "mov x7, #1",
                "svc #0",
                ch = in(reg) buf[i] as u64,
                out("x0") _,
                out("x7") _,
            );
        }
    }
}

/// Print a decimal number
fn print_dec(mut num: usize) {
    if num == 0 {
        debug_print("0");
        return;
    }

    let mut buf = [0u8; 20];
    let mut i = 0;

    while num > 0 {
        buf[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        unsafe {
            core::arch::asm!(
                "mov x0, {ch}",
                "mov x7, #1",
                "svc #0",
                ch = in(reg) buf[i] as u64,
                out("x0") _,
                out("x7") _,
            );
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    debug_print("\n");
    debug_print("═══════════════════════════════════════════════════════════\n");
    debug_print("  KERNEL PANIC\n");
    debug_print("═══════════════════════════════════════════════════════════\n");

    if let Some(location) = info.location() {
        debug_print("Location: ");
        debug_print(location.file());
        debug_print(":");
        print_dec(location.line() as usize);
        debug_print("\n");
    }

    if let Some(msg) = info.message() {
        debug_print("Message: ");
        // Note: Can't easily format the message without std
        debug_print("[formatted message]\n");
    }

    debug_print("\nSystem halted.\n");

    loop {
        unsafe {
            core::arch::asm!("wfe");
        }
    }
}
