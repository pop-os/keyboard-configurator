use once_cell::unsync::OnceCell;
use std::{
    cell::Cell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{BoardId, Daemon, Key, KeyMap, Layer, Layout};

pub(crate) struct DaemonBoardInner {
    pub(crate) daemon: Rc<dyn Daemon>,
    pub(crate) board: BoardId,
    model: String,
    layout: Layout,
    keys: OnceCell<Vec<Key>>,
    layers: OnceCell<Vec<Layer>>,
    max_brightness: i32,
    pub(crate) leds_changed: Cell<bool>,
    has_led_save: bool,
    has_matrix: bool,
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

        let num_layers = if layout.meta.has_per_layer {
            layout.meta.num_layers
        } else {
            1
        };

        let has_led_save = daemon.led_save(board).is_ok();
        let has_matrix = daemon.matrix_get(board).is_ok();

        let self_ = Self(Rc::new(DaemonBoardInner {
            daemon,
            board,
            keys: OnceCell::new(),
            layers: OnceCell::new(),
            layout,
            max_brightness,
            model,
            leds_changed: Cell::new(false),
            has_led_save,
            has_matrix,
        }));

        let keys = self_
            .0
            .layout
            .physical
            .iter()
            .map(|i| Key::new(&self_, i))
            .collect();
        self_.0.keys.set(keys).unwrap();

        let layers = (0..num_layers)
            .map(|layer| Layer::new(&self_, layer))
            .collect();
        self_.0.layers.set(layers).unwrap();

        Ok(self_)
    }

    pub(crate) fn downgrade(&self) -> DaemonBoardWeak {
        DaemonBoardWeak(Rc::downgrade(&self.0))
    }

    pub fn model(&self) -> &str {
        &self.0.model
    }

    pub fn has_matrix(&self) -> bool {
        self.0.has_matrix
    }

    pub fn refresh_matrix(&self) -> Result<bool, String> {
        let matrix = self.0.daemon.matrix_get(self.0.board)?;
        let mut changed = false;
        for key in self.keys() {
            let pressed = matrix
                .get(key.electrical.0 as usize, key.electrical.1 as usize)
                .unwrap_or(false);
            changed |= key.pressed.replace(pressed) != pressed;
        }
        Ok(changed)
    }

    pub fn max_brightness(&self) -> i32 {
        self.0.max_brightness
    }

    pub fn led_save(&self) -> Result<(), String> {
        if self.has_led_save() && self.0.leds_changed.get() {
            self.0.daemon.led_save(self.0.board)?;
            self.0.leds_changed.set(false);
            debug!("led_save");
        }
        Ok(())
    }

    pub fn is_fake(&self) -> bool {
        self.0.daemon.is_fake()
    }

    pub fn has_led_save(&self) -> bool {
        self.0.has_led_save
    }

    pub fn layout(&self) -> &Layout {
        &self.0.layout
    }

    pub fn layers(&self) -> &[Layer] {
        self.0.layers.get().unwrap()
    }

    pub fn keys(&self) -> &[Key] {
        self.0.keys.get().unwrap()
    }

    pub fn export_keymap(&self) -> KeyMap {
        let mut map = HashMap::new();
        for key in self.keys().iter() {
            let scancodes = (0..self.layout().meta.num_layers as usize)
                .map(|layer| key.get_scancode(layer).unwrap().1)
                .collect();
            map.insert(key.logical_name.clone(), scancodes);
        }
        KeyMap {
            board: self.model().to_string(),
            map,
        }
    }
}
