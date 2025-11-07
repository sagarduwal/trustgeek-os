#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_bootloader_esp_idf::esp_app_desc;
use esp_hal::xtensa_lx_rt::entry;

mod bootloader_info;
mod gpio;
mod i2c;
mod interrupts; // Provides DefaultHandler for interrupt stubs
mod ml;
mod oled;
mod uart;

use bootloader_info::{get_app_info, get_partition_info, partition_info_to_tuples};
use gpio::init_gpio;
use i2c::init_i2c;
use uart::init_uart;
esp_app_desc!(); // defaults are fine

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    init_uart();
    esp_println::println!("Initializing system...");

    let esp_hal::peripherals::Peripherals {
        I2C0,
        GPIO2,
        GPIO21,
        GPIO22,
        ..
    } = peripherals;

    let mut led = init_gpio(GPIO2);

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

        let app_info = get_app_info();
        let _ = display.show_app_info(app_info.name, app_info.version);

        // let partitions = get_partition_info();
        // let partition_rows = partition_info_to_tuples(&partitions);
        // let _ = display.show_partition_info(&partition_rows);
    }

    // Initialize ML inference
    ml::init();

    esp_println::println!("hello from no_std on ESP32!");

    // Display app info via UART
    let app_info = get_app_info();
    esp_println::println!("App: {} v{}", app_info.name, app_info.version);

    // Display partition info via UART
    let partitions = get_partition_info();
    esp_println::println!("Partitions:");
    for part in &partitions {
        esp_println::println!("  {}: {}", part.name, part.size);
    }

    // Main kernel loop
    loop {
        // Toggle LED
        led.toggle();

        // Run ML inference
        ml::run_inference();

        // Simple delay (blocking)
        // In a real OS, you'd use a timer interrupt or scheduler
        for _ in 0..1_000_000 {
            core::hint::spin_loop();
        }
    }
}
