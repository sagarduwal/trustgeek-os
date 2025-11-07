//! UART initialization for println! via esp-println
//!
//! esp-println automatically uses UART0, so minimal setup is needed

/// Initialize UART for println! output
///
/// Note: esp-println handles UART initialization automatically.
/// This function is kept for API consistency but does nothing.
pub fn init_uart() {
    // esp-println automatically initializes UART0
    // No additional setup needed unless you want custom baud rate, etc.
}
