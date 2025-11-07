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
use core::fmt::Write;
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use gpio::init_gpio;
use heapless::{String, Vec};
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
        ADC1,
        GPIO2,
        GPIO21,
        GPIO22,
        GPIO34,
        ..
    } = peripherals;

    let mut led = init_gpio(GPIO2);

    let mut adc_config = AdcConfig::new();
    let mut potentiometer = adc_config.enable_pin(GPIO34, Attenuation::_11dB);
    let mut adc = Adc::new(ADC1, adc_config);

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

    let app_info = get_app_info();
    let partitions = get_partition_info();
    let partition_rows = partition_info_to_tuples(&partitions);

    let mut scroll_lines: Vec<String<32>, 16> = Vec::new();

    let mut line = String::<32>::new();
    let _ = line.push_str("TrustG33k OS");
    let _ = scroll_lines.push(line);

    let mut line = String::<32>::new();
    let _ = write!(line, "App: {}", app_info.name);
    let _ = scroll_lines.push(line);

    let mut line = String::<32>::new();
    let _ = write!(line, "Version: {}", app_info.version);
    let _ = scroll_lines.push(line);

    let mut line = String::<32>::new();
    let _ = line.push_str("");
    let _ = scroll_lines.push(line);

    let mut line = String::<32>::new();
    let _ = line.push_str("Partitions");
    let _ = scroll_lines.push(line);

    for (name, size) in partition_rows.iter() {
        let mut line = String::<32>::new();
        let _ = write!(line, "{} {}", name, size);
        let _ = scroll_lines.push(line);
    }

    let max_scroll = scroll_lines
        .len()
        .saturating_sub(oled::OledDisplay::VISIBLE_LINES);
    let mut last_scroll_offset = usize::MAX;

    if let Some(display) = oled_display.as_mut() {
        let _ = display.show_boot_progress("Starting...");

        let _ = display.show_app_info(app_info.name, app_info.version);
        let _ = display.show_scrollable(scroll_lines.as_slice(), 0);
    }

    // Initialize ML inference
    ml::init();

    esp_println::println!("hello from no_std on ESP32!");

    // Display app info via UART
    esp_println::println!("App: {} v{}", app_info.name, app_info.version);

    // Display partition info via UART
    esp_println::println!("Partitions:");
    for part in &partitions {
        esp_println::println!("  {}: {}", part.name, part.size);
    }

    // Main kernel loop
    loop {
        // Toggle LED
        led.toggle();

        if let Some(display) = oled_display.as_mut() {
            let pot_reading =
                nb::block!(adc.read_oneshot(&mut potentiometer)).unwrap_or(0) as u32;
            let percent = (pot_reading * 100) / 4095;
            let step = percent / 10;
            let target_offset = if max_scroll == 0 {
                0
            } else {
                (step as usize * max_scroll) / 10
            };

            if target_offset != last_scroll_offset {
                last_scroll_offset = target_offset;
                let _ = display.show_scrollable(scroll_lines.as_slice(), target_offset);
            }
        }

        // Run ML inference
        ml::run_inference();

        // Simple delay (blocking)
        // In a real OS, you'd use a timer interrupt or scheduler
        for _ in 0..1_000_000 {
            core::hint::spin_loop();
        }
    }
}
