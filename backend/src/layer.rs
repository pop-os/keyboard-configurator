use glib::clone::Downgrade;
use std::cell::Cell;

use crate::{DaemonBoard, Hs, Mode};

#[derive(Debug)]
pub struct Layer {
    layer: u8,
    index: u8,
    board: glib::WeakRef<DaemonBoard>,
    mode: Cell<Option<(u8, u8)>>,
    brightness: Cell<i32>,
    color: Cell<Hs>,
}

impl Layer {
    pub(crate) fn new(board: &DaemonBoard, layer: u8) -> Self {
        let index = if board.layout().meta.has_per_layer {
            0xf0 + layer
        } else {
            0xff
        };
        let mode = if board.layout().meta.has_mode {
            board
                .daemon()
                .mode(board.board(), layer)
                .map(Some)
                .unwrap_or_else(|err| {
                    error!("Error getting layer mode: {}", err);
                    None
                })
        } else {
            None
        };
        let brightness = board
            .daemon()
            .brightness(board.board(), index)
            .unwrap_or_else(|err| {
                error!("error getting layer brightness: {}", err);
                0
            });
        let color = board
            .daemon()
            .color(board.board(), index)
            .unwrap_or_else(|err| {
                error!("error getting layer color: {}", err);
                Hs::new(0., 0.)
            });
        Self {
            layer,
            index,
            board: board.downgrade(),
            mode: Cell::new(mode),
            brightness: Cell::new(brightness),
            color: Cell::new(color),
        }
    }

    fn board(&self) -> DaemonBoard {
        self.board.upgrade().unwrap()
    }

    pub fn mode(&self) -> Option<(&'static Mode, u8)> {
        let (index, speed) = self.mode.get()?;
        Some((Mode::from_index(index)?, speed))
    }

    pub fn set_mode(&self, mode: &Mode, speed: u8) -> Result<(), String> {
        let board = self.board();
        board
            .daemon()
            .set_mode(board.board(), self.layer, mode.index, speed)?;
        self.mode.set(Some((mode.index, speed)));
        board.set_leds_changed();
        Ok(())
    }

    pub fn brightness(&self) -> i32 {
        self.brightness.get()
    }

    pub fn set_brightness(&self, brightness: i32) -> Result<(), String> {
        let board = self.board();
        board
            .daemon()
            .set_brightness(board.board(), self.index, brightness)?;
        self.brightness.set(brightness);
        board.set_leds_changed();
        Ok(())
    }

    pub fn color(&self) -> Hs {
        self.color.get()
    }

    pub fn set_color(&self, color: Hs) -> Result<(), String> {
        let board = self.board();
        board.daemon().set_color(board.board(), self.index, color)?;
        self.color.set(color);
        board.set_leds_changed();
        Ok(())
    }
}
