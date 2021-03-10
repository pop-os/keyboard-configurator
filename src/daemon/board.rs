use std::rc::Rc;

use super::{BoardId, Daemon, Matrix};
use crate::Hs;

use std::thread::Thread;
use std::sync::atomic::{AtomicU16, AtomicU8, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
struct LayerData {
    brightness: AtomicU8,
    mode: AtomicU8,
    speed: AtomicU8,
    color: Mutex<Hs>,
}
struct Data {
    changed: AtomicU16,
    max_brightness: i32,
    matrix: Mutex<Matrix>,
    model: String,
    thread: Thread,
    keymap: HashMap<(u8, u8, u8), AtomicU16>,
    layers: Vec<LayerData>,
    per_key_colors: Mutex<Vec<Hs>>,
}
impl Data {
    // xxx use enum w/ repr(u8)
    fn set_changed(&self, index: u8) {
        self.changed.fetch_or(1u16 << index, SeqCst);
    }

    fn clear_changed(&self) -> u16 {
        self.changed.swap(0, SeqCst)
    }
}
#[derive(Clone)]
struct Board {
    data: Arc<Data>
}

impl Board {
    pub fn model(&self) -> &str {
        &self.data.model
    }

    pub fn max_brightness(&self) -> i32 {
        self.data.max_brightness
    }

    fn layer(&self, layer: u8) -> Option<&LayerData> {
        self.data.layers.get(layer as usize)
    }

    pub fn mode(&self, layer: u8) -> Option<u8> {
        self.layer(layer).map(|x| x.mode.load(SeqCst))
    }

    pub fn speed(&self, layer: u8) -> Option<u8> {
        self.layer(layer).map(|x| x.speed.load(SeqCst))
    }

    pub fn brightness(&self, layer: u8) -> Option<u8> {
        self.layer(layer).map(|x| x.brightness.load(SeqCst))
    }

    pub fn color(&self, layer: u8) -> Option<Hs> {
        self.layer(layer).map(|x| *x.color.lock().unwrap())
    }
    // TODO per key color

    pub fn keymap_get(&self, layer: u8, output: u8, input: u8) -> Option<u16> {
        self.data.keymap.get(&(layer, output, input)).map(|x| x.load(SeqCst))
    }
}

#[derive(Clone, glib::GBoxed)]
#[gboxed(type_name = "S76DaemonBoard")]
pub struct DaemonBoard(pub Rc<dyn Daemon>, pub BoardId);

impl DaemonBoard {
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
}
