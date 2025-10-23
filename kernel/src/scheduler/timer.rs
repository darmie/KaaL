//! Timer-Based Preemption
//!
//! This module implements time-slicing preemption using the ARM Generic Timer.
//! It configures the timer to fire periodic interrupts, allowing the scheduler
//! to preempt long-running threads and enforce fairness.
//!
//! ## ARM Generic Timer
//!
//! ARM provides a Generic Timer that can generate periodic interrupts:
//! - EL1 Physical Timer (used for kernel scheduling)
//! - EL1 Virtual Timer (for virtualization)
//! - EL0 Physical/Virtual Timers (for userspace)
//!
//! We use the **EL1 Physical Timer** for scheduling interrupts.
//!
//! ## Timer Registers
//!
//! - `CNTFRQ_EL0`: Timer frequency (Hz)
//! - `CNTP_TVAL_EL0`: Timer value (counts down to 0)
//! - `CNTP_CTL_EL0`: Timer control (enable/disable, interrupt status)
//! - `CNTPCT_EL0`: Current counter value
//!
//! ## Preemption Strategy
//!
//! 1. Configure timer to fire every TIMESLICE_MS milliseconds
//! 2. On timer interrupt:
//!    - Decrement current thread's timeslice
//!    - If timeslice == 0:
//!      - Reset timeslice
//!      - Call yield_current() to switch threads
//! 3. Higher-priority threads always preempt lower-priority ones

use core::arch::asm;

/// Timeslice duration in milliseconds
///
/// Each thread gets this much CPU time before being preempted.
/// Typical values: 1-10ms
pub const TIMESLICE_MS: u32 = 5;

/// Timeslice in timer ticks
///
/// This is calculated based on timer frequency and TIMESLICE_MS.
/// Will be initialized at boot.
static mut TIMESLICE_TICKS: u64 = 0;

/// Timer frequency in Hz
///
/// Read from CNTFRQ_EL0 register at boot.
static mut TIMER_FREQ_HZ: u64 = 0;

/// Initialize the scheduler timer
///
/// Configures the ARM Generic Timer to fire periodic interrupts for preemption.
///
/// # Safety
///
/// - Must be called once during boot
/// - Must be called with interrupts disabled
/// - IRQ handler must be set up before enabling timer
pub unsafe fn init() {
    // Read timer frequency from CNTFRQ_EL0
    let freq: u64;
    asm!("mrs {}, cntfrq_el0", out(reg) freq);
    TIMER_FREQ_HZ = freq;

    // Calculate timeslice in ticks
    // timeslice_ticks = (freq_hz * timeslice_ms) / 1000
    TIMESLICE_TICKS = (freq * (TIMESLICE_MS as u64)) / 1000;

    crate::kprintln!("[timer] Timer frequency: {} Hz", freq);
    crate::kprintln!("[timer] Timeslice: {} ms ({} ticks)",
                     TIMESLICE_MS, TIMESLICE_TICKS);

    // Enable timer
    start_timer();
}

/// Start the preemption timer
///
/// Configures the timer to fire after TIMESLICE_TICKS.
///
/// # Safety
///
/// - Timer must be initialized (init() called)
pub unsafe fn start_timer() {
    // Set timer value (counts down from this value)
    // Use VIRTUAL timer (cntv) instead of physical (cntp) so it fires at EL0
    asm!(
        "msr cntv_tval_el0, {}",
        in(reg) TIMESLICE_TICKS
    );

    // Enable timer and unmask interrupt
    // CNTV_CTL_EL0: bit 0 = enable, bit 1 = imask (0 = not masked)
    asm!(
        "msr cntv_ctl_el0, {val}",
        val = in(reg) 0b01u64  // Enable=1, IMask=0
    );
}

/// Stop the preemption timer
///
/// Disables the timer interrupt.
pub unsafe fn stop_timer() {
    // Disable timer (clear enable bit)
    asm!(
        "msr cntv_ctl_el0, {val}",
        val = in(reg) 0b00u64  // Enable=0
    );
}

/// Timer interrupt handler
///
/// Called when the timer fires. This is invoked from the IRQ exception handler.
///
/// # Safety
///
/// - Must be called from IRQ exception context
/// - Scheduler must be initialized
pub unsafe fn timer_tick() {
    // Acknowledge timer interrupt by reloading the timer value
    start_timer();

    // Get current thread
    let current = crate::scheduler::current_thread();
    if current.is_null() {
        return; // No current thread (shouldn't happen)
    }

    let current_tcb = &mut *current;

    // Decrement timeslice
    let timeslice = current_tcb.time_slice();
    if timeslice > 0 {
        current_tcb.set_time_slice(timeslice - 1);
    }

    // If timeslice expired, preempt
    if timeslice <= 1 {
        // Reset timeslice for next run
        current_tcb.refill_time_slice();

        crate::kprintln!("[timer] Timeslice expired for TCB {}, preempting",
                         current_tcb.tid());

        // Preempt current thread
        crate::scheduler::yield_current();
    }
}

/// Get timer frequency in Hz
#[inline]
pub fn timer_frequency() -> u64 {
    unsafe { TIMER_FREQ_HZ }
}

/// Get timeslice in ticks
#[inline]
pub fn timeslice_ticks() -> u64 {
    unsafe { TIMESLICE_TICKS }
}

/// Read current timer counter value
///
/// Returns the current value of the physical counter.
pub fn read_counter() -> u64 {
    let counter: u64;
    unsafe {
        asm!("mrs {}, cntpct_el0", out(reg) counter);
    }
    counter
}

/// Get elapsed time since last call (in microseconds)
///
/// Useful for profiling and timing measurements.
pub fn elapsed_us(start: u64) -> u64 {
    let end = read_counter();
    let ticks = end.wrapping_sub(start);
    let freq = timer_frequency();

    // Convert ticks to microseconds: (ticks * 1_000_000) / freq
    (ticks * 1_000_000) / freq
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_constants() {
        assert!(TIMESLICE_MS > 0);
        assert!(TIMESLICE_MS <= 100); // Reasonable range
    }
}
