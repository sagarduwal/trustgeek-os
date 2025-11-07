//! Minimal GPIO helpers

use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    peripherals::Peripherals,
};

/// Initialize GPIO and return LED pin (GPIO2)
pub fn init_gpio(peripherals: Peripherals) -> Output<'static> {
    Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default())
}

