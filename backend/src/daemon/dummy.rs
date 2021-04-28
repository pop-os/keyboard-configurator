use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use super::{BoardId, Daemon};
use crate::{Matrix, Nelson};

#[derive(Default)]
struct BoardDummy {
    name: String,
    keymap: RefCell<HashMap<(u8, u8, u8), u16>>,
    color: Cell<(u8, u8, u8)>,
    brightness: Cell<i32>,
    mode: Cell<(u8, u8)>,
}

pub struct DaemonDummy {
    boards: Vec<BoardDummy>,
}

impl DaemonDummy {
    fn board(&self, board: BoardId) -> Result<&BoardDummy, String> {
        self.boards
            .get(board.0 as usize)
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

    fn nelson(&self, _board: BoardId) -> Result<Nelson, String> {
        Ok(Nelson {
            missing: Matrix::new(0, 0, Vec::new().into_boxed_slice()),
            bouncing: Matrix::new(0, 0, Vec::new().into_boxed_slice()),
        })
    }

    fn color(&self, board: BoardId, index: u8) -> Result<(u8, u8, u8), String> {
        // TODO implement support for per-led
        if index != 0xFF {
            return Err(format!("Can't set color index {}", index));
        }
        Ok(self.board(board)?.color.get())
    }

    fn set_color(&self, board: BoardId, index: u8, color: (u8, u8, u8)) -> Result<(), String> {
        if index != 0xFF {
            return Err(format!("Can't set color index {}", index));
        }
        self.board(board)?.color.set(color);
        Ok(())
    }

    fn max_brightness(&self, _board: BoardId) -> Result<i32, String> {
        Ok(100)
    }

    fn brightness(&self, board: BoardId, index: u8) -> Result<i32, String> {
        if index != 0xFF {
            return Err(format!("Can't set color index {}", index));
        }
        Ok(self.board(board)?.brightness.get())
    }

    fn set_brightness(&self, board: BoardId, index: u8, brightness: i32) -> Result<(), String> {
        if index != 0xFF {
            return Err(format!("Can't set color index {}", index));
        }
        self.board(board)?.brightness.set(brightness);
        Ok(())
    }

    fn mode(&self, board: BoardId, _layer: u8) -> Result<(u8, u8), String> {
        // TODO layer
        Ok(self.board(board)?.mode.get())
    }

    fn set_mode(&self, board: BoardId, _layer: u8, mode: u8, speed: u8) -> Result<(), String> {
        self.board(board)?.mode.set((mode, speed));
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
