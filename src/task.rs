//! Cooperative task implementations used by the kernel scheduler.

use core::fmt::Write as _;

use esp_hal::gpio::Input;
use esp_println::println;
use heapless::{String, Vec};

use crate::{
    bootloader_info::{AppInfo, PartitionInfo},
    drivers::{gpio::LedHandle, oled::OledHandle},
    ml,
    scheduler::{Task, TaskCommand, TaskContext, TaskPriority},
};

/// Number of lines visible on the OLED at once.
const VISIBLE_LINES: usize = 5;
const MAX_MENU_ITEMS: usize = 16;

type MenuLabel = String<32>;
type MenuItems = Vec<MenuItem, MAX_MENU_ITEMS>;

#[derive(Clone)]
struct MenuItem {
    label: MenuLabel,
    feature: MenuFeature,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum MenuFeature {
    About,
    AppInfo,
    Version,
    Partition(usize),
    Diagnostics,
    ToggleLed,
    RunMl,
    Instructions,
}

/// UI task rendering boot information and handling scroll buttons.
pub struct UiTask {
    display: Option<OledHandle>,
    scroll_up: Input<'static>,
    scroll_down: Input<'static>,
    menu_items: MenuItems,
    selected_index: usize,
    view_offset: usize,
    last_logged_index: Option<usize>,
    up_pressed: bool,
    down_pressed: bool,
    dirty: bool,
}

impl UiTask {
    pub fn new(
        display: Option<OledHandle>,
        scroll_up: Input<'static>,
        scroll_down: Input<'static>,
        app_info: AppInfo,
        partitions: [PartitionInfo; 4],
    ) -> Self {
        let mut menu_items: MenuItems = MenuItems::new();

        let mut label = MenuLabel::new();
        let _ = label.push_str("About TrustG33k OS");
        let _ = menu_items.push(MenuItem {
            label,
            feature: MenuFeature::About,
        });

        let mut label = MenuLabel::new();
        let _ = write!(label, "App: {}", app_info.name);
        let _ = menu_items.push(MenuItem {
            label,
            feature: MenuFeature::AppInfo,
        });

        let mut label = MenuLabel::new();
        let _ = write!(label, "Version: {}", app_info.version);
        let _ = menu_items.push(MenuItem {
            label,
            feature: MenuFeature::Version,
        });

        for (idx, part) in partitions.iter().enumerate() {
            let mut label = MenuLabel::new();
            let _ = write!(label, "{}: {}", part.name, part.size);
            let _ = menu_items.push(MenuItem {
                label,
                feature: MenuFeature::Partition(idx),
            });
        }

        let mut label = MenuLabel::new();
        let _ = label.push_str("Run ML Inference");
        let _ = menu_items.push(MenuItem {
            label,
            feature: MenuFeature::RunMl,
        });

        let mut label = MenuLabel::new();
        let _ = label.push_str("Toggle LED");
        let _ = menu_items.push(MenuItem {
            label,
            feature: MenuFeature::ToggleLed,
        });

        let mut label = MenuLabel::new();
        let _ = label.push_str("Diagnostics");
        let _ = menu_items.push(MenuItem {
            label,
            feature: MenuFeature::Diagnostics,
        });

        let mut label = MenuLabel::new();
        let _ = label.push_str("Use UP/DOWN to select");
        let _ = menu_items.push(MenuItem {
            label,
            feature: MenuFeature::Instructions,
        });

        Self {
            display,
            scroll_up,
            scroll_down,
            menu_items,
            selected_index: 0,
            view_offset: 0,
            last_logged_index: None,
            up_pressed: false,
            down_pressed: false,
            dirty: true,
        }
    }

    fn total_items(&self) -> usize {
        self.menu_items.len()
    }

    fn render(&mut self) {
        let total = self.total_items();
        if total == 0 {
            return;
        }

        let start = self.view_offset;
        let end = core::cmp::min(start + VISIBLE_LINES, total);

        let mut display_lines: Vec<MenuLabel, VISIBLE_LINES> = Vec::new();
        for idx in start..end {
            let item = &self.menu_items[idx];
            let mut line = MenuLabel::new();
            if idx == self.selected_index {
                let _ = write!(line, "> {}", item.label);
            } else {
                let _ = write!(line, "  {}", item.label);
            }
            let _ = display_lines.push(line);
        }

        let mut line_refs: Vec<&str, VISIBLE_LINES> = Vec::new();
        for line in &display_lines {
            let _ = line_refs.push(line.as_str());
        }

        if let Some(handle) = self.display.as_ref() {
            let _ = handle.try_with(|display| display.show_lines(line_refs.as_slice()));
        }

        if self.last_logged_index != Some(self.selected_index) {
            if let Some(item) = self.menu_items.get(self.selected_index) {
                println!("Selected option: {}", item.label.as_str());
                match item.feature {
                    MenuFeature::About => println!("Feature: About screen"),
                    MenuFeature::AppInfo => println!("Feature: Application information"),
                    MenuFeature::Version => println!("Feature: Firmware version"),
                    MenuFeature::Partition(idx) => println!("Feature: Partition entry #{}", idx),
                    MenuFeature::Diagnostics => println!("Feature: Diagnostics"),
                    MenuFeature::ToggleLed => println!("Feature: Toggle LED"),
                    MenuFeature::RunMl => println!("Feature: Run ML"),
                    MenuFeature::Instructions => println!("Feature: Instructions"),
                }
            }
            self.last_logged_index = Some(self.selected_index);
        }
    }

    fn handle_input(&mut self) {
        let total = self.total_items();
        if total == 0 {
            return;
        }

        if self.scroll_up.is_low() {
            if !self.up_pressed && self.selected_index > 0 {
                self.selected_index -= 1;
                self.dirty = true;
            }
            self.up_pressed = true;
        } else {
            self.up_pressed = false;
        }

        if self.scroll_down.is_low() {
            if !self.down_pressed && self.selected_index + 1 < total {
                self.selected_index += 1;
                self.dirty = true;
            }
            self.down_pressed = true;
        } else {
            self.down_pressed = false;
        }

        if self.dirty {
            if self.selected_index < self.view_offset {
                self.view_offset = self.selected_index;
            } else if self.selected_index >= self.view_offset + VISIBLE_LINES {
                self.view_offset = self.selected_index + 1 - VISIBLE_LINES;
            }
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
    led: LedHandle,
    state: bool,
}

impl LedTask {
    pub fn new(led: LedHandle) -> Self {
        Self { led, state: false }
    }
}

impl Task for LedTask {
    fn name(&self) -> &'static str {
        "led"
    }

    fn poll(&mut self, _ctx: &mut TaskContext) -> TaskCommand {
        self.state = !self.state;
        let _ = self.led.try_with(|led| {
            if self.state {
                let _ = led.set_high();
            } else {
                let _ = led.set_low();
            }
        });
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

