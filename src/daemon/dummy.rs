use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use super::{BoardId, Daemon};
use crate::Rgb;

#[derive(Default)]
struct BoardDummy {
    name: String,
    keymap: RefCell<HashMap<(u8, u8, u8), u16>>,
    color: Cell<Rgb>,
    brightness: Cell<i32>,
}

pub struct DaemonDummy {
    boards: Vec<BoardDummy>,
}

impl DaemonDummy {
    fn board(&self, board: BoardId) -> Result<&BoardDummy, String> {
        self.boards
            .get(board.0)
            .ok_or_else(|| "No board".to_string())
    }
}

impl DaemonDummy {
    pub fn new(board_names: Vec<String>) -> Self {
        let boards = board_names
            .into_iter()
            .map(|name| BoardDummy {
                name,
                ..Default::default()
            })
            .collect();
        Self { boards }
    }
}

impl Daemon for DaemonDummy {
    fn boards(&self) -> Result<Vec<BoardId>, String> {
        Ok((0..self.boards.len()).map(BoardId).collect())
    }

    fn model(&self, board: BoardId) -> Result<String, String> {
        Ok(self.board(board)?.name.clone())
    }

    fn is_fake(&self) -> bool {
        true
    }

    fn keymap_get(&self, board: BoardId, layer: u8, output: u8, input: u8) -> Result<u16, String> {
        let keymap = self.board(board)?.keymap.borrow();
        Ok(keymap.get(&(layer, output, input)).copied().unwrap_or(0))
    }

    fn keymap_set(
        &self,
        board: BoardId,
        layer: u8,
        output: u8,
        input: u8,
        value: u16,
    ) -> Result<(), String> {
        let mut keymap = self.board(board)?.keymap.borrow_mut();
        keymap.insert((layer, output, input), value);
        Ok(())
    }

    fn color(&self, board: BoardId) -> Result<Rgb, String> {
        Ok(self.board(board)?.color.get())
    }

    fn set_color(&self, board: BoardId, color: Rgb) -> Result<(), String> {
        self.board(board)?.color.set(color);
        Ok(())
    }

    fn max_brightness(&self, _board: BoardId) -> Result<i32, String> {
        Ok(100)
    }

    fn brightness(&self, board: BoardId) -> Result<i32, String> {
        Ok(self.board(board)?.brightness.get())
    }

    fn set_brightness(&self, board: BoardId, brightness: i32) -> Result<(), String> {
        self.board(board)?.brightness.set(brightness);
        Ok(())
    }

    fn exit(&self) -> Result<(), String> {
        Ok(())
    }
}
