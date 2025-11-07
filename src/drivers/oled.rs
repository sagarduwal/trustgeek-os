use core::cell::RefCell;

use critical_section::{with, Mutex};

use crate::drivers::i2c::I2cHandle;
use crate::oled::OledDisplay;

use super::{DriverCell, DriverError, DriverHandle};

static OLED_DRIVER: DriverCell<OledDisplay> = Mutex::new(RefCell::new(None));

pub type OledHandle = DriverHandle<OledDisplay>;

pub fn init_oled(i2c: &I2cHandle) -> Result<OledHandle, DriverError> {
    let bus = i2c.take().ok_or(DriverError::NotReady)?;

    if with(|cs| OLED_DRIVER.borrow_ref(cs).is_some()) {
        let _ = i2c.replace(bus);
        return Err(DriverError::AlreadyInitialized);
    }

    let display = match OledDisplay::new(bus) {
        Ok(display) => display,
        Err(err) => {
            esp_println::println!("OLED driver creation failed: {:?}", err);
            return Err(DriverError::InitFailed("oled init"));
        }
    };

    with(|cs| {
        let mut cell = OLED_DRIVER.borrow_ref_mut(cs);
        *cell = Some(display);
    });

    Ok(OledHandle::new(&OLED_DRIVER))
}
