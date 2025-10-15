//! Process management
//!
//! Utilities for process creation and management (placeholder for future implementation).

/// Process ID type
pub type Pid = usize;

/// Process handle
///
/// Represents a running process in the system.
pub struct Process {
    pid: Pid,
}

impl Process {
    /// Get the process ID
    pub fn pid(&self) -> Pid {
        self.pid
    }
}

// TODO: Implement process creation when SYS_PROCESS_CREATE is fully functional
