//! SSD1306 OLED display driver wrapper
//!
//! Provides high-level helpers for displaying bootloader info, app info, and
//! partition information using the `ssd1306` crate in buffered graphics mode.

use display_interface::DisplayError;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_9X18, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Ellipse, Line, PrimitiveStyle},
    text::{Alignment, Baseline, Text},
};
use ssd1306::{
    mode::{BufferedGraphicsMode, DisplayConfig},
    prelude::I2CInterface,
    rotation::DisplayRotation,
    size::DisplaySize128x64,
    I2CDisplayInterface, Ssd1306,
};

use crate::i2c::I2cBus;

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

    /// Play the Concept C "TrustG33k" horizon animation sequence.
    pub fn play_trustg33k_animation<F>(&mut self, mut delay_ms: F) -> OledResult<()>
    where
        F: FnMut(u32),
    {
        let size = self.display.size();
        let width = size.width as i32;
        let height = size.height as i32;
        let center_x = width / 2;
        let horizon_y = 42;

        let big_style = MonoTextStyleBuilder::new()
            .font(&FONT_9X18)
            .text_color(BinaryColor::On)
            .build();
        let sparkle_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        const LABEL: &str = "TrustG33k";
        const FINAL_TEXT_Y: i32 = 26;
        let start_text_y: i32 = height + 10;

        const RISE_MS: u32 = 900;
        const RIPPLE_MS: u32 = 1_500;
        const SPARKLE_START_MS: u32 = 1_100;
        const SPARKLE_END_MS: u32 = 1_400;
        const FRAME_MS: u32 = 33;
        const FINAL_HOLD_MS: u32 = 400;

        let mut t_ms: u32 = 0;

        loop {
            self.display.clear_buffer();

            // Horizon line
            let _ = Line::new(Point::new(0, horizon_y), Point::new(width - 1, horizon_y))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(&mut self.display);

            // Rising text with ease-out cubic motion
            let rise_fraction = core::cmp::min(t_ms, RISE_MS) as f32 / RISE_MS as f32;
            let eased = Self::ease_out_cubic(rise_fraction);
            let text_y = Self::lerp(start_text_y as f32, FINAL_TEXT_Y as f32, eased) as i32;
            let _ = Text::with_alignment(
                LABEL,
                Point::new(center_x, text_y),
                big_style,
                Alignment::Center,
            )
            .draw(&mut self.display);

            // Ripples expanding across the horizon
            let ripple_fraction = core::cmp::min(t_ms, RIPPLE_MS) as f32 / RIPPLE_MS as f32;
            const BASE_RX: f32 = 6.0;
            const MAX_RX: f32 = 40.0;
            for ring in 0..3 {
                let phase = ripple_fraction + ring as f32 * 0.12;
                if phase > 1.0 {
                    continue;
                }

                let radius_x = BASE_RX + (MAX_RX - BASE_RX) * phase;
                let radius_y = radius_x * 0.35;
                let stroke_width = if radius_x < 16.0 { 2 } else { 1 };
                let skip = (phase * 6.0) as u32;
                if skip >= 5 && ring == 2 {
                    continue;
                }

                let width_px = (radius_x * 2.0).max(1.0) as u32;
                let height_px = (radius_y * 2.0).max(1.0) as u32;

                let top_left = Point::new(center_x - radius_x as i32, horizon_y - 1);
                let _ = Ellipse::new(top_left, Size::new(width_px, height_px))
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, stroke_width))
                    .draw(&mut self.display);
            }

            // Sparkle accent near the finale
            if (SPARKLE_START_MS..=SPARKLE_END_MS).contains(&t_ms) {
                let sparkle_x = center_x + 22;
                let sparkle_y = FINAL_TEXT_Y - 10;
                let cross_style = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

                let _ = Line::new(Point::new(sparkle_x - 2, sparkle_y), Point::new(sparkle_x + 2, sparkle_y))
                    .into_styled(cross_style)
                    .draw(&mut self.display);
                let _ = Line::new(Point::new(sparkle_x, sparkle_y - 2), Point::new(sparkle_x, sparkle_y + 2))
                    .into_styled(cross_style)
                    .draw(&mut self.display);

                let _ = Text::with_alignment(
                    "*",
                    Point::new(sparkle_x + 6, sparkle_y + 3),
                    sparkle_style,
                    Alignment::Center,
                )
                .draw(&mut self.display);
            }

            self.display.flush()?;

            if t_ms >= RIPPLE_MS {
                delay_ms(FINAL_HOLD_MS);
                break;
            }

            delay_ms(FRAME_MS);
            t_ms = t_ms.saturating_add(FRAME_MS);
        }

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

    fn ease_out_cubic(t: f32) -> f32 {
        let inv = 1.0 - t;
        1.0 - inv * inv * inv
    }

    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }
}
