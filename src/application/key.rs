use std::cell::{Cell, RefCell};

use super::{Page, Rect, SCANCODE_LABELS};
use crate::Rgb;

#[derive(Clone, Debug)]
pub struct Key {
    // Logical position (row, column)
    pub(crate) logical: (u8, u8),
    // Logical name (something like K01, where 0 is the row and 1 is the column)
    pub(crate) logical_name: String,
    // Physical position and size
    pub(crate) physical: Rect,
    // Physical key name (what is printed on the keycap)
    pub(crate) physical_name: String,
    // Electrical mapping (output, input)
    pub(crate) electrical: (u8, u8),
    // Electrical name (output, input)
    pub(crate) electrical_name: String,
    /// LED indexes
    pub(crate) leds: Vec<u8>,
    /// LED name
    pub(crate) led_name: String,
    // Key is currently pressed
    pub(crate) pressed: Cell<bool>,
    // Currently loaded scancodes and their names
    pub(crate) scancodes: RefCell<Vec<(u16, String)>>,
    // Background color
    pub(crate) background_color: Rgb,
    // Foreground color
    pub(crate) foreground_color: Rgb,
}

impl Key {
    pub fn get_label(&self, page: Page) -> String {
        let scancodes = self.scancodes.borrow();
        match page {
            Page::Layer1 | Page::Layer2 | Page::Layer3 | Page::Layer4 => {
                let scancode_name = &scancodes[page.layer().unwrap()].1;
                SCANCODE_LABELS
                    .get(scancode_name)
                    .unwrap_or(scancode_name)
                    .into()
            }
            Page::Keycaps => self.physical_name.clone(),
            Page::Logical => self.logical_name.clone(),
            Page::Electrical => self.electrical_name.clone(),
            Page::Leds => self.led_name.clone(),
        }
    }
}
