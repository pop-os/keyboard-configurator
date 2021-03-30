use std::rc::Rc;

use crate::Hs;
use crate::{BoardId, Daemon, Layout, Matrix};

struct DaemonBoardInner {
    daemon: Rc<dyn Daemon>,
    board: BoardId,
    board_name: String,
    layout: Layout,
}

#[derive(Clone, glib::GBoxed)]
#[gboxed(type_name = "S76DaemonBoard")]
pub struct DaemonBoard(Rc<DaemonBoardInner>);

impl DaemonBoard {
    pub fn new(daemon: Rc<dyn Daemon>, board: BoardId) -> Result<Self, String> {
        let board_name = match daemon.model(board) {
            Ok(model) => model,
            Err(err) => {
                return Err(format!("Failed to get board model: {}", err));
            }
        };
        let layout = Layout::from_board(&board_name)
            .ok_or_else(|| format!("Failed to locate layout for '{}'", board_name))?;
        Ok(Self(Rc::new(DaemonBoardInner {
            daemon,
            board,
            board_name,
            layout,
        })))
    }

    pub fn model(&self) -> Result<String, String> {
        self.0.daemon.model(self.0.board)
    }

    pub fn keymap_get(&self, layer: u8, output: u8, input: u8) -> Result<u16, String> {
        self.0.daemon.keymap_get(self.0.board, layer, output, input)
    }

    pub fn keymap_set(&self, layer: u8, output: u8, input: u8, value: u16) -> Result<(), String> {
        self.0
            .daemon
            .keymap_set(self.0.board, layer, output, input, value)
    }

    pub fn matrix_get(&self) -> Result<Matrix, String> {
        self.0.daemon.matrix_get(self.0.board)
    }

    pub fn color(&self, index: u8) -> Result<Hs, String> {
        self.0.daemon.color(self.0.board, index)
    }

    pub fn set_color(&self, index: u8, color: Hs) -> Result<(), String> {
        self.0.daemon.set_color(self.0.board, index, color)
    }

    pub fn max_brightness(&self) -> Result<i32, String> {
        self.0.daemon.max_brightness(self.0.board)
    }

    pub fn brightness(&self, index: u8) -> Result<i32, String> {
        self.0.daemon.brightness(self.0.board, index)
    }

    pub fn set_brightness(&self, index: u8, brightness: i32) -> Result<(), String> {
        self.0
            .daemon
            .set_brightness(self.0.board, index, brightness)
    }

    pub fn mode(&self, layer: u8) -> Result<(u8, u8), String> {
        self.0.daemon.mode(self.0.board, layer)
    }

    pub fn set_mode(&self, layer: u8, mode: u8, speed: u8) -> Result<(), String> {
        self.0.daemon.set_mode(self.0.board, layer, mode, speed)
    }

    pub fn led_save(&self) -> Result<(), String> {
        self.0.daemon.led_save(self.0.board)
    }

    pub fn is_fake(&self) -> bool {
        self.0.daemon.is_fake()
    }

    pub fn layout(&self) -> &Layout {
        &self.0.layout
    }

    pub fn board_name(&self) -> &str {
        &self.0.board_name
    }
}
