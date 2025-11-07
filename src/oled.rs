//! SSD1306 OLED display driver wrapper
//!
//! Provides high-level helpers for displaying bootloader info, app info, and
//! partition information using the `ssd1306` crate in buffered graphics mode.

use display_interface::DisplayError;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{
    mode::{BufferedGraphicsMode, DisplayConfig},
    prelude::I2CInterface,
    rotation::DisplayRotation,
    size::DisplaySize128x64,
    I2CDisplayInterface, Ssd1306,
};

use crate::{frames, i2c::I2cBus};

/// Convenience result type for OLED operations.
pub type OledResult<T> = Result<T, DisplayError>;

type DisplayDriver =
    Ssd1306<I2CInterface<I2cBus>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

/// SSD1306 OLED display wrapper.
pub struct OledDisplay {
    display: DisplayDriver,
    text_style: MonoTextStyle<'static, BinaryColor>,
}

impl OledDisplay {
    const LINE_SPACING: i32 = 12;
    const PARTITION_SIZE_COLUMN: i32 = 72;

    /// Initialize the OLED display in buffered graphics mode.
    pub fn new(i2c: I2cBus) -> OledResult<Self> {
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();

        display.init()?;
        display.clear_buffer();
        display.flush()?;

        Ok(Self {
            display,
            text_style: MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        })
    }

    /// Clear the framebuffer and the panel.
    pub fn clear(&mut self) -> OledResult<()> {
        self.display.clear_buffer();
        self.display.flush()
    }

    fn render_lines<'a>(&mut self, lines: impl IntoIterator<Item = &'a str>) -> OledResult<()> {
        self.display.clear_buffer();

        for (index, line) in lines.into_iter().enumerate() {
            let position = Point::new(0, (index as i32) * Self::LINE_SPACING);
            let _ = Text::with_baseline(line, position, self.text_style, Baseline::Top)
                .draw(&mut self.display);
        }

        self.display.flush()
    }

    /// Display a collection of text lines, starting from the top of the panel.
    pub fn show_lines(&mut self, lines: &[&str]) -> OledResult<()> {
        self.render_lines(lines.iter().copied())
    }

    /// Play a boot animation using pre-rendered frame data.
    pub fn play_boot_animation<F>(&mut self, mut delay_ms: F) -> OledResult<()>
    where
        F: FnMut(u32),
    {
        let stride = frames::FRAME_STRIDE;

        if stride == 0 || stride * frames::NUM_FRAMES > frames::FRAMES.len() {
            return Ok(());
        }

        let width = frames::FRAME_W as u8;
        let height = frames::FRAME_H as u8;

        if let Err(err) = self.display.set_draw_area((0, 0), (width, height)) {
            return Err(err);
        }

        for frame_index in 0..frames::NUM_FRAMES {
            let start = frame_index * stride;
            let end = start + stride;
            let frame = &frames::FRAMES[start..end];

            if let Err(err) = self.display.draw(frame) {
                return Err(err);
            }
            delay_ms(33);
        }

        delay_ms(300);
        Ok(())
    }

    /// Display a boot progress message.
    pub fn show_boot_progress(&mut self, message: &str) -> OledResult<()> {
        self.render_lines(["Booting ESP32", message].into_iter())
    }

    /// Display application name and version.
    pub fn show_app_info(&mut self, app_name: &str, app_version: &str) -> OledResult<()> {
        self.render_lines(["TrustG33k", app_name, "Version", app_version].into_iter())
    }

    /// Display partition information as a simple table.
    pub fn show_partition_info(&mut self, partitions: &[(&str, &str); 4]) -> OledResult<()> {
        self.display.clear_buffer();

        let _ = Text::with_baseline("Partitions", Point::zero(), self.text_style, Baseline::Top)
            .draw(&mut self.display);

        for (index, (name, size)) in partitions.iter().enumerate() {
            let y = ((index + 1) as i32) * Self::LINE_SPACING;
            let _ = Text::with_baseline(name, Point::new(0, y), self.text_style, Baseline::Top)
                .draw(&mut self.display);
            let _ = Text::with_baseline(
                size,
                Point::new(Self::PARTITION_SIZE_COLUMN, y),
                self.text_style,
                Baseline::Top,
            )
            .draw(&mut self.display);
        }

        self.display.flush()
    }
}
