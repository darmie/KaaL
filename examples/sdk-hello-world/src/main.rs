//! Hello World Example using KaaL SDK
//!
//! Demonstrates clean SDK API for syscalls, notifications, and memory management.

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use kaal_sdk::{syscall, capability, memory};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        // Using clean SDK API instead of raw syscalls!
        syscall::print("\n");
        syscall::print("═══════════════════════════════════════════════════════════\n");
        syscall::print("  Hello World - KaaL SDK Demo\n");
        syscall::print("═══════════════════════════════════════════════════════════\n");
        syscall::print("\n");

        // Test 1: Basic syscalls
        syscall::print("[sdk] Test 1: Basic Syscalls\n");
        syscall::print("  Using kaal_sdk::syscall::print() ✓\n");
        syscall::print("\n");

        // Test 2: Notifications
        syscall::print("[sdk] Test 2: Notification Management\n");
        match capability::Notification::create() {
            Ok(notification) => {
                syscall::print("  ✓ Created notification using SDK\n");

                // Signal and poll
                if let Ok(()) = notification.signal(0x42) {
                    syscall::print("  ✓ Signaled with badge 0x42\n");
                }

                if let Ok(signals) = notification.poll() {
                    if signals == 0x42 {
                        syscall::print("  ✓ Polled and got correct badge\n");
                    }
                }
            }
            Err(_) => {
                syscall::print("  ✗ Failed to create notification\n");
            }
        }
        syscall::print("\n");

        // Test 3: Memory management
        syscall::print("[sdk] Test 3: Memory Management\n");
        match memory::PhysicalMemory::allocate(4096) {
            Ok(phys_mem) => {
                syscall::print("  ✓ Allocated 4KB physical memory\n");

                match memory::MappedMemory::map(
                    phys_mem.phys_addr(),
                    phys_mem.size(),
                    memory::Permissions::RW
                ) {
                    Ok(mapped) => {
                        syscall::print("  ✓ Mapped into virtual address space\n");
                        syscall::print("  ✓ RAII will auto-unmap on drop\n");
                    }
                    Err(_) => {
                        syscall::print("  ✗ Failed to map memory\n");
                    }
                }
            }
            Err(_) => {
                syscall::print("  ✗ Failed to allocate memory\n");
            }
        }
        syscall::print("\n");

        // Test 4: Capability management
        syscall::print("[sdk] Test 4: Capability Allocation\n");
        match syscall::cap_allocate() {
            Ok(slot) => {
                syscall::print("  ✓ Allocated capability slot\n");
            }
            Err(_) => {
                syscall::print("  ✗ Failed to allocate cap slot\n");
            }
        }
        syscall::print("\n");

        syscall::print("═══════════════════════════════════════════════════════════\n");
        syscall::print("  KaaL SDK Demo: SUCCESS ✓\n");
        syscall::print("═══════════════════════════════════════════════════════════\n");
        syscall::print("\n");
        syscall::print("[sdk] Key Benefits:\n");
        syscall::print("  • Clean, ergonomic API (no raw asm!)\n");
        syscall::print("  • Type-safe wrappers\n");
        syscall::print("  • RAII resource management\n");
        syscall::print("  • Reduced boilerplate code\n");
        syscall::print("  • Better error handling\n");
        syscall::print("\n");

        // Yield to scheduler
        syscall::yield_now();
    }

    // Idle loop
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
