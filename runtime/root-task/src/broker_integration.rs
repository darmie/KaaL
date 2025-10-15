//! Capability Broker Integration for Root Task
//!
//! This module provides integration between the root task and the capability broker,
//! demonstrating how to use the broker's clean API instead of raw syscalls.

use capability_broker::{CapabilityBroker, DeviceId};

/// Print helper for integration messages
unsafe fn sys_print(msg: &str) {
    let msg_ptr = msg.as_ptr() as usize;
    let msg_len = msg.len();

    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {msg_ptr}",
        "mov x1, {msg_len}",
        "svc #0",
        syscall_num = in(reg) 0x1001u64, // SYS_DEBUG_PRINT
        msg_ptr = in(reg) msg_ptr,
        msg_len = in(reg) msg_len,
        out("x0") _,
        out("x1") _,
        out("x8") _,
    );
}

/// Print a number in decimal
unsafe fn print_number(n: usize) {
    let mut buf = [0u8; 20];
    let mut num = n;
    let mut i = 0;

    if num == 0 {
        sys_print("0");
        return;
    }

    while num > 0 {
        buf[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        let digit = core::str::from_utf8_unchecked(&buf[i..i+1]);
        sys_print(digit);
    }
}

/// Print a number in hexadecimal
unsafe fn print_hex(n: usize) {
    let hex_chars = b"0123456789abcdef";
    let mut buf = [0u8; 16];

    for i in 0..16 {
        let shift = (15 - i) * 4;
        let nibble = ((n >> shift) & 0xf) as usize;
        buf[i] = hex_chars[nibble];
    }

    let hex_str = core::str::from_utf8_unchecked(&buf);
    sys_print(hex_str);
}

/// Test Capability Broker functionality
pub unsafe fn test_capability_broker() {
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("  Chapter 9 Phase 1: Testing Capability Broker API\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("\n");

    // Initialize the Capability Broker
    sys_print("[root_task] Initializing Capability Broker...\n");
    let mut broker = match CapabilityBroker::init() {
        Ok(b) => {
            sys_print("  ✓ Capability Broker initialized\n");
            b
        }
        Err(_) => {
            sys_print("  ✗ Failed to initialize Capability Broker\n");
            return;
        }
    };

    // Test 1: Allocate memory through broker
    sys_print("\n[root_task] Test 1: Allocating memory via broker...\n");
    match broker.allocate_memory(4096) {
        Ok(mem) => {
            sys_print("  ✓ Allocated 4096 bytes at: 0x");
            print_hex(mem.phys_addr);
            sys_print("\n");
            sys_print("    Cap slot: ");
            print_number(mem.cap_slot);
            sys_print("\n");
        }
        Err(_) => {
            sys_print("  ✗ Memory allocation failed\n");
        }
    }

    // Test 2: Request device through broker
    sys_print("\n[root_task] Test 2: Requesting UART0 device via broker...\n");
    match broker.request_device(DeviceId::Uart(0)) {
        Ok(dev) => {
            sys_print("  ✓ UART0 device allocated:\n");
            sys_print("    MMIO base: 0x");
            print_hex(dev.mmio_base);
            sys_print("\n");
            sys_print("    MMIO size: ");
            print_number(dev.mmio_size);
            sys_print(" bytes\n");
            if let Some(irq_cap) = dev.irq_cap {
                sys_print("    IRQ cap: ");
                print_number(irq_cap);
                sys_print("\n");
            }
        }
        Err(_) => {
            sys_print("  ✗ Device request failed\n");
        }
    }

    // Test 3: Create IPC endpoint through broker
    sys_print("\n[root_task] Test 3: Creating IPC endpoint via broker...\n");
    match broker.create_endpoint() {
        Ok(endpoint) => {
            sys_print("  ✓ IPC endpoint created:\n");
            sys_print("    Cap slot: ");
            print_number(endpoint.cap_slot);
            sys_print("\n");
            sys_print("    Endpoint ID: ");
            print_number(endpoint.id);
            sys_print("\n");
        }
        Err(_) => {
            sys_print("  ✗ Endpoint creation failed\n");
        }
    }

    // Test 4: Request multiple devices
    sys_print("\n[root_task] Test 4: Requesting multiple devices...\n");

    sys_print("  → Requesting RTC...\n");
    match broker.request_device(DeviceId::Rtc) {
        Ok(dev) => {
            sys_print("    ✓ RTC MMIO: 0x");
            print_hex(dev.mmio_base);
            sys_print("\n");
        }
        Err(_) => {
            sys_print("    ✗ RTC request failed\n");
        }
    }

    sys_print("  → Requesting Timer...\n");
    match broker.request_device(DeviceId::Timer) {
        Ok(dev) => {
            sys_print("    ✓ Timer MMIO: 0x");
            print_hex(dev.mmio_base);
            sys_print("\n");
        }
        Err(_) => {
            sys_print("    ✗ Timer request failed\n");
        }
    }

    sys_print("\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("  Chapter 9 Phase 1: Capability Broker Tests Complete ✓\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("\n");
}
