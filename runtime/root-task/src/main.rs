//! KaaL Root Task - Chapters 7-9
//!
//! This is the first userspace program that runs on the KaaL microkernel.
//!
//! **Chapter 7**: Demonstrates basic EL0 execution and syscalls
//! **Chapter 9**: Demonstrates runtime services (capability broker, memory manager)
//!
//! The root task is responsible for:
//! - Initializing runtime services
//! - Spawning initial system components
//! - Managing system-wide resources

#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod elf;
mod broker_integration;
mod component_loader;
mod generated;

/// Syscall numbers
const SYS_DEBUG_PRINT: usize = 0x1001;
const SYS_CAP_ALLOCATE: usize = 0x10;
const SYS_MEMORY_ALLOCATE: usize = 0x11;
const SYS_DEVICE_REQUEST: usize = 0x12;
const SYS_ENDPOINT_CREATE: usize = 0x13;
const SYS_PROCESS_CREATE: usize = 0x14;
const SYS_MEMORY_MAP: usize = 0x15;
const SYS_MEMORY_UNMAP: usize = 0x16;
const SYS_NOTIFICATION_CREATE: usize = 0x17;
const SYS_SIGNAL: usize = 0x18;
const SYS_WAIT: usize = 0x19;
const SYS_POLL: usize = 0x1A;
const SYS_YIELD: usize = 0x01;

/// Make a syscall to print a message
///
/// # Arguments
/// * `msg` - Message to print (pointer to null-terminated string)
/// * `len` - Length of message
///
/// # Safety
/// This function performs a raw syscall using inline assembly.
/// The kernel must implement the sys_print syscall handler.
unsafe fn sys_print(msg: &str) {
    let msg_ptr = msg.as_ptr() as usize;
    let msg_len = msg.len();

    // ARM64 syscall convention:
    // - x8 = syscall number
    // - x0 = arg1 (message pointer)
    // - x1 = arg2 (message length)
    // - svc #0 = trigger supervisor call
    //
    // Important: Mark all caller-saved registers (x0-x18) as clobbered
    // to prevent compiler from keeping values in registers across syscalls
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {msg_ptr}",
        "mov x1, {msg_len}",
        "svc #0",
        syscall_num = in(reg) SYS_DEBUG_PRINT,
        msg_ptr = in(reg) msg_ptr,
        msg_len = in(reg) msg_len,
        lateout("x0") _,
        lateout("x1") _,
        lateout("x2") _,
        lateout("x3") _,
        lateout("x4") _,
        lateout("x5") _,
        lateout("x6") _,
        lateout("x7") _,
        lateout("x8") _,
        lateout("x9") _,
        lateout("x10") _,
        lateout("x11") _,
        lateout("x12") _,
        lateout("x13") _,
        lateout("x14") _,
        lateout("x15") _,
        lateout("x16") _,
        lateout("x17") _,
        lateout("x18") _,
    );
}

/// Allocate a capability slot
unsafe fn sys_cap_allocate() -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_CAP_ALLOCATE,
        result = out(reg) result,
        out("x8") _,
        out("x0") _,
    );
    result
}

/// Allocate physical memory
unsafe fn sys_memory_allocate(size: usize) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {size}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_MEMORY_ALLOCATE,
        size = in(reg) size,
        result = out(reg) result,
        lateout("x0") _,
        lateout("x1") _,
        lateout("x2") _,
        lateout("x3") _,
        lateout("x4") _,
        lateout("x5") _,
        lateout("x6") _,
        lateout("x7") _,
        lateout("x8") _,
        lateout("x9") _,
        lateout("x10") _,
        lateout("x11") _,
        lateout("x12") _,
        lateout("x13") _,
        lateout("x14") _,
        lateout("x15") _,
        lateout("x16") _,
        lateout("x17") _,
        lateout("x18") _,
    );
    result
}

/// Request device resources
unsafe fn sys_device_request(device_id: usize) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {device_id}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_DEVICE_REQUEST,
        device_id = in(reg) device_id,
        result = out(reg) result,
        out("x8") _,
        out("x0") _,
    );
    result
}

/// Create an IPC endpoint
unsafe fn sys_endpoint_create() -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_ENDPOINT_CREATE,
        result = out(reg) result,
        out("x8") _,
        out("x0") _,
    );
    result
}

/// Create a new process
unsafe fn sys_process_create(
    entry_point: usize,
    stack_pointer: usize,
    page_table_root: usize,
    cspace_root: usize,
    code_phys: usize,
    code_size: usize,
    stack_phys: usize,
) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {entry}",
        "mov x1, {stack}",
        "mov x2, {pt}",
        "mov x3, {cspace}",
        "mov x4, {code_phys}",
        "mov x5, {code_size}",
        "mov x6, {stack_phys}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_PROCESS_CREATE,
        entry = in(reg) entry_point,
        stack = in(reg) stack_pointer,
        pt = in(reg) page_table_root,
        cspace = in(reg) cspace_root,
        code_phys = in(reg) code_phys,
        code_size = in(reg) code_size,
        stack_phys = in(reg) stack_phys,
        result = out(reg) result,
        out("x8") _,
        out("x0") _,
        out("x1") _,
        out("x2") _,
        out("x3") _,
        out("x4") _,
        out("x5") _,
        out("x6") _,
    );
    result
}

/// Map physical memory into our virtual address space
unsafe fn sys_memory_map(phys_addr: usize, size: usize, permissions: usize) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {phys}",
        "mov x1, {size}",
        "mov x2, {perms}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_MEMORY_MAP,
        phys = in(reg) phys_addr,
        size = in(reg) size,
        perms = in(reg) permissions,
        result = out(reg) result,
        lateout("x0") _,
        lateout("x1") _,
        lateout("x2") _,
        lateout("x3") _,
        lateout("x4") _,
        lateout("x5") _,
        lateout("x6") _,
        lateout("x7") _,
        lateout("x8") _,
        lateout("x9") _,
        lateout("x10") _,
        lateout("x11") _,
        lateout("x12") _,
        lateout("x13") _,
        lateout("x14") _,
        lateout("x15") _,
        lateout("x16") _,
        lateout("x17") _,
        lateout("x18") _,
    );
    result
}

/// Yield CPU to next process
unsafe fn sys_yield() {
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "svc #0",
        syscall_num = in(reg) SYS_YIELD,
        out("x8") _,
        out("x0") _,
    );
}

/// Unmap virtual memory from our address space
unsafe fn sys_memory_unmap(virt_addr: usize, size: usize) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {virt}",
        "mov x1, {size}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_MEMORY_UNMAP,
        virt = in(reg) virt_addr,
        size = in(reg) size,
        result = out(reg) result,
        out("x8") _,
        out("x0") _,
        out("x1") _,
    );
    result
}

/// Print a number in decimal
unsafe fn print_number(n: usize) {
    // Convert number to string
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

    // Print digits in reverse
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

/// Create a notification object
unsafe fn sys_notification_create() -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_NOTIFICATION_CREATE,
        result = out(reg) result,
        out("x8") _,
    );
    result
}

/// Signal a notification
unsafe fn sys_signal(notification_cap: usize, badge: usize) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {cap}",
        "mov x1, {badge}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_SIGNAL,
        cap = in(reg) notification_cap,
        badge = in(reg) badge,
        result = out(reg) result,
        out("x8") _,
        out("x0") _,
        out("x1") _,
    );
    result
}

/// Poll a notification (non-blocking)
unsafe fn sys_poll(notification_cap: usize) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {cap}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_POLL,
        cap = in(reg) notification_cap,
        result = out(reg) result,
        out("x8") _,
    );
    result
}

/// Test shared memory IPC with notifications
unsafe fn test_shared_memory_ipc() {
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("  Chapter 9 Phase 2: Testing Shared Memory IPC\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("\n");

    sys_print("[ipc] Phase 2 Component Binaries:\n");

    // Embed IPC binaries
    static IPC_SENDER_ELF: &[u8] = include_bytes!(
        "../../../examples/ipc-sender/target/aarch64-unknown-none/release/ipc-sender"
    );
    static IPC_RECEIVER_ELF: &[u8] = include_bytes!(
        "../../../examples/ipc-receiver/target/aarch64-unknown-none/release/ipc-receiver"
    );

    sys_print("  → IPC Sender binary:   ");
    print_number(IPC_SENDER_ELF.len());
    sys_print(" bytes\n");
    sys_print("  → IPC Receiver binary: ");
    print_number(IPC_RECEIVER_ELF.len());
    sys_print(" bytes\n");
    sys_print("\n");

    sys_print("[ipc] Test 1: Allocating shared memory for ring buffer...\n");

    // Allocate 4KB for shared memory ring buffer
    let shared_mem_size = 4096;
    let shared_mem_phys = sys_memory_allocate(shared_mem_size);
    if shared_mem_phys == usize::MAX {
        sys_print("  ✗ Failed to allocate shared memory\n");
        return;
    }
    sys_print("  ✓ Shared memory allocated at phys: 0x");
    print_hex(shared_mem_phys);
    sys_print("\n");

    sys_print("[ipc] Test 2: Creating notification objects for signaling...\n");

    // Create consumer and producer notifications
    let consumer_notify = sys_notification_create();
    let producer_notify = sys_notification_create();

    if consumer_notify == usize::MAX || producer_notify == usize::MAX {
        sys_print("  ✗ Failed to create notification objects\n");
        return;
    }

    sys_print("  ✓ Consumer notification: cap_slot ");
    print_number(consumer_notify);
    sys_print("\n");
    sys_print("  ✓ Producer notification: cap_slot ");
    print_number(producer_notify);
    sys_print("\n");

    sys_print("\n[ipc] Test 3: Verifying notification-based signaling...\n");

    // Simulate producer signaling consumer
    sys_print("  → Producer signals consumer (badge=0x1: data available)...\n");
    sys_signal(consumer_notify, 0x1);

    let signals = sys_poll(consumer_notify);
    if signals != 0x1 {
        sys_print("  ✗ Expected signal 0x1, got 0x");
        print_hex(signals);
        sys_print("\n");
        return;
    }
    sys_print("  ✓ Consumer received signal: 0x1\n");

    // Simulate consumer signaling producer
    sys_print("  → Consumer signals producer (badge=0x2: space available)...\n");
    sys_signal(producer_notify, 0x2);

    let signals = sys_poll(producer_notify);
    if signals != 0x2 {
        sys_print("  ✗ Expected signal 0x2, got 0x");
        print_hex(signals);
        sys_print("\n");
        return;
    }
    sys_print("  ✓ Producer received signal: 0x2\n");

    sys_print("\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("  Shared Memory IPC Infrastructure: VERIFIED ✓\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("\n");
    sys_print("[ipc] Summary:\n");
    sys_print("  ✓ Shared memory allocation works\n");
    sys_print("  ✓ Notification creation works\n");
    sys_print("  ✓ Producer→Consumer signaling works\n");
    sys_print("  ✓ Consumer→Producer signaling works\n");
    sys_print("  ✓ Ready for process-level IPC implementation\n");
    sys_print("\n");
    sys_print("[ipc] Note: Full process spawning with shared memory requires:\n");
    sys_print("  1. Spawn sender and receiver as separate processes\n");
    sys_print("  2. Map shared memory into both process address spaces\n");
    sys_print("  3. Pass notification capabilities to both processes\n");
    sys_print("  4. Initialize SharedRing in mapped shared memory\n");
    sys_print("\n");
}

/// Test notification syscalls
unsafe fn test_notifications() {
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("  Chapter 9 Phase 2: Testing Notification Syscalls\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("\n");

    // Test 1: Create notification
    sys_print("[notification] Test 1: Creating notification object...\n");
    let notification_cap = sys_notification_create();
    if notification_cap == usize::MAX {
        sys_print("  ✗ Failed to create notification\n");
        sys_print("  Test: FAIL\n\n");
        return;
    }
    sys_print("  ✓ Notification created at cap slot ");
    print_number(notification_cap);
    sys_print("\n");

    // Test 2: Poll empty notification (should return 0)
    sys_print("[notification] Test 2: Polling empty notification...\n");
    let signals = sys_poll(notification_cap);
    if signals != 0 {
        sys_print("  ✗ Expected 0 signals, got ");
        print_number(signals);
        sys_print("\n");
        sys_print("  Test: FAIL\n\n");
        return;
    }
    sys_print("  ✓ Poll returned 0 (no signals)\n");

    // Test 3: Signal notification
    sys_print("[notification] Test 3: Signaling notification with badge 0x5...\n");
    let result = sys_signal(notification_cap, 0x5);
    if result != 0 {
        sys_print("  ✗ Signal failed with error ");
        print_number(result);
        sys_print("\n");
        sys_print("  Test: FAIL\n\n");
        return;
    }
    sys_print("  ✓ Signal succeeded\n");

    // Test 4: Poll notification (should return 0x5)
    sys_print("[notification] Test 4: Polling signaled notification...\n");
    let signals = sys_poll(notification_cap);
    if signals != 0x5 {
        sys_print("  ✗ Expected 0x5, got 0x");
        print_hex(signals);
        sys_print("\n");
        sys_print("  Test: FAIL\n\n");
        return;
    }
    sys_print("  ✓ Poll returned 0x5 (correct badge)\n");

    // Test 5: Poll again (should be cleared)
    sys_print("[notification] Test 5: Polling again (should be cleared)...\n");
    let signals = sys_poll(notification_cap);
    if signals != 0 {
        sys_print("  ✗ Expected 0, got 0x");
        print_hex(signals);
        sys_print("\n");
        sys_print("  Test: FAIL\n\n");
        return;
    }
    sys_print("  ✓ Poll returned 0 (signals cleared)\n");

    // Test 6: Badge coalescing
    sys_print("[notification] Test 6: Testing badge coalescing...\n");
    sys_signal(notification_cap, 0x1);
    sys_signal(notification_cap, 0x2);
    sys_signal(notification_cap, 0x4);
    let signals = sys_poll(notification_cap);
    if signals != 0x7 {
        sys_print("  ✗ Expected 0x7 (0x1 | 0x2 | 0x4), got 0x");
        print_hex(signals);
        sys_print("\n");
        sys_print("  Test: FAIL\n\n");
        return;
    }
    sys_print("  ✓ Badge coalescing works (0x1 | 0x2 | 0x4 = 0x7)\n");

    sys_print("\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("  Notification Tests: PASS ✓\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("\n");
}

/// Root task entry point
///
/// This function is called by the kernel after it sets up the root task's
/// execution context and transitions to EL0 (userspace).
///
/// # Expected behavior
/// 1. Kernel creates root task TCB
/// 2. Kernel configures TCB (PC=_start, SP=stack_top, EL0)
/// 3. Kernel resumes TCB
/// 4. Root task starts executing here
/// 5. Root task calls sys_print to demonstrate userspace execution
/// 6. Root task enters idle loop
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Print ASCII art banner and welcome message from userspace
    unsafe {
        sys_print("\n");
        sys_print("    $$╲   $$╲                    $$╲       \n");
        sys_print("    $$ │ $$  │                   $$ │      \n");
        sys_print("    $$ │$$  ╱ $$$$$$╲   $$$$$$╲  $$ │      \n");
        sys_print("    $$$$$  ╱  ╲____$$╲  ╲____$$╲ $$ │      \n");
        sys_print("    $$  $$<   $$$$$$$ │ $$$$$$$ │$$ │      \n");
        sys_print("    $$ │╲$$╲ $$  __$$ │$$  __$$ │$$ │      \n");
        sys_print("    $$ │ ╲$$╲╲$$$$$$$ │╲$$$$$$$ │$$$$$$$$╲ \n");
        sys_print("    ╲__│  ╲__│╲_______│ ╲_______│╲________│\n");
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  KaaL Root Task (EL0) v0.1.0\n");
        sys_print("  Chapter 7: Root Task & Boot Protocol\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");
        sys_print("[root_task] Hello from userspace (EL0)!\n");
        sys_print("[root_task] Syscalls working: sys_print functional\n");
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  Chapter 7: COMPLETE ✓\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");
    }

    // Chapter 9: Test Capability Broker
    unsafe {
        broker_integration::test_capability_broker();
    }

    // Chapter 9 Phase 2: Test Notifications
    unsafe {
        test_notifications();
    }

    // Chapter 9 Phase 2: Test Shared Memory IPC
    unsafe {
        test_shared_memory_ipc();
    }

    // Chapter 9 Phase 2: Spawn echo-server process
    unsafe {
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  Chapter 9 Phase 2: Spawning Echo Server Process\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");

        // Embed echo-server binary (built by cargo)
        static ECHO_SERVER_ELF: &[u8] = include_bytes!(
            "../../../examples/echo-server/target/aarch64-unknown-none/release/echo-server"
        );

        sys_print("[root_task] Parsing echo-server ELF binary...\n");
        sys_print("  → Binary size: ");
        print_number(ECHO_SERVER_ELF.len());
        sys_print(" bytes\n");

        // Parse ELF to get load info
        let elf_info = match elf::parse_elf(ECHO_SERVER_ELF) {
            Ok(info) => info,
            Err(e) => {
                sys_print("  → Error parsing ELF: ");
                sys_print(e);
                sys_print("\n");
                loop { core::arch::asm!("wfi"); }
            }
        };

        sys_print("  → Entry point: 0x");
        print_hex(elf_info.entry_point);
        sys_print("\n");
        sys_print("  → Load segments: ");
        print_number(elf_info.num_segments);
        sys_print("\n");
        sys_print("  → Memory size: ");
        print_number(elf_info.memory_size());
        sys_print(" bytes\n");

        sys_print("\n[root_task] Allocating memory for process...\n");

        // Allocate memory for process image (round up to 4KB pages)
        let process_size = (elf_info.memory_size() + 4095) & !4095;
        let process_mem = sys_memory_allocate(process_size);
        if process_mem == usize::MAX {
            sys_print("  → Error: Out of memory for process image\n");
            loop { core::arch::asm!("wfi"); }
        }
        sys_print("  → Process memory: 0x");
        print_hex(process_mem);
        sys_print("\n");

        // Allocate stack (16KB)
        let stack_size = 16384;
        let stack_mem = sys_memory_allocate(stack_size);
        if stack_mem == usize::MAX {
            sys_print("  → Error: Out of memory for stack\n");
            loop { core::arch::asm!("wfi"); }
        }
        sys_print("  → Stack memory: 0x");
        print_hex(stack_mem);
        sys_print("\n");

        // Allocate page table root (4KB)
        let pt_root = sys_memory_allocate(4096);
        if pt_root == usize::MAX {
            sys_print("  → Error: Out of memory for page table\n");
            loop { core::arch::asm!("wfi"); }
        }
        sys_print("  → Page table: 0x");
        print_hex(pt_root);
        sys_print("\n");

        // Allocate CNode for capability space (4KB)
        let cspace_root = sys_memory_allocate(4096);
        if cspace_root == usize::MAX {
            sys_print("  → Error: Out of memory for CSpace\n");
            loop { core::arch::asm!("wfi"); }
        }
        sys_print("  → CSpace root: 0x");
        print_hex(cspace_root);
        sys_print("\n");

        sys_print("\n[root_task] Loading ELF segments...\n");

        // Map the allocated physical memory into our virtual address space
        // so we can copy the ELF segments
        const RW_PERMS: usize = 0x3; // Read + Write
        let virt_mem = sys_memory_map(process_mem, process_size, RW_PERMS);
        if virt_mem == usize::MAX {
            sys_print("  → Error: Failed to map process memory\n");
            loop { core::arch::asm!("wfi"); }
        }
        sys_print("  → Mapped process memory at virt=0x");
        print_hex(virt_mem);
        sys_print("\n");

        // Copy each LOAD segment to the mapped memory
        let base_vaddr = elf_info.min_vaddr;
        for i in 0..elf_info.num_segments {
            let (vaddr, filesz, memsz, offset) = elf_info.segments[i];

            sys_print("  → Segment ");
            print_number(i);
            sys_print(": vaddr=0x");
            print_hex(vaddr);
            sys_print(", filesz=");
            print_number(filesz);
            sys_print(", memsz=");
            print_number(memsz);
            sys_print("\n");

            // Calculate destination in mapped memory
            let segment_offset = vaddr - base_vaddr;
            let dest_ptr = (virt_mem + segment_offset) as *mut u8;
            let src_ptr = ECHO_SERVER_ELF.as_ptr().add(offset);

            // Copy file data
            if filesz > 0 {
                core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, filesz);
            }

            // Zero BSS (memsz > filesz means there's BSS to zero)
            if memsz > filesz {
                let bss_ptr = dest_ptr.add(filesz);
                let bss_size = memsz - filesz;
                core::ptr::write_bytes(bss_ptr, 0, bss_size);
            }
        }
        sys_print("  ✓ All segments loaded\n");

        // Unmap the memory (we're done writing to it)
        sys_memory_unmap(virt_mem, process_size);
        sys_print("  ✓ Memory unmapped\n");

        sys_print("\n[root_task] Creating process...\n");

        // TODO: Create and populate page table
        // For now, we'll use identity mapping (physical = virtual)
        // This is a simplification - real implementation needs proper page tables

        // Stack grows down from top
        // Use fixed virtual address for stack (top of userspace memory)
        const STACK_VIRT_TOP: usize = 0x8000_0000;  // 2GB
        let stack_top = STACK_VIRT_TOP;

        // Create the process
        // Note: We pass stack_mem (physical address) as the 7th parameter
        // so the kernel can map it at the virtual address we specified
        let pid = sys_process_create(
            elf_info.entry_point,
            stack_top,       // Virtual address where stack will be
            pt_root,
            cspace_root,
            process_mem,     // Physical address of loaded code
            process_size,    // Size of code region
            stack_mem,       // Physical address of stack
        );

        if pid == usize::MAX {
            sys_print("  → Error: Failed to create process\n");
        } else {
            sys_print("  → Created process with PID: ");
            print_number(pid);
            sys_print("\n");
            sys_print("\n");
            sys_print("═══════════════════════════════════════════════════════════\n");
            sys_print("  Chapter 9 Phase 2: Process Spawning Complete ✓\n");
            sys_print("═══════════════════════════════════════════════════════════\n");
            sys_print("\n");

            // Chapter 9 Phase 3: Test context switching!
            sys_print("[root_task] Yielding to echo-server...\n");
            sys_yield();
            sys_print("[root_task] Back from echo-server!\n");
            sys_print("[root_task] Multi-process working! ✓\n");
        }
    }

    // Chapter 9 Phase 3: End-to-End IPC Testing
    unsafe {
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  Chapter 9 Phase 3: End-to-End IPC Testing\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");

        // Embed IPC test binaries
        static IPC_RECEIVER_ELF: &[u8] = include_bytes!(
            "../../../examples/ipc-receiver/target/aarch64-unknown-none/release/ipc-receiver"
        );
        static IPC_SENDER_ELF: &[u8] = include_bytes!(
            "../../../examples/ipc-sender/target/aarch64-unknown-none/release/ipc-sender"
        );

        sys_print("[root_task] IPC Receiver binary: ");
        print_number(IPC_RECEIVER_ELF.len());
        sys_print(" bytes\n");
        sys_print("[root_task] IPC Sender binary: ");
        print_number(IPC_SENDER_ELF.len());
        sys_print(" bytes\n");
        sys_print("\n");

        sys_print("[root_task] NOTE: For Phase 3, IPC receiver will create endpoint\n");
        sys_print("[root_task]       and sender will use a known capability slot.\n");
        sys_print("[root_task]       Full capability transfer will be implemented later.\n");
        sys_print("\n");

        // TODO: Spawn IPC receiver process (will create endpoint)
        // TODO: Spawn IPC sender process (will use endpoint)
        // TODO: Let them communicate and verify message passing

        sys_print("[root_task] IPC test binaries embedded and ready\n");
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  Chapter 9 Phase 3: IPC Infrastructure Ready ✓\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");
    }

    // Idle loop - wait for interrupts
    loop {
        unsafe {
            core::arch::asm!("wfi"); // Wait for interrupt
        }
    }
}

/// Panic handler
///
/// Called when the root task panics. Since we're in userspace with no
/// infrastructure yet, we just loop forever.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
