//! System Initializer Component
//!
//! This is the first component spawned by root-task. It is responsible for:
//! - Discovering and spawning system drivers
//! - Initializing system services
//! - Starting user applications
//! - Managing component lifecycle
//!
//! The system_init component acts as the "system bootstrapper" - it receives
//! elevated privileges from root-task to spawn other components, then steps
//! back into a monitoring/management role.

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use kaal_sdk::{
    component::{Component, ServiceBase},
    syscall,
};

// Declare this as a system service with process creation privileges
kaal_sdk::component_metadata! {
    name: "system_init",
    type: Service,
    version: "0.1.0",
    capabilities: ["process:create", "process:destroy", "memory:allocate", "ipc:*"],
}

/// System initialization state
struct SystemInit {
    base: ServiceBase,
    components_spawned: usize,
}

impl Component for SystemInit {
    fn init() -> kaal_sdk::Result<Self> {
        syscall::print("[system_init] Initializing system bootstrapper...\n");

        let base = ServiceBase::new("system_init");

        syscall::print("[system_init] ✓ System initializer ready\n");

        Ok(Self {
            base,
            components_spawned: 0,
        })
    }

    fn run(&mut self) -> ! {
        syscall::print("\n");
        syscall::print("═══════════════════════════════════════════════════════════\n");
        syscall::print("  System Initializer - Bootstrapping KaaL System\n");
        syscall::print("═══════════════════════════════════════════════════════════\n");
        syscall::print("\n");

        // Phase 1: Spawn device drivers
        syscall::print("[system_init] Phase 1: Spawning device drivers...\n");
        self.spawn_drivers();

        // Phase 2: Spawn system services
        syscall::print("[system_init] Phase 2: Spawning system services...\n");
        self.spawn_services();

        // Phase 3: Spawn applications
        syscall::print("[system_init] Phase 3: Spawning applications...\n");
        self.spawn_applications();

        syscall::print("\n");
        syscall::print("═══════════════════════════════════════════════════════════\n");
        syscall::print("  System Initialization Complete ✓\n");
        syscall::print("  Total components spawned: ");
        print_number(self.components_spawned);
        syscall::print("\n");
        syscall::print("═══════════════════════════════════════════════════════════\n");
        syscall::print("\n");

        // Phase 4: Enter monitoring loop
        syscall::print("[system_init] Entering monitoring loop...\n");
        self.monitoring_loop()
    }
}

impl SystemInit {
    /// Spawn device drivers
    fn spawn_drivers(&mut self) {
        syscall::print("  → Serial driver...\n");
        if self.spawn_component("serial_driver") {
            syscall::print("    ✓ Serial driver spawned\n");
        } else {
            syscall::print("    ✗ Failed to spawn serial driver\n");
        }

        syscall::print("  → Timer driver...\n");
        if self.spawn_component("timer_driver") {
            syscall::print("    ✓ Timer driver spawned\n");
        } else {
            syscall::print("    ✗ Failed to spawn timer driver\n");
        }
    }

    /// Spawn system services
    fn spawn_services(&mut self) {
        syscall::print("  → Process manager...\n");
        if self.spawn_component("process_manager") {
            syscall::print("    ✓ Process manager spawned\n");
        } else {
            syscall::print("    ✗ Failed to spawn process manager\n");
        }

        syscall::print("  → VFS service...\n");
        if self.spawn_component("vfs_service") {
            syscall::print("    ✓ VFS service spawned\n");
        } else {
            syscall::print("    ✗ Failed to spawn VFS service\n");
        }
    }

    /// Spawn user applications
    fn spawn_applications(&mut self) {
        syscall::print("  → Shell...\n");
        if self.spawn_component("shell") {
            syscall::print("    ✓ Shell spawned\n");
        } else {
            syscall::print("    ✗ Failed to spawn shell\n");
        }
    }

    /// Spawn a component by name
    ///
    /// In a full implementation, this would:
    /// 1. Look up component in registry/manifest
    /// 2. Load component binary (ELF)
    /// 3. Create process (TCB + address space + stack)
    /// 4. Grant capabilities per manifest
    /// 5. Start process execution
    ///
    /// For now, this is a placeholder that demonstrates the pattern.
    fn spawn_component(&mut self, _name: &str) -> bool {
        // TODO: Implement actual component spawning via syscalls
        // This requires:
        // - SYS_PROCESS_CREATE syscall implementation
        // - ELF loader in userspace
        // - Capability granting mechanism

        // For now, just count as spawned for demonstration
        self.components_spawned += 1;

        // Return false to indicate not yet implemented
        false
    }

    /// Monitoring loop - watches component health
    fn monitoring_loop(&mut self) -> ! {
        syscall::print("[system_init] Monitoring system components...\n");
        syscall::print("  (In production: watch for crashes, respawn failed components)\n");
        syscall::print("\n");

        // In a full implementation, this would:
        // - Monitor component health via IPC heartbeats
        // - Detect crashed components
        // - Restart failed components
        // - Handle graceful shutdown
        // - Manage dynamic component loading/unloading

        loop {
            syscall::yield_now();

            // Idle - wait for events
            unsafe {
                core::arch::asm!("wfi");
            }
        }
    }
}

/// Print a number (simple implementation)
fn print_number(num: usize) {
    if num == 0 {
        syscall::print("0");
        return;
    }

    let mut n = num;
    let mut digits = [0u8; 20];
    let mut digit_count = 0;

    while n > 0 {
        digits[digit_count] = (n % 10) as u8 + b'0';
        digit_count += 1;
        n /= 10;
    }

    // Print digits in reverse order
    for i in (0..digit_count).rev() {
        let digit_str = unsafe {
            core::str::from_utf8_unchecked(&digits[i..i+1])
        };
        syscall::print(digit_str);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    syscall::print("\n");
    syscall::print("═══════════════════════════════════════════════════════════\n");
    syscall::print("  System Initializer Component\n");
    syscall::print("  First component spawned by root-task\n");
    syscall::print("═══════════════════════════════════════════════════════════\n");
    syscall::print("\n");

    // Start the system initializer component
    SystemInit::start()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    syscall::print("[system_init] PANIC: System initialization failed!\n");

    if let Some(location) = info.location() {
        syscall::print("  Location: ");
        syscall::print(location.file());
        syscall::print("\n");
    }

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
