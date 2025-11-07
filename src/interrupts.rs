//! Interrupt Management Helpers
//!
//! Thin wrapper around `esp-hal` interrupt primitives plus a simple
//! critical-section guard used across the kernel.

use critical_section::{self, RestoreState};
use esp_hal::{
    interrupt::{self, Error as InterruptError, IsrCallback, Priority},
    peripherals::Interrupt,
    system::Cpu,
};

/// Interrupt priority abstraction used by the kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptPriority {
    Level1,
    Level2,
    Level3,
}

impl From<InterruptPriority> for Priority {
    fn from(value: InterruptPriority) -> Self {
        match value {
            InterruptPriority::Level1 => Priority::Priority1,
            InterruptPriority::Level2 => Priority::Priority2,
            InterruptPriority::Level3 => Priority::Priority3,
        }
    }
}

/// Register and enable a peripheral interrupt handler.
///
/// # Safety
/// The handler must be an `extern "C"` function that follows ISR safety rules.
#[allow(dead_code)]
pub unsafe fn register_handler(
    interrupt: Interrupt,
    handler: extern "C" fn(),
    priority: InterruptPriority,
) -> Result<(), InterruptError> {
    interrupt::bind_interrupt(interrupt, IsrCallback::new(handler));
    interrupt::enable(interrupt, priority.into())
}

/// Disable a previously enabled peripheral interrupt.
#[allow(dead_code)]
pub fn disable_interrupt(interrupt: Interrupt) {
    interrupt::disable(Cpu::current(), interrupt);
}

/// RAII guard representing an acquired critical section.
pub struct CriticalSectionGuard {
    state: RestoreState,
}

impl CriticalSectionGuard {
    /// Enter a critical section, returning a guard that will restore the
    /// previous interrupt state when dropped.
    #[allow(dead_code)]
    pub fn new() -> Self {
        let state = unsafe { critical_section::acquire() };
        Self { state }
    }
}

impl Drop for CriticalSectionGuard {
    fn drop(&mut self) {
        unsafe { critical_section::release(self.state) };
    }
}

/// Enter a critical section using RAII semantics.
#[allow(dead_code)]
pub fn enter_critical() -> CriticalSectionGuard {
    CriticalSectionGuard::new()
}

/// Execute the provided closure with interrupts disabled.
#[allow(dead_code)]
pub fn with_critical<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    critical_section::with(|_| f())
}
