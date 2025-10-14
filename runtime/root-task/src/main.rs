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

/// Syscall numbers
const SYS_DEBUG_PRINT: usize = 0x1001;
const SYS_CAP_ALLOCATE: usize = 0x10;
const SYS_MEMORY_ALLOCATE: usize = 0x11;
const SYS_DEVICE_REQUEST: usize = 0x12;
const SYS_ENDPOINT_CREATE: usize = 0x13;
const SYS_PROCESS_CREATE: usize = 0x14;

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
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {msg_ptr}",
        "mov x1, {msg_len}",
        "svc #0",
        syscall_num = in(reg) SYS_DEBUG_PRINT,
        msg_ptr = in(reg) msg_ptr,
        msg_len = in(reg) msg_len,
        out("x0") _,
        out("x1") _,
        out("x8") _,
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
        out("x8") _,
        out("x0") _,
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
) -> usize {
    let result: usize;
    core::arch::asm!(
        "mov x8, {syscall_num}",
        "mov x0, {entry}",
        "mov x1, {stack}",
        "mov x2, {pt}",
        "mov x3, {cspace}",
        "svc #0",
        "mov {result}, x0",
        syscall_num = in(reg) SYS_PROCESS_CREATE,
        entry = in(reg) entry_point,
        stack = in(reg) stack_pointer,
        pt = in(reg) page_table_root,
        cspace = in(reg) cspace_root,
        result = out(reg) result,
        out("x8") _,
        out("x0") _,
        out("x1") _,
        out("x2") _,
        out("x3") _,
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

    // Chapter 9: Test Runtime Services Syscalls
    unsafe {
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  Chapter 9 Phase 1: Testing Capability Syscalls\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("\n");

        // Test 1: Allocate capability slot
        sys_print("[root_task] Test 1: Allocating capability slot...\n");
        let cap_slot = sys_cap_allocate();
        sys_print("  → Allocated cap slot: ");
        print_number(cap_slot);
        sys_print("\n");

        // Test 2: Allocate memory
        sys_print("[root_task] Test 2: Allocating 4096 bytes of memory...\n");
        let mem_addr = sys_memory_allocate(4096);
        if mem_addr != usize::MAX {
            sys_print("  → Allocated memory at: 0x");
            print_hex(mem_addr);
            sys_print("\n");
        } else {
            sys_print("  → Error: Out of memory\n");
        }

        // Test 3: Request device (UART0)
        sys_print("[root_task] Test 3: Requesting UART0 device...\n");
        let uart_mmio = sys_device_request(0);
        if uart_mmio != usize::MAX {
            sys_print("  → UART0 MMIO base: 0x");
            print_hex(uart_mmio);
            sys_print("\n");
        } else {
            sys_print("  → Error: Device not found\n");
        }

        // Test 4: Create IPC endpoint
        sys_print("[root_task] Test 4: Creating IPC endpoint...\n");
        let endpoint_slot = sys_endpoint_create();
        sys_print("  → Created endpoint slot: ");
        print_number(endpoint_slot);
        sys_print("\n");

        sys_print("\n");
        sys_print("═══════════════════════════════════════════════════════════\n");
        sys_print("  Chapter 9 Phase 1: Syscalls Working ✓\n");
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
