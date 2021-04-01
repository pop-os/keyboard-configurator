use std::{collections::HashMap, rc::Rc};

use crate::Hs;
use crate::{BoardId, Daemon, Key, KeyMap, Layout, Matrix};

struct DaemonBoardInner {
    daemon: Rc<dyn Daemon>,
    board: BoardId,
    model: String,
    layout: Layout,
    keys: Vec<Key>,
}

#[derive(Clone, glib::GBoxed)]
#[gboxed(type_name = "S76DaemonBoard")]
pub struct DaemonBoard(Rc<DaemonBoardInner>);

impl DaemonBoard {
    pub fn new(daemon: Rc<dyn Daemon>, board: BoardId) -> Result<Self, String> {
        let model = match daemon.model(board) {
            Ok(model) => model,
            Err(err) => {
                return Err(format!("Failed to get board model: {}", err));
            }
        };
        let layout = Layout::from_board(&model)
            .ok_or_else(|| format!("Failed to locate layout for '{}'", model))?;

        let mut keys = layout.keys();
        for key in keys.iter_mut() {
            for layer in 0..layout.meta.num_layers {
                debug!("  Layer {}", layer);
                let scancode =
                    match daemon.keymap_get(board, layer, key.electrical.0, key.electrical.1) {
                        Ok(value) => value,
                        Err(err) => {
                            error!("Failed to read scancode: {:?}", err);
                            0
                        }
                    };
                debug!("    Scancode: {:04X}", scancode);

                let scancode_name = match layout.scancode_names.get(&scancode) {
                    Some(some) => some.to_string(),
                    None => String::new(),
                };
                debug!("    Scancode Name: {}", scancode_name);

                key.scancodes.borrow_mut().push((scancode, scancode_name));
            }
        }

        Ok(Self(Rc::new(DaemonBoardInner {
            daemon,
            board,
            keys,
            layout,
            model,
        })))
    }

    pub fn model(&self) -> &str {
        &self.0.model
    }

    pub fn keymap_set(
        &self,
        key_index: usize,
        layer: usize,
        scancode_name: &str,
    ) -> Result<(), String> {
        let k = &self.keys()[key_index];
        let scancode = *self
            .layout()
            .keymap
            .get(scancode_name)
            .ok_or_else(|| format!("Unable to find scancode '{}'", scancode_name))?;
        self.0.daemon.keymap_set(
            self.0.board,
            layer as u8,
            k.electrical.0,
            k.electrical.1,
            scancode,
        )?;
        k.scancodes.borrow_mut()[layer] = (scancode, scancode_name.to_string());
        Ok(())
    }

    pub fn matrix_get(&self) -> Result<Matrix, String> {
        self.0.daemon.matrix_get(self.0.board)
    }

    pub fn color(&self, index: u8) -> Result<Hs, String> {
        self.0.daemon.color(self.0.board, index)
    }

    pub fn set_color(&self, index: u8, color: Hs) -> Result<(), String> {
        self.0.daemon.set_color(self.0.board, index, color)
    }

    pub fn max_brightness(&self) -> Result<i32, String> {
        self.0.daemon.max_brightness(self.0.board)
    }

    pub fn brightness(&self, index: u8) -> Result<i32, String> {
        self.0.daemon.brightness(self.0.board, index)
    }

    pub fn set_brightness(&self, index: u8, brightness: i32) -> Result<(), String> {
        self.0
            .daemon
            .set_brightness(self.0.board, index, brightness)
    }

    pub fn mode(&self, layer: u8) -> Result<(u8, u8), String> {
        self.0.daemon.mode(self.0.board, layer)
    }

    pub fn set_mode(&self, layer: u8, mode: u8, speed: u8) -> Result<(), String> {
        self.0.daemon.set_mode(self.0.board, layer, mode, speed)
    }

    pub fn led_save(&self) -> Result<(), String> {
        self.0.daemon.led_save(self.0.board)
    }

    pub fn is_fake(&self) -> bool {
        self.0.daemon.is_fake()
    }

    pub fn layout(&self) -> &Layout {
        &self.0.layout
    }

    pub fn keys(&self) -> &[Key] {
        &self.0.keys
    }

    pub fn export_keymap(&self) -> KeyMap {
        let mut map = HashMap::new();
        for key in self.keys().iter() {
            let scancodes = key.scancodes.borrow();
            let scancodes = scancodes.iter().map(|s| s.1.clone()).collect();
            map.insert(key.logical_name.clone(), scancodes);
        }
        KeyMap {
            board: self.model().to_string(),
            map,
        }
    }
}
