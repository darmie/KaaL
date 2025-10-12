//! Kernel components
//!
//! Minimal components built into the kernel for essential functionality.
//! These are NOT full-featured drivers - they provide only what the kernel
//! needs to function.
//!
//! # Design Philosophy (seL4-inspired)
//!
//! Kernel components are MINIMAL by design:
//! - **console**: Just `putc()` for debug output (no interrupts, no buffering)
//! - **timer**: Basic ticks for scheduling (Chapter 3)
//! - **irq**: IRQ routing to user-space (Chapter 3)
//!
//! Full-featured drivers live in user-space as framework components:
//! - **uart_driver**: Full PL011 with interrupts, DMA, buffering
//! - **network_driver**: Complete network stack
//! - **storage_driver**: Block device drivers
//!
//! # Component Composition (Compile-Time)
//!
//! Kernel components are composed at compile-time via cargo features,
//! mirroring the framework's runtime component spawning model:
//!
//! ```
//! // Framework (runtime spawning)
//! spawner.spawn_component_with_device(
//!     ComponentConfig {
//!         name: "uart_driver",
//!         device: DeviceId::Serial { port: 0 },
//!     }
//! );
//!
//! // Kernel (compile-time composition)
//! #[cfg(feature = "console-pl011")]
//! static CONSOLE: Pl011Console = Pl011Console::new(Pl011Config {
//!     mmio_base: 0x9000000,
//! });
//! ```

pub mod console;

// Future chapters:
// pub mod timer;   // Chapter 3: Timer for scheduling
// pub mod irq;     // Chapter 3: IRQ controller
