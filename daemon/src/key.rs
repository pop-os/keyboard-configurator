use std::cell::{Cell, RefCell};

use crate::{Rect, Rgb};

#[derive(Clone, Debug)]
pub struct Key {
    // Logical position (row, column)
    pub logical: (u8, u8),
    // Logical name (something like K01, where 0 is the row and 1 is the column)
    pub logical_name: String,
    // Physical position and size
    pub physical: Rect,
    // Physical key name (what is printed on the keycap)
    pub physical_name: String,
    // Electrical mapping (output, input)
    pub electrical: (u8, u8),
    // Electrical name (output, input)
    pub electrical_name: String,
    /// LED indexes
    pub leds: Vec<u8>,
    /// LED name
    pub led_name: String,
    // Key is currently pressed
    pub pressed: Cell<bool>,
    // Currently loaded scancodes and their names
    pub(crate) scancodes: RefCell<Vec<(u16, String)>>,
    // Background color
    pub background_color: Rgb,
}

impl Key {
    pub fn get_scancode(&self, layer: usize) -> Option<(u16, String)> {
        self.scancodes.borrow().get(layer).cloned()
    }
}
