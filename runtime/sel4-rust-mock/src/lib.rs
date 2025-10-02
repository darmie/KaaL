//! ⚠️  MOCK seL4 Rust Bindings for Phase 1 Development
//!
//! # WARNING: This is NOT the real seL4 Rust library!
//!
//! This provides Rust-friendly wrappers around the mock seL4-sys.
//!
//! ## Phase 2 TODO: Replace with Real seL4 Rust bindings
//!
//! See: https://github.com/seL4/rust-sel4

#![no_std]

pub use sel4_sys;

/// TODO PHASE 2: Replace with real seL4 Rust types
pub mod types {
    pub use sel4_sys::*;
}

/// TODO PHASE 2: Replace with real capability management
pub mod cap {
    use sel4_sys::*;

    pub struct Capability {
        cptr: seL4_CPtr,
    }

    impl Capability {
        pub fn new(cptr: seL4_CPtr) -> Self {
            Self { cptr }
        }

        pub fn cptr(&self) -> seL4_CPtr {
            self.cptr
        }
    }
}

/// TODO PHASE 2: Replace with real IPC implementation
pub mod ipc {
    use sel4_sys::*;

    pub fn send(dest: seL4_CPtr, msg: seL4_MessageInfo) {
        unsafe { seL4_Send(dest, msg) }
    }

    pub fn call(dest: seL4_CPtr, msg: seL4_MessageInfo) -> seL4_MessageInfo {
        unsafe { seL4_Call(dest, msg) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mock_exists() {
        // Just ensure the mock compiles
    }
}
