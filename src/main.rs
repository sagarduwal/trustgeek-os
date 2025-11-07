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
mod frames;
mod gpio;
mod heap;
mod i2c;
mod interrupts; // Provides DefaultHandler for interrupt stubs
mod ml;
mod oled;
mod scheduler;
mod stack;
mod task;
mod timer;
mod uart;

use bootloader_info::{get_app_info, get_partition_info};
use gpio::init_gpio;
use i2c::init_i2c;
use scheduler::Scheduler;
use task::{LedTask, MlTask, UiTask};
use uart::init_uart;
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

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    unsafe { heap::init(); }

    init_uart();
    esp_println::println!("Initializing system...");

    let esp_hal::peripherals::Peripherals {
        I2C0,
        GPIO2,
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

    let led = init_gpio(GPIO2);
    let scroll_up = Input::new(GPIO18, InputConfig::default().with_pull(Pull::Up));
    let scroll_down = Input::new(GPIO19, InputConfig::default().with_pull(Pull::Up));

    esp_println::println!("Initializing I2C0 for OLED display...");
    let mut oled_display = match init_i2c(I2C0, GPIO21, GPIO22) {
        Ok(i2c) => match oled::OledDisplay::new(i2c) {
            Ok(display) => {
                esp_println::println!("OLED display initialized");
                Some(display)
            }
            Err(err) => {
                esp_println::println!("OLED init failed: {:?}", err);
                None
            }
        },
        Err(err) => {
            esp_println::println!("I2C initialization failed: {:?}", err);
            None
        }
    };

    if let Some(display) = oled_display.as_mut() {
        let _ = display.show_boot_progress("Starting...");
    }

    let app_info = get_app_info();
    if let Some(display) = oled_display.as_mut() {
        let _ = display.show_app_info(app_info.name, app_info.version);
        let _ = display.play_boot_animation(spin_delay_ms);
    }

    let partitions = get_partition_info();

    ml::init();

    esp_println::println!("hello from no_std on ESP32!");
    esp_println::println!("App: {} v{}", app_info.name, app_info.version);
    esp_println::println!("Partitions:");
    for part in &partitions {
        esp_println::println!("  {}: {}", part.name, part.size);
    }

    // Create tasks and register them with the scheduler
    #[allow(static_mut_refs)]
    let ui_task_ref: &mut dyn scheduler::Task = unsafe {
        UI_TASK.write(UiTask::new(oled_display, scroll_up, scroll_down, app_info, partitions))
    };
    #[allow(static_mut_refs)]
    let led_task_ref: &mut dyn scheduler::Task = unsafe { LED_TASK.write(LedTask::new(led)) };
    #[allow(static_mut_refs)]
    let ml_task_ref: &mut dyn scheduler::Task = unsafe { ML_TASK.write(MlTask::new()) };

    #[allow(static_mut_refs)]
    unsafe {
        let scheduler = &mut SCHEDULER;
        let _ = scheduler.spawn(ui_task_ref);
        let _ = scheduler.spawn(led_task_ref);
        let _ = scheduler.spawn(ml_task_ref);
    }

    loop {
        #[allow(static_mut_refs)]
        unsafe {
            SCHEDULER.run_ready();
        }
    }
}
