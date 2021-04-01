use once_cell::unsync::OnceCell;
use std::{
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::Hs;
use crate::{BoardId, Daemon, Key, KeyMap, Layout, Matrix};

pub(crate) struct DaemonBoardInner {
    pub(crate) daemon: Rc<dyn Daemon>,
    pub(crate) board: BoardId,
    model: String,
    layout: Layout,
    keys: OnceCell<Vec<Key>>,
    max_brightness: i32,
}

#[derive(Clone, glib::GBoxed)]
#[gboxed(type_name = "S76DaemonBoard")]
pub struct DaemonBoard(pub(crate) Rc<DaemonBoardInner>);

pub(crate) struct DaemonBoardWeak(Weak<DaemonBoardInner>);

impl std::fmt::Debug for DaemonBoardWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "DaemonBoardWeak({:p})", self)
    }
}

impl DaemonBoardWeak {
    pub fn upgrade(&self) -> Option<DaemonBoard> {
        self.0.upgrade().map(DaemonBoard)
    }
}

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

        let max_brightness = daemon.max_brightness(board).unwrap_or_else(|err| {
            error!("Error getting max brightness: {}", err);
            100
        });

        let inner = Rc::new(DaemonBoardInner {
            daemon,
            board,
            keys: OnceCell::new(),
            layout,
            max_brightness,
            model,
        });

        let mut keys = inner.layout.keys();
        for key in &mut keys {
            for layer in 0..inner.layout.meta.num_layers {
                debug!("  Layer {}", layer);
                let scancode =
                    match inner
                        .daemon
                        .keymap_get(board, layer, key.electrical.0, key.electrical.1)
                    {
                        Ok(value) => value,
                        Err(err) => {
                            error!("Failed to read scancode: {:?}", err);
                            0
                        }
                    };
                debug!("    Scancode: {:04X}", scancode);

                let scancode_name = match inner.layout.scancode_names.get(&scancode) {
                    Some(some) => some.to_string(),
                    None => String::new(),
                };
                debug!("    Scancode Name: {}", scancode_name);

                key.scancodes.borrow_mut().push((scancode, scancode_name));
            }

            key.board
                .set(DaemonBoardWeak(Rc::downgrade(&inner)))
                .unwrap();
        }
        inner.keys.set(keys).unwrap();

        Ok(Self(inner))
    }

    pub fn model(&self) -> &str {
        &self.0.model
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

    pub fn max_brightness(&self) -> i32 {
        self.0.max_brightness
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
        self.0.keys.get().unwrap()
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
