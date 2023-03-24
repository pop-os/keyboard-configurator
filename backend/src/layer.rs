use std::sync::{
    atomic::{AtomicI32, Ordering},
    Mutex,
};

use crate::{Board, Daemon, Hs, Mode, Rgb, WeakBoard};

#[derive(Debug)]
pub struct Layer {
    layer: u8,
    index: u8,
    board: WeakBoard,
    pub(crate) mode: Mutex<Option<(u8, u8)>>,
    brightness: AtomicI32,
    color: Mutex<Hs>,
}

impl Layer {
    pub(crate) fn new(daemon: &dyn Daemon, board: &Board, layer: u8) -> Self {
        let index = if board.layout().meta.has_per_layer {
            0xf0 + layer
        } else {
            0xff
        };
        let mode = if board.layout().meta.has_mode {
            daemon
                .mode(board.board(), layer)
                .map(Some)
                .unwrap_or_else(|err| {
                    error!("Error getting layer mode: {}", err);
                    None
                })
        } else {
            None
        };
        let brightness = daemon
            .brightness(board.board(), index)
            .unwrap_or_else(|err| {
                error!("error getting layer brightness: {}", err);
                0
            });
        let color = daemon
            .color(board.board(), index)
            .map(|color| {
                if index == 0xff {
                    Rgb::new(color.0, color.1, color.2).to_hs_lossy()
                } else {
                    Hs::from_ints(color.0, color.1)
                }
            })
            .unwrap_or_else(|err| {
                error!("error getting layer color: {}", err);
                Hs::new(0., 0.)
            });
        Self {
            layer,
            index,
            board: board.downgrade(),
            mode: Mutex::new(mode),
            brightness: AtomicI32::new(brightness),
            color: Mutex::new(color),
        }
    }

    fn board(&self) -> Board {
        self.board.upgrade().unwrap()
    }

    /// Get the current mode and speed. `None` if not supported by board.
    pub fn mode(&self) -> Option<(&'static Mode, u8)> {
        let (index, speed) = (*self.mode.lock().unwrap())?;
        Some((Mode::from_index(index)?, speed))
    }

    pub async fn set_mode(&self, mode: &Mode, speed: u8) -> Result<(), String> {
        let board = self.board();
        board
            .thread_client()
            .set_mode(board.board(), self.layer, mode.index, speed)
            .await?;
        *self.mode.lock().unwrap() = Some((mode.index, speed));
        board.set_leds_changed();
        Ok(())
    }

    /// Get the current brightness
    pub fn brightness(&self) -> i32 {
        self.brightness.load(Ordering::SeqCst)
    }

    pub async fn set_brightness(&self, brightness: i32) -> Result<(), String> {
        let board = self.board();
        board
            .thread_client()
            .set_brightness(board.board(), self.index, brightness)
            .await?;
        self.brightness.store(brightness, Ordering::SeqCst);
        board.set_leds_changed();
        Ok(())
    }

    /// Get the current color
    pub fn color(&self) -> Hs {
        *self.color.lock().unwrap()
    }

    pub async fn set_color(&self, hs: Hs) -> Result<(), String> {
        let board = self.board();
        let color = if self.index == 0xff {
            let Rgb { r, g, b } = hs.to_rgb();
            (r, g, b)
        } else {
            let (h, s) = hs.to_ints();
            (h, s, 0)
        };
        board
            .thread_client()
            .set_color(board.board(), self.index, color)
            .await?;
        *self.color.lock().unwrap() = hs;
        board.set_leds_changed();
        Ok(())
    }
}
