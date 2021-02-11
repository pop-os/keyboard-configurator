use std::rc::Rc;

use super::Daemon;
use crate::Rgb;

#[derive(Clone, glib::GBoxed)]
#[gboxed(type_name = "S76DaemonBoard")]
pub struct DaemonBoard(pub Rc<dyn Daemon>, pub usize);

impl DaemonBoard {
    pub fn keymap_get(&self, layer: u8, output: u8, input: u8) -> Result<u16, String> {
        self.0.keymap_get(self.1, layer, output, input)
    }

    pub fn keymap_set(&self, layer: u8, output: u8, input: u8, value: u16) -> Result<(), String> {
        self.0.keymap_set(self.1, layer, output, input, value)
    }

    pub fn color(&self) -> Result<Rgb, String> {
        self.0.color(self.1)
    }

    pub fn set_color(&self, color: Rgb) -> Result<(), String> {
        self.0.set_color(self.1, color)
    }

    pub fn max_brightness(&self) -> Result<i32, String> {
        self.0.max_brightness(self.1)
    }

    pub fn brightness(&self) -> Result<i32, String> {
        self.0.brightness(self.1)
    }

    pub fn set_brightness(&self, brightness: i32) -> Result<(), String> {
        self.0.set_brightness(self.1, brightness)
    }
}
