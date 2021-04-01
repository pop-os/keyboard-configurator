use once_cell::unsync::OnceCell;
use std::cell::{Cell, RefCell};

use crate::{DaemonBoard, DaemonBoardWeak, Rect, Rgb};

#[derive(Debug)]
pub struct Key {
    pub(crate) board: OnceCell<DaemonBoardWeak>,
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
    fn board(&self) -> DaemonBoard {
        self.board.get().unwrap().upgrade().unwrap()
    }

    pub fn get_scancode(&self, layer: usize) -> Option<(u16, String)> {
        self.scancodes.borrow().get(layer).cloned()
    }

    pub fn set_scancode(&self, layer: usize, scancode_name: &str) -> Result<(), String> {
        let board = self.board();
        let scancode = *board
            .layout()
            .keymap
            .get(scancode_name)
            .ok_or_else(|| format!("Unable to find scancode '{}'", scancode_name))?;
        board.0.daemon.keymap_set(
            board.0.board,
            layer as u8,
            self.electrical.0,
            self.electrical.1,
            scancode,
        )?;
        self.scancodes.borrow_mut()[layer] = (scancode, scancode_name.to_string());
        Ok(())
    }
}
