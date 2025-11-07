//! I2C initialization for the SSD1306 OLED display.
//!
//! Configures `I2C0` on the ESP32 with GPIO21 (SDA) and GPIO22 (SCL) and returns
//! a blocking I2C driver that implements the `embedded-hal` 1.0 traits.

use esp_hal::{
    i2c::master::{Config, ConfigError, I2c},
    peripherals::{GPIO21, GPIO22, I2C0},
    time::Rate,
    Blocking,
};

/// Convenience alias for the blocking ESP HAL I2C driver used throughout the
/// project.
pub type I2cBus = I2c<'static, Blocking>;

/// Initialize `I2C0` for OLED display communication.
///
/// * SDA: `GPIO21`
/// * SCL: `GPIO22`
/// * Clock: 400 kHz (fast mode)
pub fn init_i2c(
    i2c0: I2C0<'static>,
    sda: GPIO21<'static>,
    scl: GPIO22<'static>,
) -> Result<I2cBus, ConfigError> {
    let config = Config::default().with_frequency(Rate::from_khz(400));

    let i2c = I2c::new(i2c0, config)?.with_sda(sda).with_scl(scl);

    Ok(i2c)
}
