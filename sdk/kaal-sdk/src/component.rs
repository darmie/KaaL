//! Component development utilities
//!
//! Provides patterns and helpers for building system components (drivers, services, apps).

use crate::{Result, capability::Notification};

/// Component lifecycle trait
///
/// Implement this trait for your driver or service to get standardized lifecycle management.
///
/// # Example
/// ```no_run
/// struct MyDriver {
///     // driver state
/// }
///
/// impl Component for MyDriver {
///     fn init() -> Result<Self> {
///         // Initialize hardware, allocate resources
///         Ok(Self { /* ... */ })
///     }
///
///     fn run(&mut self) -> ! {
///         loop {
///             // Event loop
///         }
///     }
/// }
/// ```
pub trait Component: Sized {
    /// Initialize the component
    ///
    /// Called once during component startup. Allocate resources, map memory, etc.
    fn init() -> Result<Self>;

    /// Run the component's main loop
    ///
    /// This should be the component's event loop. Never returns.
    fn run(&mut self) -> !;

    /// Start the component (convenience method)
    ///
    /// Combines init + run for simple components.
    fn start() -> ! {
        match Self::init() {
            Ok(mut component) => component.run(),
            Err(_) => {
                // Component failed to initialize
                loop {
                    unsafe { core::arch::asm!("wfi") }
                }
            }
        }
    }
}

/// Event types that components can handle
#[derive(Debug, Clone, Copy)]
pub enum Event {
    /// IPC message received
    IpcMessage,
    /// Hardware interrupt
    Interrupt,
    /// Timeout expired
    Timeout,
    /// Notification signaled
    Notification { signals: u64 },
}

/// Component type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// Device driver (has hardware access)
    Driver,
    /// System service (no hardware, provides IPC services)
    Service,
    /// Application (consumes services)
    Application,
}

/// Component metadata
///
/// Describes a component's characteristics and requirements.
pub struct ComponentMetadata {
    /// Component name
    pub name: &'static str,
    /// Component type
    pub component_type: ComponentType,
    /// Version string
    pub version: &'static str,
    /// Required capabilities (for validation)
    pub required_caps: &'static [&'static str],
}

impl ComponentMetadata {
    /// Create new component metadata
    pub const fn new(
        name: &'static str,
        component_type: ComponentType,
        version: &'static str,
    ) -> Self {
        Self {
            name,
            component_type,
            version,
            required_caps: &[],
        }
    }

    /// Set required capabilities
    pub const fn with_caps(mut self, caps: &'static [&'static str]) -> Self {
        self.required_caps = caps;
        self
    }
}

/// Macro to define component metadata
///
/// # Example
/// ```
/// component_metadata! {
///     name: "serial_driver",
///     type: Driver,
///     version: "0.1.0",
///     capabilities: ["memory_map", "interrupt"],
/// }
/// ```
#[macro_export]
macro_rules! component_metadata {
    (
        name: $name:expr,
        type: $type:ident,
        version: $version:expr,
        $(capabilities: [$($cap:expr),*],)?
    ) => {
        #[no_mangle]
        #[link_section = ".component_metadata"]
        pub static COMPONENT_METADATA: $crate::component::ComponentMetadata =
            $crate::component::ComponentMetadata::new(
                $name,
                $crate::component::ComponentType::$type,
                $version,
            )$(
                .with_caps(&[$($cap),*])
            )?;
    };
}

/// Complete component declaration macro
///
/// This macro generates everything needed for a component:
/// - Component metadata
/// - _start entry point
/// - Panic handler
/// - Global allocator
#[macro_export]
macro_rules! component {
    (
        name: $name:expr,
        type: $type:ident,
        version: $version:expr,
        $(capabilities: [$($cap:expr),*],)?
        impl: $component_type:ty
    ) => {

        // Generate metadata
        #[no_mangle]
        #[link_section = ".component_metadata"]
        pub static COMPONENT_METADATA: $crate::component::ComponentMetadata =
            $crate::component::ComponentMetadata::new(
                $name,
                $crate::component::ComponentType::$type,
                $version,
            )$(
                .with_caps(&[$($cap),*])
            )?;

        // Generate _start entry point
        #[no_mangle]
        pub extern "C" fn _start() -> ! {
            <$component_type as $crate::component::Component>::start()
        }

        // Generate panic handler
        #[panic_handler]
        fn panic(info: &core::panic::PanicInfo) -> ! {
            $crate::syscall::print("[");
            $crate::syscall::print($name);
            $crate::syscall::print("] PANIC");

            if let Some(location) = info.location() {
                $crate::syscall::print(" at ");
                $crate::syscall::print(location.file());
            }

            $crate::syscall::print("\n");

            loop {
                unsafe {
                    core::arch::asm!("wfi");
                }
            }
        }
    };
}

/// Device driver base structure
///
/// Provides common functionality for device drivers.
pub struct DriverBase {
    /// IRQ notification
    pub irq_notification: Option<Notification>,
    /// Device name
    pub name: &'static str,
}

impl DriverBase {
    /// Create a new driver base
    pub fn new(name: &'static str) -> Result<Self> {
        Ok(Self {
            irq_notification: None,
            name,
        })
    }

    /// Register for IRQ notifications
    pub fn register_irq(&mut self) -> Result<()> {
        let notification = Notification::create()?;
        self.irq_notification = Some(notification);
        Ok(())
    }

    /// Wait for IRQ
    pub fn wait_irq(&self) -> Result<u64> {
        match &self.irq_notification {
            Some(notification) => notification.wait(),
            None => Err(crate::Error::InvalidParameter),
        }
    }
}

/// Service base structure
///
/// Provides common functionality for system services.
pub struct ServiceBase {
    /// Service name
    pub name: &'static str,
    /// Number of clients connected
    pub client_count: usize,
}

impl ServiceBase {
    /// Create a new service base
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            client_count: 0,
        }
    }

    /// Register a new client
    pub fn register_client(&mut self) {
        self.client_count += 1;
    }

    /// Unregister a client
    pub fn unregister_client(&mut self) {
        if self.client_count > 0 {
            self.client_count -= 1;
        }
    }
}

/// Component discovery support
///
/// Components can be discovered by root-task or system_init through:
/// 1. Static registry (components known at compile time)
/// 2. Component metadata section (embedded in binary)
/// 3. Manifest file (components.toml)

/// Component binary descriptor
///
/// Used by component loaders to spawn components with proper configuration.
#[derive(Debug)]
pub struct ComponentBinary {
    /// Component name
    pub name: &'static str,
    /// Component type
    pub component_type: ComponentType,
    /// Version
    pub version: &'static str,
    /// Required capabilities
    pub required_caps: &'static [&'static str],
    /// Priority (0-255, higher = more important)
    pub priority: u8,
    /// Should autostart at boot
    pub autostart: bool,
}

impl ComponentBinary {
    /// Create a new component binary descriptor
    pub const fn new(
        name: &'static str,
        component_type: ComponentType,
        version: &'static str,
    ) -> Self {
        Self {
            name,
            component_type,
            version,
            required_caps: &[],
            priority: 100,
            autostart: false,
        }
    }

    /// Set required capabilities
    pub const fn with_caps(mut self, caps: &'static [&'static str]) -> Self {
        self.required_caps = caps;
        self
    }

    /// Set priority
    pub const fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set autostart
    pub const fn with_autostart(mut self, autostart: bool) -> Self {
        self.autostart = autostart;
        self
    }
}

/// Macro to register a component for discovery
///
/// This places the component descriptor in a special section that can be
/// read by the component loader.
///
/// # Example
/// ```
/// register_component! {
///     name: "serial_driver",
///     type: Driver,
///     version: "0.1.0",
///     priority: 200,
///     autostart: true,
///     capabilities: ["memory_map:0x09000000", "interrupt:33"],
/// }
/// ```
#[macro_export]
macro_rules! register_component {
    (
        name: $name:expr,
        type: $type:ident,
        version: $version:expr,
        priority: $priority:expr,
        autostart: $autostart:expr,
        $(capabilities: [$($cap:expr),*],)?
    ) => {
        #[no_mangle]
        #[link_section = ".component_registry"]
        pub static COMPONENT_BINARY: $crate::component::ComponentBinary =
            $crate::component::ComponentBinary::new(
                $name,
                $crate::component::ComponentType::$type,
                $version,
            )
            .with_priority($priority)
            .with_autostart($autostart)
            $(
                .with_caps(&[$($cap),*])
            )?;
    };
}
