//! Cooperative task implementations used by the kernel scheduler.

use core::fmt::Write as _;

use esp_hal::gpio::{Input, Output};
use heapless::{String, Vec};

use crate::{
    bootloader_info::{AppInfo, PartitionInfo},
    ml,
    oled::OledDisplay,
    scheduler::{Task, TaskCommand, TaskContext, TaskPriority},
};

/// Number of lines visible on the OLED at once.
const VISIBLE_LINES: usize = 5;

/// Heapless string used to render partition rows.
type PartitionLine = String<32>;
/// Stored partition rows.
type PartitionLines = Vec<PartitionLine, 4>;

/// UI task rendering boot information and handling scroll buttons.
pub struct UiTask {
    display: Option<OledDisplay>,
    scroll_up: Input<'static>,
    scroll_down: Input<'static>,
    prefix_lines: [&'static str; 7],
    suffix_lines: [&'static str; 2],
    partition_lines: PartitionLines,
    scroll_offset: usize,
    up_pressed: bool,
    down_pressed: bool,
    dirty: bool,
}

impl UiTask {
    pub fn new(
        display: Option<OledDisplay>,
        scroll_up: Input<'static>,
        scroll_down: Input<'static>,
        app_info: AppInfo,
        partitions: [PartitionInfo; 4],
    ) -> Self {
        let mut partition_lines: PartitionLines = PartitionLines::new();
        for part in partitions.iter() {
            let mut line: PartitionLine = PartitionLine::new();
            let _ = write!(line, "{}: {}", part.name, part.size);
            let _ = partition_lines.push(line);
        }

        Self {
            display,
            scroll_up,
            scroll_down,
            prefix_lines: [
                "TrustG33k OS",
                "-----------",
                "App Name",
                app_info.name,
                "Version",
                app_info.version,
                "Partitions",
            ],
            suffix_lines: ["Use buttons", "UP/DOWN to scroll"],
            partition_lines,
            scroll_offset: 0,
            up_pressed: false,
            down_pressed: false,
            dirty: true,
        }
    }

    fn total_lines(&self) -> usize {
        self.prefix_lines.len() + self.partition_lines.len() + self.suffix_lines.len()
    }

    fn render(&mut self) {
        let total = self.total_lines();
        let prefix_len = self.prefix_lines.len();
        let partition_len = self.partition_lines.len();

        let start = self.scroll_offset;
        let end = core::cmp::min(start + VISIBLE_LINES, total);

        let mut lines: Vec<&str, VISIBLE_LINES> = Vec::new();
        for idx in start..end {
            let line = if idx < prefix_len {
                self.prefix_lines[idx]
            } else if idx < prefix_len + partition_len {
                let offset = idx - prefix_len;
                self.partition_lines[offset].as_str()
            } else {
                let offset = idx - prefix_len - partition_len;
                self.suffix_lines[offset]
            };
            let _ = lines.push(line);
        }

        if let Some(display) = self.display.as_mut() {
            let _ = display.show_lines(&lines);
        }
    }

    fn handle_input(&mut self) {
        let total = self.total_lines();

        if self.scroll_up.is_low() {
            if !self.up_pressed && self.scroll_offset > 0 {
                self.scroll_offset -= 1;
                self.dirty = true;
            }
            self.up_pressed = true;
        } else {
            self.up_pressed = false;
        }

        if self.scroll_down.is_low() {
            if !self.down_pressed && self.scroll_offset + VISIBLE_LINES < total {
                self.scroll_offset += 1;
                self.dirty = true;
            }
            self.down_pressed = true;
        } else {
            self.down_pressed = false;
        }
    }
}

impl Task for UiTask {
    fn name(&self) -> &'static str {
        "ui"
    }

    fn priority(&self) -> TaskPriority {
        TaskPriority::High
    }

    fn poll(&mut self, _ctx: &mut TaskContext) -> TaskCommand {
        self.handle_input();

        if self.dirty {
            self.render();
            self.dirty = false;
        }

        TaskCommand::SleepMs(50)
    }
}

/// Simple LED heartbeat task.
pub struct LedTask {
    led: Output<'static>,
    state: bool,
}

impl LedTask {
    pub fn new(led: Output<'static>) -> Self {
        Self { led, state: false }
    }
}

impl Task for LedTask {
    fn name(&self) -> &'static str {
        "led"
    }

    fn poll(&mut self, _ctx: &mut TaskContext) -> TaskCommand {
        self.state = !self.state;
        if self.state {
            let _ = self.led.set_high();
        } else {
            let _ = self.led.set_low();
        }
        TaskCommand::SleepMs(500)
    }
}

/// Periodic ML inference task.
pub struct MlTask;

impl MlTask {
    pub const fn new() -> Self {
        MlTask
    }
}

impl Task for MlTask {
    fn name(&self) -> &'static str {
        "ml"
    }

    fn poll(&mut self, _ctx: &mut TaskContext) -> TaskCommand {
        ml::run_inference();
        TaskCommand::SleepMs(100)
    }
}

