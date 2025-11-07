use core::cell::RefCell;

use critical_section::{with, Mutex};
use esp_hal::i2c::master::{Config, I2c};
use esp_hal::peripherals::{GPIO21, GPIO22, I2C0};
use esp_hal::time::Rate;
use esp_hal::Blocking;

use super::{DriverCell, DriverError, DriverHandle};

pub type I2cBus = I2c<'static, Blocking>;

static I2C0_DRIVER: DriverCell<I2cBus> = Mutex::new(RefCell::new(None));

pub type I2cHandle = DriverHandle<I2cBus>;

pub fn init_i2c0(
    i2c0: I2C0<'static>,
    sda: GPIO21<'static>,
    scl: GPIO22<'static>,
) -> Result<I2cHandle, DriverError> {
    with(|cs| {
        let mut cell = I2C0_DRIVER.borrow_ref_mut(cs);
        if cell.is_some() {
            return Err(DriverError::AlreadyInitialized);
        }
        let config = Config::default().with_frequency(Rate::from_khz(400));
        let bus = I2c::new(i2c0, config)
            .map_err(|_| DriverError::InitFailed("i2c init"))?
            .with_sda(sda)
            .with_scl(scl);
        *cell = Some(bus);
        Ok(())
    })?;
    Ok(I2cHandle::new(&I2C0_DRIVER))
}
