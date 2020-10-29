use std::{
    cell::{
        Cell,
        RefCell,
    },
    collections::HashMap,
};

use crate::color::Rgb;
use super::Daemon;
use crate::application::layout;

#[derive(Default)]
struct BoardDummy {
    keymap: RefCell<HashMap<(u8, u8, u8), u16>>,
    color: Cell<Rgb>,
    brightness: Cell<i32>,
}

pub struct DaemonDummy {
    board_names: Vec<String>,
    boards: Vec<BoardDummy>,
}

impl DaemonDummy {
    fn board(&self, board: usize) -> Result<&BoardDummy, String> {
        self.boards.get(board).ok_or("No board".to_string())
    }
}

impl DaemonDummy {
    pub fn new(board_names: Vec<String>) -> Self {
        let boards = board_names.iter().map(|_| BoardDummy::default()).collect();
        Self {
            board_names,
            boards,
        }
    }
}

impl Daemon for DaemonDummy {
    fn boards(&self) -> Result<Vec<String>, String> {
        Ok(self.board_names.clone())
    }

    fn keymap_get(&self, board: usize, layer: u8, output: u8, input: u8) -> Result<u16, String> {
        let keymap = self.board(board)?.keymap.borrow();
        Ok(keymap.get(&(layer, output, input)).copied().unwrap_or(0))
    }

    fn keymap_set(&self, board: usize, layer: u8, output: u8, input: u8, value: u16) -> Result<(), String> {
        let mut keymap = self.board(board)?.keymap.borrow_mut();
        keymap.insert((layer, output, input), value);
        Ok(())
    }

    fn color(&self, board: usize) -> Result<Rgb, String> {
        Ok(self.board(board)?.color.get())
    }

    fn set_color(&self, board: usize, color: Rgb) -> Result<(), String> {
        self.board(board)?.color.set(color);
        Ok(())
    }

    fn max_brightness(&self, _board: usize) -> Result<i32, String> {
        Ok(100)
    }

    fn brightness(&self, board: usize) -> Result<i32, String> {
        Ok(self.board(board)?.brightness.get())
    }

    fn set_brightness(&self, board: usize, brightness: i32) -> Result<(), String> {
        self.board(board)?.brightness.set(brightness);
        Ok(())
    }

    fn exit(&self) -> Result<(), String> {
        Ok(())
    }
}
