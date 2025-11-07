#![no_std]
#![no_main]

use core::fmt::Write as _;
use esp_backtrace as _;
use esp_bootloader_esp_idf::esp_app_desc;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    xtensa_lx_rt::entry,
};
use heapless::{String, Vec};

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

const VISIBLE_LINES: usize = 5;

type PartitionLine = String<32>;
type PartitionLines = Vec<PartitionLine, 4>;
type VisibleLines<'a> = Vec<&'a str, VISIBLE_LINES>;

fn line_at<'a>(
    index: usize,
    prefix: &'a [&'a str],
    partitions: &'a [PartitionLine],
    suffix: &'a [&'a str],
) -> Option<&'a str> {
    if index < prefix.len() {
        Some(prefix[index])
    } else if index < prefix.len() + partitions.len() {
        Some(partitions[index - prefix.len()].as_str())
    } else {
        let suffix_index = index - prefix.len() - partitions.len();
        suffix.get(suffix_index).copied()
    }
}

fn fill_visible_lines<'a>(
    buffer: &mut VisibleLines<'a>,
    prefix: &'a [&'a str],
    partitions: &'a [PartitionLine],
    suffix: &'a [&'a str],
    start: usize,
    total_lines: usize,
) {
    buffer.clear();

    if total_lines == 0 || start >= total_lines {
        return;
    }

    let end = core::cmp::min(start + VISIBLE_LINES, total_lines);
    for index in start..end {
        if let Some(line) = line_at(index, prefix, partitions, suffix) {
            let _ = buffer.push(line);
        }
    }
}

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    init_uart();
    esp_println::println!("Initializing system...");

    let esp_hal::peripherals::Peripherals {
        I2C0,
        GPIO2,
        GPIO18,
        GPIO19,
        GPIO21,
        GPIO22,
        ..
    } = peripherals;

    let mut led = init_gpio(GPIO2);
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
    }

    let partitions = get_partition_info();
    let partition_rows = partition_info_to_tuples(&partitions);

    let prefix_lines = [
        "TrustG33k OS",
        "-----------",
        "App Name",
        app_info.name,
        "Version",
        app_info.version,
        "Partitions",
    ];
    let suffix_lines = ["Use buttons", "UP/DOWN to scroll"];

    let mut partition_lines: PartitionLines = PartitionLines::new();
    for &(name, size) in &partition_rows {
        let mut line: PartitionLine = PartitionLine::new();
        let _ = write!(line, "{}: {}", name, size);
        let _ = partition_lines.push(line);
    }

    let total_lines = prefix_lines.len() + partition_lines.len() + suffix_lines.len();

    let mut scroll_offset = 0usize;
    let mut visible_lines: VisibleLines = VisibleLines::new();
    fill_visible_lines(
        &mut visible_lines,
        &prefix_lines,
        &partition_lines,
        &suffix_lines,
        scroll_offset,
        total_lines,
    );

    if let Some(display) = oled_display.as_mut() {
        let _ = display.show_lines(&visible_lines);
    }
    let mut up_pressed = false;
    let mut down_pressed = false;

    // Initialize ML inference
    ml::init();

    esp_println::println!("hello from no_std on ESP32!");
    esp_println::println!("App: {} v{}", app_info.name, app_info.version);
    esp_println::println!("Partitions:");
    for part in &partitions {
        esp_println::println!("  {}: {}", part.name, part.size);
    }

    // Main kernel loop
    loop {
        // Toggle LED
        // led.toggle();

        // Handle scroll button inputs (active low due to pull-ups)
        let mut display_updated = false;

        if scroll_up.is_low() {
            if !up_pressed {
                if scroll_offset > 0 {
                    scroll_offset -= 1;
                    display_updated = true;
                }
                up_pressed = true;
            }
        } else {
            up_pressed = false;
        }

        if scroll_down.is_low() {
            if !down_pressed {
                if scroll_offset + VISIBLE_LINES < total_lines {
                    scroll_offset += 1;
                    display_updated = true;
                }
                down_pressed = true;
            }
        } else {
            down_pressed = false;
        }

        if display_updated {
            fill_visible_lines(
                &mut visible_lines,
                &prefix_lines,
                &partition_lines,
                &suffix_lines,
                scroll_offset,
                total_lines,
            );
            if let Some(display) = oled_display.as_mut() {
                let _ = display.show_lines(&visible_lines);
            }
            esp_println::println!("Scroll offset: {}", scroll_offset);
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
