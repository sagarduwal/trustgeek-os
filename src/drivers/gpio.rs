use core::cell::RefCell;

use critical_section::{with, Mutex};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::peripherals::GPIO2;

use super::{DriverCell, DriverError, DriverHandle};

static LED_DRIVER: DriverCell<Output<'static>> = Mutex::new(RefCell::new(None));

pub type LedHandle = DriverHandle<Output<'static>>;

pub fn init_led(gpio2: GPIO2<'static>) -> Result<LedHandle, DriverError> {
    with(|cs| {
        let mut cell = LED_DRIVER.borrow_ref_mut(cs);
        if cell.is_some() {
            return Err(DriverError::AlreadyInitialized);
        }
        *cell = Some(Output::new(gpio2, Level::Low, OutputConfig::default()));
        Ok(())
    })?;
    Ok(LedHandle::new(&LED_DRIVER))
}
