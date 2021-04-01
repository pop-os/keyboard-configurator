use std::{
    cell::{Cell, RefCell},
    char,
};

use crate::{DaemonBoard, DaemonBoardWeak, Hs, Rect, Rgb};

#[derive(Debug)]
pub struct Key {
    pub(crate) board: DaemonBoardWeak,
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
    pub(crate) led_color: Cell<Option<Hs>>,
    // Key is currently pressed
    pub(crate) pressed: Cell<bool>,
    // Currently loaded scancodes and their names
    pub(crate) scancodes: RefCell<Vec<(u16, String)>>,
    // Background color
    pub background_color: Rgb,
}

impl Key {
    pub(crate) fn new(
        board: &DaemonBoard,
        logical: (u8, u8),
        physical: Rect,
        physical_name: String,
        background_color: Rgb,
    ) -> Self {
        debug!("Key {}, {} = {:?}", physical.x, physical.y, physical_name);

        debug!("  Logical: {:?}", logical);

        let row_char =
            char::from_digit(logical.0 as u32, 36).expect("Failed to convert row to char");
        let col_char =
            char::from_digit(logical.1 as u32, 36).expect("Failed to convert col to char");
        let logical_name = format!("K{}{}", row_char, col_char).to_uppercase();
        debug!("  Logical Name: {}", logical_name);

        let electrical = *board
            .layout()
            .layout
            .get(logical_name.as_str())
            //.expect("Failed to find electrical mapping");
            .unwrap_or(&(0, 0));
        debug!("  Electrical: {:?}", electrical);

        let leds = board
            .layout()
            .leds
            .get(logical_name.as_str())
            .map_or(Vec::new(), |x| x.clone());
        debug!("  LEDs: {:?}", leds);

        let mut led_name = String::new();
        for led in leds.iter() {
            if !led_name.is_empty() {
                led_name.push_str(", ");
            }
            led_name.push_str(&led.to_string());
        }

        Self {
            board: board.downgrade(),
            logical,
            logical_name,
            physical,
            physical_name,
            electrical,
            electrical_name: format!("{}, {}", electrical.0, electrical.1),
            leds,
            led_name,
            led_color: Cell::new(None),
            pressed: Cell::new(false),
            scancodes: RefCell::new(Vec::new()),
            background_color,
        }
    }

    fn board(&self) -> DaemonBoard {
        self.board.upgrade().unwrap()
    }

    pub fn pressed(&self) -> bool {
        self.pressed.get()
    }

    pub fn color(&self) -> Option<Hs> {
        self.led_color.get()
    }

    pub fn set_color(&self, color: Hs) -> Result<(), String> {
        let board = self.board();
        for index in &self.leds {
            board.0.daemon.set_color(board.0.board, *index, color)?;
        }
        self.led_color.set(Some(color));
        board.0.leds_changed.set(true);
        Ok(())
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
