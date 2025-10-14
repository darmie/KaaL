//! Device Driver Development Kit (DDDK) - Procedural macros for driver development
//!
//! # Purpose
//! Reduces driver code from 500+ lines to ~50 lines through declarative
//! macros that generate boilerplate for resource management, initialization,
//! and interrupt handling.
//!
//! # Integration Points
//! - Depends on: Capability Broker (device resources)
//! - Provides to: Device drivers
//! - Generates: Driver registration, MMIO mapping, DMA allocation, IRQ setup
//!
//! # Architecture
//! Procedural macros that parse driver attributes and generate:
//! - Device probing and initialization
//! - MMIO region mapping
//! - DMA pool allocation
//! - IRQ handler registration
//! - Driver database registration
//!
//! # Testing Strategy
//! - Unit tests: Macro expansion (using trybuild)
//! - Integration tests: Generated code compilation
//! - Examples: Complete driver implementations

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Expr, Fields, Lit, Meta};

/// Device identifier parsed from attributes
#[derive(Debug)]
enum DeviceIdent {
    Pci { vendor: u16, device: u16 },
    Platform { name: String },
    Serial { port: u8 },
}

/// Resource requirements parsed from attributes
#[derive(Debug, Default)]
struct ResourceSpec {
    mmio: Option<String>,
    irq: Option<String>,
    dma_size: Option<String>,
}

/// Parse PCI device attributes
fn parse_pci_attr(attr: &Attribute) -> Option<DeviceIdent> {
    if let Meta::List(meta_list) = &attr.meta {
        let mut vendor = None;
        let mut device = None;

        // Parse nested meta items
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("vendor") {
                if let Ok(Expr::Lit(expr_lit)) = meta.value()?.parse::<Expr>() {
                    if let Lit::Int(lit_int) = &expr_lit.lit {
                        vendor = lit_int.base10_parse::<u16>().ok();
                    }
                }
            } else if meta.path.is_ident("device") {
                if let Ok(Expr::Lit(expr_lit)) = meta.value()?.parse::<Expr>() {
                    if let Lit::Int(lit_int) = &expr_lit.lit {
                        device = lit_int.base10_parse::<u16>().ok();
                    }
                }
            }
            Ok(())
        });

        if let (Some(v), Some(d)) = (vendor, device) {
            return Some(DeviceIdent::Pci {
                vendor: v,
                device: d,
            });
        }
    }
    None
}

/// Parse platform device attributes
fn parse_platform_attr(attr: &Attribute) -> Option<DeviceIdent> {
    if let Meta::List(_) = &attr.meta {
        let mut name = None;

        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                if let Ok(Expr::Lit(expr_lit)) = meta.value()?.parse::<Expr>() {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        name = Some(lit_str.value());
                    }
                }
            }
            Ok(())
        });

        name.map(|n| DeviceIdent::Platform { name: n })
    } else {
        None
    }
}

/// Parse resource attributes
fn parse_resource_attr(attr: &Attribute) -> ResourceSpec {
    let mut spec = ResourceSpec::default();

    if let Meta::List(_) = &attr.meta {
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("mmio") {
                if let Ok(Expr::Lit(expr_lit)) = meta.value()?.parse::<Expr>() {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        spec.mmio = Some(lit_str.value());
                    }
                }
            } else if meta.path.is_ident("irq") {
                if let Ok(Expr::Lit(expr_lit)) = meta.value()?.parse::<Expr>() {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        spec.irq = Some(lit_str.value());
                    }
                }
            } else if meta.path.is_ident("dma") {
                if let Ok(Expr::Lit(expr_lit)) = meta.value()?.parse::<Expr>() {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        spec.dma_size = Some(lit_str.value());
                    }
                }
            }
            Ok(())
        });
    }

    spec
}

/// Derive macro for device drivers
///
/// # Example
/// ```ignore
/// #[derive(Driver)]
/// #[pci(vendor = 0x8086, device = 0x100E)]
/// #[resources(mmio = "bar0", irq = "auto", dma = "4MB")]
/// pub struct E1000Driver {
///     #[mmio]
///     regs: &'static mut E1000Registers,
/// }
/// ```
#[proc_macro_derive(Driver, attributes(pci, platform, resources, mmio, dma_ring))]
pub fn derive_driver(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Parse device identifier attributes
    let mut device_id = None;
    for attr in &input.attrs {
        if attr.path().is_ident("pci") {
            device_id = parse_pci_attr(attr);
        } else if attr.path().is_ident("platform") {
            device_id = parse_platform_attr(attr);
        }
    }

    // Parse resource attributes
    let mut resources = ResourceSpec::default();
    for attr in &input.attrs {
        if attr.path().is_ident("resources") {
            resources = parse_resource_attr(attr);
        }
    }

    // Generate device ID expression
    let device_id_expr = match device_id {
        Some(DeviceIdent::Pci { vendor, device }) => {
            quote! {
                cap_broker::DeviceId::Pci {
                    vendor: #vendor,
                    device: #device,
                }
            }
        }
        Some(DeviceIdent::Platform { ref name }) => {
            quote! {
                cap_broker::DeviceId::Platform {
                    name: #name,
                }
            }
        }
        Some(DeviceIdent::Serial { port }) => {
            quote! {
                cap_broker::DeviceId::Serial {
                    port: #port,
                }
            }
        }
        None => {
            // No device ID specified, generate error
            return TokenStream::from(quote! {
                compile_error!("Driver must specify device identifier using #[pci(...)] or #[platform(...)]");
            });
        }
    };

    // Generate probe function
    let expanded = quote! {
        impl #name {
            /// Auto-generated probe function
            ///
            /// Requests device resources from the capability broker and
            /// initializes the driver with MMIO regions, IRQ handlers, and DMA pools.
            pub fn probe(broker: &mut dyn cap_broker::CapabilityBroker) -> Result<Self, DriverError> {
                use cap_broker::CapabilityBroker;

                // Request device bundle from capability broker
                let device_id = #device_id_expr;
                let bundle = broker.request_device(device_id)
                    .map_err(|e| DriverError::ResourceAllocation(format!("{:?}", e)))?;

                // TODO PHASE 1.5: Initialize driver with device resources
                // - Map MMIO regions
                // - Register IRQ handler
                // - Allocate DMA buffers

                Err(DriverError::NotImplemented)
            }
        }

        /// Auto-generated driver metadata
        impl DriverMetadata for #name {
            fn device_id() -> cap_broker::DeviceId {
                #device_id_expr
            }

            fn driver_name() -> &'static str {
                stringify!(#name)
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for driver implementation
///
/// # Example
/// ```ignore
/// #[driver_impl]
/// impl E1000Driver {
///     #[init]
///     fn initialize(&mut self) -> Result<()> {
///         // Driver-specific initialization
///     }
///
///     #[interrupt]
///     fn handle_interrupt(&mut self) {
///         // Interrupt handling
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn driver_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // TODO: Parse and transform impl block
    // For now, pass through unchanged
    item
}
