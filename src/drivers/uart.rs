use core::cell::RefCell;

use critical_section::{with, Mutex};

use super::{DriverCell, DriverError, DriverHandle};

static UART0_DRIVER: DriverCell<()> = Mutex::new(RefCell::new(None));

pub type UartHandle = DriverHandle<()>;

pub fn init_uart() -> Result<UartHandle, DriverError> {
    with(|cs| {
        let mut cell = UART0_DRIVER.borrow_ref_mut(cs);
        if cell.is_some() {
            return Err(DriverError::AlreadyInitialized);
        }
        *cell = Some(());
        Ok(())
    })?;
    Ok(UartHandle::new(&UART0_DRIVER))
}
