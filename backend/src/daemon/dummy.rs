use std::{cell::RefCell, collections::HashMap};

use super::{BoardId, Daemon};
use crate::{fl, Layout, Matrix};

struct BoardDummy {
    name: String,
    layout: Layout,
    keymap: RefCell<HashMap<(u8, u8, u8), u16>>,
    colors: RefCell<HashMap<u8, (u8, u8, u8)>>,
    brightnesses: RefCell<HashMap<u8, i32>>,
    modes: RefCell<HashMap<u8, (u8, u8)>>,
}

impl BoardDummy {
    fn valid_index(&self, index: u8, allow_key: bool) -> bool {
        if !self.layout.meta.has_per_layer {
            index == 0xff
        } else if index >= 0xf0 {
            index < 0xf0 + self.layout.meta.num_layers
        } else {
            allow_key
                && self
                    .layout
                    .leds
                    .values()
                    .flatten()
                    .find(|i| **i == index)
                    .is_some()
        }
    }
}

pub struct DaemonDummy {
    boards: Vec<BoardDummy>,
}

impl DaemonDummy {
    pub fn new(board_names: Vec<String>) -> Self {
        let boards = board_names
            .into_iter()
            .map(|name| BoardDummy {
                layout: Layout::from_board(&name).unwrap(),
                name,
                keymap: Default::default(),
                colors: Default::default(),
                brightnesses: Default::default(),
                modes: Default::default(),
            })
            .collect();
        Self { boards }
    }

    fn board(&self, board: BoardId) -> Result<&BoardDummy, String> {
        self.boards
            .get(board.0 as usize)
            .ok_or_else(|| fl!("no-board"))
    }
}

impl Daemon for DaemonDummy {
    fn boards(&self) -> Result<Vec<BoardId>, String> {
        Ok((0..self.boards.len() as u128).map(BoardId).collect())
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

    fn matrix_get(&self, _board: BoardId) -> Result<Matrix, String> {
        Ok(Matrix::new(0, 0, Vec::new().into_boxed_slice()))
    }

    fn color(&self, board: BoardId, index: u8) -> Result<(u8, u8, u8), String> {
        let board = self.board(board)?;
        if !board.valid_index(index, true) {
            return Err(format!("Can't get color index {} {}", index, board.name));
        }
        Ok(*board.colors.borrow_mut().entry(index).or_default())
    }

    fn set_color(&self, board: BoardId, index: u8, color: (u8, u8, u8)) -> Result<(), String> {
        let board = self.board(board)?;
        if !board.valid_index(index, true) {
            return Err(format!("Can't set color index {}", index));
        }
        board.colors.borrow_mut().insert(index, color);
        Ok(())
    }

    fn max_brightness(&self, _board: BoardId) -> Result<i32, String> {
        Ok(100)
    }

    fn brightness(&self, board: BoardId, index: u8) -> Result<i32, String> {
        let board = self.board(board)?;
        if !board.valid_index(index, false) {
            return Err(format!("Can't get brightness index {}", index));
        }
        Ok(*board.brightnesses.borrow_mut().entry(index).or_default())
    }

    fn set_brightness(&self, board: BoardId, index: u8, brightness: i32) -> Result<(), String> {
        let board = self.board(board)?;
        if !board.valid_index(index, false) {
            return Err(format!("Can't set brightness index {}", index));
        }
        board.brightnesses.borrow_mut().insert(index, brightness);
        Ok(())
    }

    fn mode(&self, board: BoardId, layer: u8) -> Result<(u8, u8), String> {
        let index = layer + 0xf0;
        let board = self.board(board)?;
        if !board.valid_index(index, false) {
            return Err(format!("Can't get mode index {}", index));
        }
        Ok(*board.modes.borrow_mut().entry(index).or_default())
    }

    fn set_mode(&self, board: BoardId, layer: u8, mode: u8, speed: u8) -> Result<(), String> {
        let index = layer + 0xf0;
        let board = self.board(board)?;
        if !board.valid_index(index, false) {
            return Err(format!("Can't get mode index {}", index));
        }
        board.modes.borrow_mut().insert(index, (mode, speed));
        Ok(())
    }

    fn led_save(&self, board: BoardId) -> Result<(), String> {
        self.board(board)?;
        Ok(())
    }

    fn refresh(&self) -> Result<(), String> {
        Ok(())
    }

    fn exit(&self) -> Result<(), String> {
        Ok(())
    }
}
