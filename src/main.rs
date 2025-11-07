#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::xtensa_lx_rt::entry;
use esp_bootloader_esp_idf::esp_app_desc;

mod gpio;
mod uart;
mod ml;
mod interrupts; // Provides DefaultHandler for interrupt stubs

use gpio::init_gpio;
use uart::init_uart;
esp_app_desc!(); // defaults are fine

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    
    // Initialize GPIO (takes GPIO and IO_MUX peripherals)
    let mut led = init_gpio(peripherals);
    
    // Initialize UART for println! (esp-println handles this automatically)
    init_uart();
    
    // Initialize ML inference
    ml::init();
    
    esp_println::println!("hello from no_std on ESP32!");
    
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
