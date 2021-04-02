use glib::subclass::prelude::*;
use std::{cell::Cell, collections::HashMap, rc::Rc};

use crate::{BoardId, Daemon, DerefCell, Key, KeyMap, Layer, Layout};

// GObject
// Add changed signal
// Want DerefCell, I guess... Or use OnceCell

#[derive(Default)]
pub struct DaemonBoardInner {
    daemon: DerefCell<Rc<dyn Daemon>>,
    board: DerefCell<BoardId>,
    model: DerefCell<String>,
    layout: DerefCell<Layout>,
    keys: DerefCell<Vec<Key>>,
    layers: DerefCell<Vec<Layer>>,
    max_brightness: DerefCell<i32>,
    pub(crate) leds_changed: Cell<bool>,
    has_led_save: DerefCell<bool>,
    has_matrix: DerefCell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for DaemonBoardInner {
    const NAME: &'static str = "S76DaemonBoard";
    type ParentType = glib::Object;
    type Type = DaemonBoard;
}

impl ObjectImpl for DaemonBoardInner {}

glib::wrapper! {
    pub struct DaemonBoard(ObjectSubclass<DaemonBoardInner>);
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

        let self_ = glib::Object::new::<DaemonBoard>(&[]).unwrap();
        self_.inner().daemon.set(daemon);
        self_.inner().board.set(board);
        self_.inner().model.set(model);
        self_.inner().layout.set(layout);
        self_.inner().max_brightness.set(max_brightness);
        self_.inner().has_led_save.set(has_led_save);
        self_.inner().has_matrix.set(has_matrix);

        let keys = self_
            .inner()
            .layout
            .physical
            .keys
            .iter()
            .map(|i| Key::new(&self_, i))
            .collect();
        self_.inner().keys.set(keys);

        let layers = (0..num_layers)
            .map(|layer| Layer::new(&self_, layer))
            .collect();
        self_.inner().layers.set(layers);

        Ok(self_)
    }

    pub(crate) fn inner(&self) -> &DaemonBoardInner {
        DaemonBoardInner::from_instance(self)
    }

    pub(crate) fn daemon(&self) -> &dyn Daemon {
        self.inner().daemon.as_ref()
    }

    pub(crate) fn board(&self) -> BoardId {
        *self.inner().board
    }

    pub fn model(&self) -> &str {
        &self.inner().model
    }

    pub fn has_matrix(&self) -> bool {
        *self.inner().has_matrix
    }

    pub fn refresh_matrix(&self) -> Result<bool, String> {
        let matrix = self.daemon().matrix_get(self.board())?;
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
        *self.inner().max_brightness
    }

    pub fn led_save(&self) -> Result<(), String> {
        if self.has_led_save() && self.inner().leds_changed.get() {
            self.daemon().led_save(self.board())?;
            self.inner().leds_changed.set(false);
            debug!("led_save");
        }
        Ok(())
    }

    pub fn is_fake(&self) -> bool {
        self.daemon().is_fake()
    }

    pub fn has_led_save(&self) -> bool {
        *self.inner().has_led_save
    }

    pub fn layout(&self) -> &Layout {
        &*self.inner().layout
    }

    pub fn layers(&self) -> &[Layer] {
        &*self.inner().layers
    }

    pub fn keys(&self) -> &[Key] {
        &*self.inner().keys
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
