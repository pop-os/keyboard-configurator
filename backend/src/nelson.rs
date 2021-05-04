use serde::{Deserialize, Serialize};
use std::cmp;

use crate::Matrix;

#[derive(Deserialize, Serialize)]
pub struct Nelson {
    pub missing: Matrix,
    pub bouncing: Matrix,
    pub sticking: Matrix,
}

impl Nelson {
    pub fn max_rows(&self) -> usize {
        cmp::max(self.missing.rows(), cmp::max(self.bouncing.rows(), self.sticking.rows()))
    }

    pub fn max_cols(&self) -> usize {
        cmp::max(self.missing.cols(), cmp::max(self.bouncing.cols(), self.sticking.cols()))
    }

    pub fn success(&self) -> bool {
        for matrix in &[&self.missing, &self.bouncing, &self.sticking] {
            for row in 0..matrix.rows() {
                for col in 0..matrix.cols() {
                    if matrix.get(row, col).unwrap_or(false) {
                        return false;
                    }
                }
            }
        }
        true
    }
}
