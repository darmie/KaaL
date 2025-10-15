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

    // Chapter 9 Phase 2: Test IPC syscalls
    test_ipc_syscalls();
}

/// Test IPC syscalls to verify kernel integration
unsafe fn test_ipc_syscalls() {
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("  Chapter 9 Phase 2: Testing IPC Syscalls\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("\n");

    // Syscall numbers
    const SYS_SEND: u64 = 0x02;
    const SYS_RECV: u64 = 0x03;
    const SYS_CALL: u64 = 0x04;
    const SYS_REPLY: u64 = 0x05;

    // Test 1: IPC Send syscall
    sys_print("[root_task] Test 1: IPC Send syscall\n");
    sys_print("  → Calling sys_send(endpoint=102, msg_ptr=0x1000, len=18)...\n");

    let message = b"Hello from sender!";
    let result: u64;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {endpoint}",
        "mov x1, {msg_ptr}",
        "mov x2, {msg_len}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_SEND,
        endpoint = in(reg) 102u64,
        msg_ptr = in(reg) message.as_ptr() as u64,
        msg_len = in(reg) message.len() as u64,
        result = out(reg) result,
        out("x8") _,
        out("x0") _,
        out("x1") _,
        out("x2") _,
    );

    if result == 0 {
        sys_print("  ✓ sys_send returned success (0)\n");
    } else if result == u64::MAX {
        sys_print("  ✗ sys_send returned error (-1)\n");
    } else {
        sys_print("  ? sys_send returned: ");
        print_hex(result as usize);
        sys_print("\n");
    }

    // Test 2: IPC Recv syscall
    sys_print("\n[root_task] Test 2: IPC Recv syscall\n");
    sys_print("  → Calling sys_recv(endpoint=102, buf_ptr=0x2000, len=256)...\n");

    let mut buffer = [0u8; 256];
    let bytes_received: u64;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {endpoint}",
        "mov x1, {buf_ptr}",
        "mov x2, {buf_len}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_RECV,
        endpoint = in(reg) 102u64,
        buf_ptr = in(reg) buffer.as_mut_ptr() as u64,
        buf_len = in(reg) buffer.len() as u64,
        result = out(reg) bytes_received,
        out("x8") _,
        out("x0") _,
        out("x1") _,
        out("x2") _,
    );

    if bytes_received == u64::MAX {
        sys_print("  ✗ sys_recv returned error (-1)\n");
    } else {
        sys_print("  ✓ sys_recv returned ");
        print_number(bytes_received as usize);
        sys_print(" bytes\n");
    }

    // Test 3: IPC Call syscall
    sys_print("\n[root_task] Test 3: IPC Call syscall (RPC)\n");
    sys_print("  → Calling sys_call(endpoint=102, req_len=7, rep_len=256)...\n");

    let request = b"REQUEST";
    let mut reply = [0u8; 256];
    let reply_len: u64;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {endpoint}",
        "mov x1, {req_ptr}",
        "mov x2, {req_len}",
        "mov x3, {rep_ptr}",
        "mov x4, {rep_len}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_CALL,
        endpoint = in(reg) 102u64,
        req_ptr = in(reg) request.as_ptr() as u64,
        req_len = in(reg) request.len() as u64,
        rep_ptr = in(reg) reply.as_mut_ptr() as u64,
        rep_len = in(reg) reply.len() as u64,
        result = out(reg) reply_len,
        out("x8") _,
        out("x0") _,
        out("x1") _,
        out("x2") _,
        out("x3") _,
        out("x4") _,
    );

    if reply_len == u64::MAX {
        sys_print("  ✗ sys_call returned error (-1)\n");
    } else {
        sys_print("  ✓ sys_call returned ");
        print_number(reply_len as usize);
        sys_print(" bytes in reply\n");
    }

    // Test 4: IPC Reply syscall
    sys_print("\n[root_task] Test 4: IPC Reply syscall\n");
    sys_print("  → Calling sys_reply(reply_cap=200, msg_ptr=0x3000)...\n");

    let reply_msg = b"REPLY";
    let reply_result: u64;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {reply_cap}",
        "mov x1, {msg_ptr}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_REPLY,
        reply_cap = in(reg) 200u64,
        msg_ptr = in(reg) reply_msg.as_ptr() as u64,
        result = out(reg) reply_result,
        out("x8") _,
        out("x0") _,
        out("x1") _,
    );

    if reply_result == 0 {
        sys_print("  ✓ sys_reply returned success (0)\n");
    } else if reply_result == u64::MAX {
        sys_print("  ✗ sys_reply returned error (-1)\n");
    } else {
        sys_print("  ? sys_reply returned: ");
        print_hex(reply_result as usize);
        sys_print("\n");
    }

    // Test 5: Error handling - invalid parameters
    sys_print("\n[root_task] Test 5: Error handling (invalid params)\n");
    sys_print("  → Calling sys_send with invalid endpoint=9999...\n");

    let error_result: u64;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {endpoint}",
        "mov x1, {msg_ptr}",
        "mov x2, {msg_len}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_SEND,
        endpoint = in(reg) 9999u64,  // Invalid endpoint
        msg_ptr = in(reg) message.as_ptr() as u64,
        msg_len = in(reg) message.len() as u64,
        result = out(reg) error_result,
        out("x8") _,
        out("x0") _,
        out("x1") _,
        out("x2") _,
    );

    if error_result == u64::MAX {
        sys_print("  ✓ Correctly returned error (-1) for invalid endpoint\n");
    } else {
        sys_print("  ✗ Should have returned error but got: ");
        print_hex(error_result as usize);
        sys_print("\n");
    }

    sys_print("\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("  Chapter 9 Phase 2: IPC Syscall Tests Complete ✓\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("\n");
}
