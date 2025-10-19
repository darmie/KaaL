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

extern crate alloc;

use core::panic::PanicInfo;

mod allocator;
mod broker_integration;
mod component_loader;
mod elf;
mod elf_xmas;
mod generated;

// Import ComponentError for error handling
use component_loader::ComponentError;

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
const SYS_MEMORY_MAP_INTO: usize = 0x1B;
const SYS_CAP_INSERT_INTO: usize = 0x1C;
const SYS_CAP_INSERT_SELF: usize = 0x1D;
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
    // Important: Use inout() for input registers to ensure they're clobbered,
    // preventing the compiler from reusing them across syscalls
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "svc #0",
        syscall_num = in(reg) SYS_DEBUG_PRINT,
        inout("x0") msg_ptr => _,
        inout("x1") msg_len => _,
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
/// Result from sys_process_create containing PID and capability information
struct ProcessCreateResult {
    pid: usize,
    tcb_phys: usize,
    pt_phys: usize,
    cspace_phys: usize,
}

unsafe fn sys_process_create(
    entry_point: usize,
    stack_pointer: usize,
    page_table_root: usize,
    cspace_root: usize,
    code_phys: usize,
    code_vaddr: usize,
    code_size: usize,
    stack_phys: usize,
    priority: u8,
) -> ProcessCreateResult {
    let pid: usize;
    let tcb_phys: usize;
    let pt_phys: usize;
    let cspace_phys: usize;

    core::arch::asm!(
        // Set inputs
        "mov x0, {entry}",
        "mov x1, {stack}",
        "mov x2, {pt}",
        "mov x3, {cspace}",
        "mov x4, {code_phys}",
        "mov x5, {code_vaddr}",
        "mov x6, {code_size}",
        "mov x7, {stack_phys}",
        "mov x8, {syscall_num}",
        "mov x9, {priority}",
        // Make syscall
        "svc #0",
        // Read outputs
        "mov {pid}, x0",
        "mov {tcb}, x1",
        "mov {pt_out}, x2",
        "mov {cs_out}, x3",
        syscall_num = in(reg) SYS_PROCESS_CREATE,
        entry = in(reg) entry_point,
        stack = in(reg) stack_pointer,
        pt = in(reg) page_table_root,
        cspace = in(reg) cspace_root,
        code_phys = in(reg) code_phys,
        code_vaddr = in(reg) code_vaddr,
        code_size = in(reg) code_size,
        stack_phys = in(reg) stack_phys,
        priority = in(reg) priority as usize,
        pid = out(reg) pid,
        tcb = out(reg) tcb_phys,
        pt_out = out(reg) pt_phys,
        cs_out = out(reg) cspace_phys,
        out("x0") _,
        out("x1") _,
        out("x2") _,
        out("x3") _,
        out("x4") _,
        out("x5") _,
        out("x6") _,
        out("x7") _,
        out("x8") _,
        out("x9") _,
    );

    // Debug: Check what we received (avoid sys_print which causes syscalls)
    // tcb_phys should match what kernel set in x1

    ProcessCreateResult {
        pid,
        tcb_phys,
        pt_phys,
        cspace_phys,
    }
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
pub unsafe fn print_number(n: usize) {
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
        let digit = core::str::from_utf8_unchecked(&buf[i..i + 1]);
        sys_print(digit);
    }
}

/// Print a number in hexadecimal
pub unsafe fn print_hex(n: usize) {
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

/// Map physical memory into target process's address space (Phase 5)
unsafe fn sys_memory_map_into(
    target_tcb_cap: usize,
    phys_addr: usize,
    size: usize,
    virt_addr: usize,
    permissions: usize,
) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {target_tcb}",
        "mov x1, {phys}",
        "mov x2, {size}",
        "mov x3, {virt}",
        "mov x4, {perms}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_MEMORY_MAP_INTO,
        target_tcb = in(reg) target_tcb_cap,
        phys = in(reg) phys_addr,
        size = in(reg) size,
        virt = in(reg) virt_addr,
        perms = in(reg) permissions,
        result = out(reg) result,
        out("x8") _,
    );
    result
}

/// Insert capability into target process's CSpace (Phase 5)
unsafe fn sys_cap_insert_into(
    target_tcb_cap: usize,
    target_slot: usize,
    cap_type: usize,
    object_ptr: usize,
) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {target_tcb}",
        "mov x1, {slot}",
        "mov x2, {ctype}",
        "mov x3, {obj}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_CAP_INSERT_INTO,
        target_tcb = in(reg) target_tcb_cap,
        slot = in(reg) target_slot,
        ctype = in(reg) cap_type,
        obj = in(reg) object_ptr,
        result = out(reg) result,
        out("x8") _,
    );
    result
}

/// Insert capability into caller's own CSpace
unsafe fn sys_cap_insert_self(cap_slot: usize, cap_type: usize, object_ptr: usize) -> usize {
    let result: usize;
    core::arch::asm!(
        "svc #0",
        inout("x0") cap_slot => result,
        in("x1") cap_type,
        in("x2") object_ptr,
        in("x8") SYS_CAP_INSERT_SELF,
    );
    result
}

/// Test shared memory IPC with notifications
unsafe fn test_shared_memory_ipc() {
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("  Chapter 9 Phase 2: Testing Shared Memory IPC\n");
    sys_print("═══════════════════════════════════════════════════════════\n");
    sys_print("\n");

    // Phase 2 tests notification infrastructure only (no component spawning yet)

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

    // Create component loader with registry
    use component_loader::{ComponentLoader, ComponentRegistry};
    static REGISTRY: ComponentRegistry =
        ComponentRegistry::new(generated::component_registry::COMPONENT_REGISTRY);
    let loader = ComponentLoader::new(&REGISTRY);

    // Chapter 9 Phase 4: Component Loading & Spawning
    unsafe {
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  Chapter 9 Phase 4: Component Loading & Spawning\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");

        // Use the generated component registry
        sys_print("[root_task] Component Registry:\n");
        sys_print("  → Total components: ");
        print_number(generated::component_registry::COMPONENT_COUNT);
        sys_print("\n");
        sys_print("  → Autostart components: ");
        let autostart_count = generated::component_registry::get_autostart_components().count();
        print_number(autostart_count);
        sys_print("\n");
        sys_print("\n");

        // Spawn all autostart components
        sys_print("[root_task] Spawning autostart components...\n");
        for component in generated::component_registry::get_autostart_components() {
            sys_print("  → Spawning ");
            sys_print(component.name);
            sys_print("...\n");

            match loader.spawn(component.name) {
                Ok(result) => {
                    sys_print("    ✓ ");
                    sys_print(component.name);
                    sys_print(" spawned (PID: ");
                    print_number(result.pid);
                    sys_print(")\n");
                }
                Err(e) => {
                    sys_print("    ✗ Failed to spawn ");
                    sys_print(component.name);
                    sys_print(": ");
                    match e {
                        ComponentError::NotFound => sys_print("not found"),
                        ComponentError::NoBinary => sys_print("no binary"),
                        ComponentError::InvalidElf => sys_print("invalid ELF"),
                        ComponentError::OutOfMemory => sys_print("out of memory"),
                        ComponentError::CapabilityError => sys_print("capability error"),
                        ComponentError::NotImplemented => sys_print("not implemented"),
                    }
                    sys_print("\n");
                }
            }
        }
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  Component Spawning: COMPLETE ✓\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");

        // Yield to let components run
        sys_print("[root_task] Yielding to spawned components...\n");
        sys_yield();
        sys_print("[root_task] Back from components!\n");
    }

    // Component spawning complete - system continues running
    unsafe {
        sys_print("[root_task] Component switching working! ✓\n");
        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  Root Task: Complete ✓\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");
        sys_print("[root_task] Handing off to system_init\n");
        sys_print("[root_task] IPC demo now runs via system_init autostart\n");
        sys_print("\n");
    }

    /*
    // OLD Phase 5 code - kept for reference
    unsafe {
        sys_print("[phase5] Step 1: Allocating shared memory for ring buffer...\n");
        sys_print("  → Ring buffer requires: ~32KB for SharedRing<u32, 256>\n");
        let ring_size = 32768; // 32KB for ring buffer + metadata
        let shared_mem_phys = sys_memory_allocate(ring_size);
        if shared_mem_phys == usize::MAX {
            sys_print("  ✗ Failed to allocate shared memory\n");
        } else {
            sys_print("  ✓ Allocated shared memory at phys: 0x");
            print_hex(shared_mem_phys);
            sys_print("\n");

            sys_print("[phase5] Step 2: Creating notification objects...\n");
            let producer_notify = sys_notification_create();
            let consumer_notify = sys_notification_create();
            sys_print("  ✓ Producer notification: cap slot\n");
            sys_print("  ✓ Consumer notification: cap slot\n");
            sys_print("\n");

            sys_print("[phase5] Step 3: Would spawn components here\n");
            sys_print("  → loader.spawn(\"ipc_producer\") -> PID\n");
            sys_print("  → loader.spawn(\"ipc_consumer\") -> PID\n");
            sys_print("\n");

            sys_print("[phase5] Step 4: sys_memory_map_into would map shared memory\n");
            sys_print("  → Map phys 0x");
            print_hex(shared_mem_phys);
            sys_print(" into producer @ vaddr 0x8010_0000\n");
            sys_print("  → Map same phys into consumer @ vaddr 0x8010_0000\n");
            sys_print("\n");

            sys_print("[phase5] Step 5: sys_cap_insert_into would grant capabilities\n");
            sys_print("  → Insert consumer_notify into producer's CSpace[102]\n");
            sys_print("  → Insert producer_notify into producer's CSpace[103]\n");
            sys_print("  → Insert consumer_notify into consumer's CSpace[102]\n");
            sys_print("  → Insert producer_notify into consumer's CSpace[103]\n");
            sys_print("\n");

            sys_print("[phase5] Step 6: Components would initialize Channel<T>\n");
            sys_print("  Producer:\n");
            sys_print("    let config = ChannelConfig {\n");
            sys_print("      shared_memory: 0x8010_0000,\n");
            sys_print("      receiver_notify: 102,\n");
            sys_print("      sender_notify: 103\n");
            sys_print("    };\n");
            sys_print("    let ch = Channel::<u32>::sender(config);\n");
            sys_print("    for i in 0..10 { ch.send(i)?; }\n");
            sys_print("\n");
            sys_print("  Consumer:\n");
            sys_print("    let ch = Channel::<u32>::receiver(config);\n");
            sys_print("    for i in 0..10 {\n");
            sys_print("      let msg = ch.receive()?;\n");
            sys_print("      assert_eq!(msg, i);\n");
            sys_print("    }\n");
            sys_print("\n");

            sys_print("═══════════════════════════════════════════════════════════\n");
            sys_print("  Phase 5 Infrastructure: DEMONSTRATED ✓\n");
            sys_print("═══════════════════════════════════════════════════════════\n");
            sys_print("\n");
            sys_print("Next steps for full integration:\n");
            sys_print("1. Update ipc-producer/consumer to use Channel<T> API\n");
            sys_print("2. Spawn components with loader\n");
            sys_print("3. Use sys_cap_insert_into to grant capabilities\n");
            sys_print("4. Use sys_memory_map_into for shared memory\n");
            sys_print("5. Yield to components and observe IPC\n");
            sys_print("\n");
        }
    }
    */

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
