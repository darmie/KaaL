//! KaaL Rust Microkernel
//!
//! A pure-Rust seL4-compatible microkernel for ARM64.
//!
//! # Architecture
//!
//! The kernel is organized into the following modules:
//! - `boot`: Boot sequence and initialization
//! - `arch`: Architecture-specific code (ARM64)
//! - `debug`: Debug output and logging
//!
//! # Chapter 1: Bare Metal Boot & Early Init
//!
//! This is the initial implementation focusing on:
//! - Booting on QEMU ARM64 virt platform
//! - Initializing serial UART output
//! - Parsing device tree (DTB)
//! - Printing "Hello from KaaL Kernel!"

#![no_std]
#![feature(naked_functions)]
#![feature(asm_const)]

// Module declarations
pub mod arch;
pub mod boot;
pub mod debug;
