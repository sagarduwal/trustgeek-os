//! System Timer and Tick Management
//!
//! Provides system-wide timekeeping using the ESP32 timer peripheral (TIMG0).
//! The timer generates a periodic interrupt that increments a global tick
//! counter, forming the "heartbeat" of the cooperative OS.

use core::cell::RefCell;
use core::sync::atomic::{AtomicU32, Ordering};

use critical_section::Mutex;
use esp_hal::{
    interrupt::{self, IsrCallback, Priority},
    peripherals::{Interrupt, TIMG0},
    timer::{timg::TimerGroup, PeriodicTimer},
    time::Duration,
    Blocking,
};

/// System tick frequency in Hz (1000 Hz = 1ms per tick)
pub const TICK_FREQUENCY_HZ: u32 = 1_000;

/// Global system tick counter (32-bit → wraps after ~49 days at 1 kHz)
static SYSTEM_TICKS: AtomicU32 = AtomicU32::new(0);

/// Stored hardware timer instance so we can acknowledge interrupts.
/// Wrapped in a critical-section Mutex to allow safe access from ISRs.
type HwTimer = PeriodicTimer<'static, Blocking>;
static TIMER: Mutex<RefCell<Option<HwTimer>>> = Mutex::new(RefCell::new(None));

/// Initialize the system timer (TIMG0 timer0) to generate periodic interrupts.
///
/// # Safety
/// Must be called exactly once during system startup, before the scheduler starts.
pub unsafe fn init(timg0: TIMG0<'static>) -> Result<(), &'static str> {
    critical_section::with(|cs| {
        if TIMER.borrow_ref(cs).is_some() {
            return Err("System timer already initialized");
        }

        // Create the timer group driver
        let tg0 = TimerGroup::new(timg0);
        let mut timer0 = PeriodicTimer::new(tg0.timer0);

        // Configure auto-reload period based on desired tick frequency
        let period = Duration::from_micros((1_000_000u32 / TICK_FREQUENCY_HZ) as u64);
        timer0
            .start(period)
            .map_err(|_| "Failed to start system timer")?;
        timer0.listen();

        // Route the timer interrupt to our handler at priority level 1
        unsafe {
            interrupt::bind_interrupt(
                Interrupt::TG0_T0_LEVEL,
                IsrCallback::new(timer_isr_trampoline),
            );
        }
        interrupt::enable(Interrupt::TG0_T0_LEVEL, Priority::Priority1)
            .map_err(|_| "Failed to enable timer interrupt")?;

        // Store timer so ISR can clear the interrupt flag
        TIMER.borrow_ref_mut(cs).replace(timer0);

        Ok(())
    })
}

/// Returns the number of ticks since system startup.
pub fn get_ticks() -> u32 {
    SYSTEM_TICKS.load(Ordering::Relaxed)
}

/// Converts milliseconds to ticks, rounding down.
pub fn ms_to_ticks(ms: u32) -> u32 {
    ms.saturating_mul(TICK_FREQUENCY_HZ) / 1_000
}

/// ISR trampoline registered with the HAL interrupt controller.
extern "C" fn timer_isr_trampoline() {
    // Increment tick counter first to minimize latency for waiting tasks
    SYSTEM_TICKS.fetch_add(1, Ordering::Relaxed);

    // Acknowledge hardware interrupt
    critical_section::with(|cs| {
        if let Some(timer) = TIMER.borrow_ref_mut(cs).as_mut() {
            timer.clear_interrupt();
        }
    });
}

/// Manual tick increment helper (used for testing without hardware timer).
#[allow(dead_code)]
pub unsafe fn force_tick() {
    timer_isr_trampoline();
}

