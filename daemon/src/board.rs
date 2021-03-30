use std::rc::Rc;

use crate::Hs;
use crate::{BoardId, Daemon, Matrix};

#[derive(Clone, glib::GBoxed)]
#[gboxed(type_name = "S76DaemonBoard")]
pub struct DaemonBoard(Rc<dyn Daemon>, BoardId);

impl DaemonBoard {
    pub fn new(daemon: Rc<dyn Daemon>, id: BoardId) -> Self {
        Self(daemon, id)
    }

    pub fn model(&self) -> Result<String, String> {
        self.0.model(self.1)
    }

    pub fn keymap_get(&self, layer: u8, output: u8, input: u8) -> Result<u16, String> {
        self.0.keymap_get(self.1, layer, output, input)
    }

    pub fn keymap_set(&self, layer: u8, output: u8, input: u8, value: u16) -> Result<(), String> {
        self.0.keymap_set(self.1, layer, output, input, value)
    }

    pub fn matrix_get(&self) -> Result<Matrix, String> {
        self.0.matrix_get(self.1)
    }

    pub fn color(&self, index: u8) -> Result<Hs, String> {
        self.0.color(self.1, index)
    }

    pub fn set_color(&self, index: u8, color: Hs) -> Result<(), String> {
        self.0.set_color(self.1, index, color)
    }

    pub fn max_brightness(&self) -> Result<i32, String> {
        self.0.max_brightness(self.1)
    }

    pub fn brightness(&self, index: u8) -> Result<i32, String> {
        self.0.brightness(self.1, index)
    }

    pub fn set_brightness(&self, index: u8, brightness: i32) -> Result<(), String> {
        self.0.set_brightness(self.1, index, brightness)
    }

    pub fn mode(&self, layer: u8) -> Result<(u8, u8), String> {
        self.0.mode(self.1, layer)
    }

    pub fn set_mode(&self, layer: u8, mode: u8, speed: u8) -> Result<(), String> {
        self.0.set_mode(self.1, layer, mode, speed)
    }

    pub fn led_save(&self) -> Result<(), String> {
        self.0.led_save(self.1)
    }

    pub fn is_fake(&self) -> bool {
        self.0.is_fake()
    }
}
