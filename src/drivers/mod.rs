use core::{cell::RefCell, marker::PhantomData};

use critical_section::Mutex;
use esp_println::println;

pub type DriverCell<T> = Mutex<RefCell<Option<T>>>;

#[derive(Debug)]
pub enum DriverError {
    AlreadyInitialized,
    NotReady,
    InitFailed(&'static str),
}

pub struct DriverHandle<T: 'static> {
    cell: &'static DriverCell<T>,
    _marker: PhantomData<T>,
}

impl<T: 'static> Copy for DriverHandle<T> {}

impl<T: 'static> Clone for DriverHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> DriverHandle<T> {
    pub const fn new(cell: &'static DriverCell<T>) -> Self {
        Self {
            cell,
            _marker: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub fn is_ready(&self) -> bool {
        critical_section::with(|cs| self.cell.borrow_ref(cs).is_some())
    }

    pub fn try_with<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        critical_section::with(|cs| self.cell.borrow_ref_mut(cs).as_mut().map(f))
    }

    pub fn take(&self) -> Option<T> {
        critical_section::with(|cs| {
            let result = self.cell.borrow_ref_mut(cs).take();
            if result.is_none() {
                println!("Driver handle take failed");
            }
            result
        })
    }

    pub fn replace(&self, value: T) -> Option<T> {
        critical_section::with(|cs| self.cell.borrow_ref_mut(cs).replace(value))
    }
}

pub mod gpio;
pub mod i2c;
pub mod oled;
pub mod uart;
