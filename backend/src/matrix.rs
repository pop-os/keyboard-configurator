use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Default, PartialEq, Clone)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: Box<[u8]>,
}

impl Matrix {
    pub fn new(rows: usize, cols: usize, data: Box<[u8]>) -> Self {
        Self { rows, cols, data }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn get(&self, row: usize, col: usize) -> Option<bool> {
        if row < self.rows && col < self.cols {
            let i = row * self.cols + col;
            let byte = i / 8;
            let bit = i % 8;
            Some((self.data[byte] & (1 << bit)) != 0)
        } else {
            None
        }
    }

    pub fn set(&mut self, row: usize, col: usize, value: bool) {
        if row < self.rows && col < self.cols {
            let i = row * self.cols + col;
            let byte = i / 8;
            let bit = i % 8;
            if value {
                self.data[byte] |= 1 << bit;
            } else {
                self.data[byte] &= !(1 << bit);
            }
        }
    }
}
