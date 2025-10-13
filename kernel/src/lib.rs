//! KaaL Rust Microkernel
//!
//! A native Rust microkernel inspired by seL4's architecture and security model.
//!
//! # Architecture
//!
//! The kernel is organized into the following modules:
//! - `boot`: Boot sequence and initialization
//! - `arch`: Architecture-specific code (ARM64)
//! - `components`: Minimal kernel components (console, timer, irq)
//! - `debug`: Debug output and logging
//!
//! # Chapter 1: Bare Metal Boot & Early Init
//!
//! This is the initial implementation focusing on:
//! - Booting on QEMU ARM64 virt platform
//! - Component-based console (compile-time composition)
//! - Parsing device tree (DTB)
//! - Printing kernel banner

#![no_std]
#![feature(naked_functions)]
#![feature(asm_const)]
#![feature(alloc_error_handler)]

// Enable the alloc crate for heap allocation
extern crate alloc;

// Module declarations
pub mod arch;
pub mod boot;
pub mod components;
pub mod debug;
pub mod config;
pub mod memory;
pub mod objects;
pub mod syscall;
