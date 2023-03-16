use serde::{Deserialize, Serialize};
use std::cmp;
use std::collections::HashMap;

use crate::Matrix;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum NelsonKind {
    Normal,
    Bouncing,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Nelson {
    pub missing: Matrix,
    pub bouncing: Matrix,
    pub sticking: Matrix,
}

impl Nelson {
    pub fn max_rows(&self) -> usize {
        cmp::max(
            self.missing.rows(),
            cmp::max(self.bouncing.rows(), self.sticking.rows()),
        )
    }

    pub fn max_cols(&self) -> usize {
        cmp::max(
            self.missing.cols(),
            cmp::max(self.bouncing.cols(), self.sticking.cols()),
        )
    }

    pub fn success(&self, layout: &HashMap<std::string::String, (u8, u8)>) -> bool {
        let values: Vec<&(u8, u8)> = layout.values().collect();
        for matrix in &[&self.missing, &self.bouncing, &self.sticking] {
            for (row, col) in values.iter() {
                if matrix.get(*row as usize, *col as usize).unwrap_or(false) {
                    return false;
                }
            }
        }
        true
    }
}
