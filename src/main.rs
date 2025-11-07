#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::mem::MaybeUninit;
use esp_backtrace as _;
use esp_bootloader_esp_idf::esp_app_desc;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    xtensa_lx_rt::entry,
};

mod bootloader_info;
mod drivers;
mod frames;
mod heap;
mod interrupts; // Provides DefaultHandler for interrupt stubs
mod ml;
mod oled;
mod scheduler;
mod stack;
mod syscall;
mod task;
mod timer;

use bootloader_info::{get_app_info, get_partition_info};
use drivers::{gpio, i2c, oled as oled_driver, uart, DriverError};
use scheduler::Scheduler;
use task::{LedTask, MlTask, UiTask};
esp_app_desc!(); // defaults are fine

static mut SCHEDULER: Scheduler = Scheduler::new();
static mut UI_TASK: MaybeUninit<UiTask> = MaybeUninit::uninit();
static mut LED_TASK: MaybeUninit<LedTask> = MaybeUninit::uninit();
static mut ML_TASK: MaybeUninit<MlTask> = MaybeUninit::uninit();

fn spin_delay_ms(ms: u32) {
    const INNER_LOOPS: u32 = 25_000;
    for _ in 0..ms {
        for _ in 0..INNER_LOOPS {
            core::hint::spin_loop();
        }
    }
}

fn log_driver_error(name: &str, err: DriverError) {
    match err {
        DriverError::InitFailed(reason) => esp_println::println!("{} init failed: {}", name, reason),
        other => esp_println::println!("{} init failed: {:?}", name, other),
    }
}

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    unsafe { heap::init(); }

    let esp_hal::peripherals::Peripherals {
        I2C0,
        GPIO2,
        GPIO5,
        GPIO18,
        GPIO19,
        GPIO21,
        GPIO22,
        TIMG0,
        ..
    } = peripherals;

    // Initialize system tick timer (1 kHz)
    if let Err(err) = unsafe { timer::init(TIMG0) } {
        esp_println::println!("Timer init failed: {}", err);
    }

    if let Err(err) = uart::init_uart() {
        log_driver_error("UART", err);
    }

    let led_handle = match gpio::init_led(GPIO2) {
        Ok(handle) => Some(handle),
        Err(err) => {
            log_driver_error("LED", err);
            None
        }
    };

    let scroll_up = Input::new(GPIO18, InputConfig::default().with_pull(Pull::Up));
    let scroll_down = Input::new(GPIO19, InputConfig::default().with_pull(Pull::Up));
    let select_button = Input::new(GPIO5, InputConfig::default().with_pull(Pull::Up));

    esp_println::println!("Initializing I2C0 for OLED display...");
    let i2c_handle = match i2c::init_i2c0(I2C0, GPIO21, GPIO22) {
        Ok(handle) => {
            esp_println::println!("I2C0 initialised");
            Some(handle)
        }
        Err(err) => {
            esp_println::println!("I2C initialization failed: {:?}", err);
            None
        }
    };

    let oled_handle: Option<oled_driver::OledHandle> = if let Some(ref handle) = i2c_handle {
        match oled_driver::init_oled(handle) {
            Ok(display_handle) => {
                esp_println::println!("OLED display initialized");
                Some(display_handle)
            }
            Err(err) => {
                log_driver_error("OLED", err);
                None
            }
        }
    } else {
        None
    };

    if let Some(handle) = &oled_handle {
        let _ = handle.try_with(|display| display.show_boot_progress("Starting..."));
    }

    let app_info = get_app_info();
    if let Some(handle) = &oled_handle {
        let _ = handle.try_with(|display| display.show_app_info(app_info.name, app_info.version));
        let _ = handle.try_with(|display| display.play_boot_animation(spin_delay_ms));
    }

    let partitions = get_partition_info();

    ml::init();

    esp_println::println!("hello from no_std on ESP32!");
    esp_println::println!("App: {} v{}", app_info.name, app_info.version);
    esp_println::println!("Partitions:");
    for part in &partitions {
        esp_println::println!("  {}: {}", part.name, part.size);
    }

    #[allow(static_mut_refs)]
    unsafe {
        let scheduler = &mut SCHEDULER;

        let ui_display = oled_handle.clone();
        let ui_task: &mut dyn scheduler::Task = UI_TASK.write(UiTask::new(
            ui_display,
            scroll_up,
            scroll_down,
            select_button,
            app_info.name,
            app_info.version,
            partitions,
        ));
        let _ = scheduler.spawn(ui_task);

        if let Some(handle) = led_handle {
            let led_task: &mut dyn scheduler::Task = LED_TASK.write(LedTask::new(handle));
            let _ = scheduler.spawn(led_task);
        }

        let ml_task: &mut dyn scheduler::Task = ML_TASK.write(MlTask::new());
        let _ = scheduler.spawn(ml_task);
    }

    loop {
        #[allow(static_mut_refs)]
        unsafe {
            SCHEDULER.run_ready();
        }
    }
}
