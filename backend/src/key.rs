use glib::clone::Downgrade;
use std::cell::Cell;

use crate::{Board, Daemon, Hs, PhysicalLayoutKey, Rect, Rgb};

#[derive(Debug)]
pub struct Key {
    pub(crate) board: glib::WeakRef<Board>,
    /// Logical position (row, column)
    pub logical: (u8, u8),
    /// Logical name (something like K01, where 0 is the row and 1 is the column)
    pub logical_name: String,
    /// Physical position and size
    pub physical: Rect,
    /// Physical key name (what is printed on the keycap)
    pub physical_name: String,
    /// Electrical mapping (output, input)
    pub electrical: (u8, u8),
    /// Electrical name (output, input)
    pub electrical_name: String,
    /// LED indexes
    pub leds: Vec<u8>,
    /// LED name
    pub led_name: String,
    led_color: Cell<Option<Hs>>,
    /// Key is currently pressed
    pub(crate) pressed: Cell<bool>,
    /// Currently loaded scancodes and their names
    scancodes: Vec<Cell<u16>>,
    /// Background color
    pub background_color: Rgb,
}

impl Key {
    pub(crate) fn new(
        daemon: &dyn Daemon,
        board: &Board,
        physical_key: &PhysicalLayoutKey,
    ) -> Self {
        let logical = physical_key.logical;
        let logical_name = physical_key.logical_name();
        let physical = physical_key.physical;
        let physical_name = physical_key.physical_name.clone();
        let background_color = physical_key.background_color;

        debug!("Key {}, {} = {:?}", physical.x, physical.y, physical_name);

        debug!("  Logical: {:?}", logical);
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

        let mut scancodes = Vec::new();
        for layer in 0..board.layout().meta.num_layers {
            debug!("  Layer {}", layer);
            let scancode = match daemon.keymap_get(board.board(), layer, electrical.0, electrical.1)
            {
                Ok(value) => value,
                Err(err) => {
                    error!("Failed to read scancode: {:?}", err);
                    0
                }
            };
            debug!("    Scancode: {:04X}", scancode);
            debug!(
                "    Scancode Name: {:?}",
                board.layout().scancode_to_name(scancode)
            );

            scancodes.push(Cell::new(scancode));
        }

        let mut led_color = None;
        if board.layout().meta.has_mode && leds.len() > 0 {
            match daemon.color(board.board(), leds[0]) {
                Ok((0, 0, 0)) => {}
                Ok((r, g, b)) => led_color = Some(Rgb::new(r, g, b).to_hs_lossy()),
                Err(err) => error!("error getting key color: {}", err),
            }
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
            led_color: Cell::new(led_color),
            pressed: Cell::new(false),
            scancodes,
            background_color,
        }
    }

    fn board(&self) -> Board {
        self.board.upgrade().unwrap()
    }

    pub fn pressed(&self) -> bool {
        self.pressed.get()
    }

    pub fn color(&self) -> Option<Hs> {
        self.led_color.get()
    }

    pub async fn set_color(&self, color: Option<Hs>) -> Result<(), String> {
        let board = self.board();
        let Rgb { r, g, b } = color.map_or(Rgb::new(0, 0, 0), Hs::to_rgb);
        for index in &self.leds {
            board
                .thread_client()
                .set_color(board.board(), *index, (r, g, b))
                .await?;
        }
        self.led_color.set(color);
        board.set_leds_changed();
        Ok(())
    }

    pub fn get_scancode(&self, layer: usize) -> Option<(u16, String)> {
        let board = self.board();
        let scancode = self.scancodes.get(layer)?.get();
        let scancode_name = match board.layout().scancode_to_name(scancode) {
            Some(some) => some.to_string(),
            None => String::new(),
        };
        Some((scancode, scancode_name))
    }

    pub async fn set_scancode(&self, layer: usize, scancode_name: &str) -> Result<(), String> {
        let board = self.board();
        let scancode = board
            .layout()
            .scancode_from_name(scancode_name)
            .ok_or_else(|| format!("Unable to find scancode '{}'", scancode_name))?;
        board
            .thread_client()
            .keymap_set(
                board.board(),
                layer as u8,
                self.electrical.0,
                self.electrical.1,
                scancode,
            )
            .await?;
        self.scancodes[layer].set(scancode);
        Ok(())
    }
}
