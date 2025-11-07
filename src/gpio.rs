//! Minimal GPIO helpers

use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    peripherals::GPIO2,
};

/// Initialize GPIO and return LED pin (GPIO2)
pub fn init_gpio(gpio2: GPIO2<'static>) -> Output<'static> {
    Output::new(gpio2, Level::Low, OutputConfig::default())
}
