#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    /// Test if `(x, y)` is a point in the rectangle
    pub fn contains(&self, x: f64, y: f64) -> bool {
        (self.x..=self.x + self.w).contains(&x) && (self.y..=self.y + self.h).contains(&y)
    }
}
