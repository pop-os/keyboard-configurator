use futures::{channel::mpsc as async_mpsc, prelude::*};
use glib::{
    prelude::*,
    subclass::{prelude::*, Signal},
    SignalHandlerId,
};
use once_cell::sync::Lazy;
use std::{cell::Cell, collections::HashMap, sync::Arc};

use crate::daemon::ThreadClient;
use crate::{BoardId, Daemon, DerefCell, Key, KeyMap, Layer, Layout, Matrix, Nelson};

#[derive(Default)]
#[doc(hidden)]
pub struct BoardInner {
    thread_client: DerefCell<Arc<ThreadClient>>,
    board: DerefCell<BoardId>,
    model: DerefCell<String>,
    layout: DerefCell<Layout>,
    keys: DerefCell<Vec<Key>>,
    layers: DerefCell<Vec<Layer>>,
    max_brightness: DerefCell<i32>,
    leds_changed: Cell<bool>,
    has_led_save: DerefCell<bool>,
    led_save_blocked: Cell<bool>,
    has_matrix: DerefCell<bool>,
    is_fake: DerefCell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for BoardInner {
    const NAME: &'static str = "S76DaemonBoard";
    type ParentType = glib::Object;
    type Type = Board;
}

impl ObjectImpl for BoardInner {
    fn signals() -> &'static [Signal] {
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![
                Signal::builder("leds-changed", &[], glib::Type::UNIT.into()).build(),
                Signal::builder("matrix-changed", &[], glib::Type::UNIT.into()).build(),
                Signal::builder("removed", &[], glib::Type::UNIT.into()).build(),
            ]
        });
        SIGNALS.as_ref()
    }
}

glib::wrapper! {
    pub struct Board(ObjectSubclass<BoardInner>);
}

unsafe impl Send for Board {}

impl Board {
    pub fn new(
        daemon: &dyn Daemon,
        thread_client: Arc<ThreadClient>,
        board: BoardId,
        mut matrix_reciever: async_mpsc::UnboundedReceiver<Matrix>,
    ) -> Result<Self, String> {
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

        let self_ = glib::Object::new::<Board>(&[]).unwrap();
        self_.inner().thread_client.set(thread_client);
        self_.inner().board.set(board);
        self_.inner().model.set(model);
        self_.inner().layout.set(layout);
        self_.inner().max_brightness.set(max_brightness);
        self_.inner().has_led_save.set(has_led_save);
        self_.inner().has_matrix.set(has_matrix);
        self_.inner().is_fake.set(daemon.is_fake());

        let keys = self_
            .layout()
            .physical
            .keys
            .iter()
            .map(|i| Key::new(daemon, &self_, i))
            .collect();
        self_.inner().keys.set(keys);

        let layers = (0..num_layers)
            .map(|layer| Layer::new(daemon, &self_, layer))
            .collect();
        self_.inner().layers.set(layers);

        {
            let self_ = self_.clone();
            glib::MainContext::default().spawn(async move {
                while let Some(matrix) = matrix_reciever.next().await {
                    for key in self_.keys() {
                        let pressed = matrix
                            .get(key.electrical.0 as usize, key.electrical.1 as usize)
                            .unwrap_or(false);
                        key.pressed.set(pressed);
                    }
                    self_.emit_by_name("matrix-changed", &[]).unwrap();
                }
            });
        }

        Ok(self_)
    }

    fn inner(&self) -> &BoardInner {
        BoardInner::from_instance(self)
    }

    pub fn connect_removed<F: Fn() + 'static>(&self, cb: F) -> SignalHandlerId {
        self.connect_local("removed", false, move |_| {
            cb();
            None
        })
        .unwrap()
    }

    pub(crate) fn set_leds_changed(&self) {
        self.inner().leds_changed.set(true);
        self.emit_by_name("leds-changed", &[]).unwrap();
    }

    pub fn connect_leds_changed<F: Fn() + 'static>(&self, cb: F) -> SignalHandlerId {
        self.connect_local("leds-changed", false, move |_| {
            cb();
            None
        })
        .unwrap()
    }

    pub fn board(&self) -> BoardId {
        *self.inner().board
    }

    pub(crate) fn thread_client(&self) -> &ThreadClient {
        &self.inner().thread_client
    }

    pub fn model(&self) -> &str {
        &self.inner().model
    }

    pub fn has_matrix(&self) -> bool {
        *self.inner().has_matrix
    }

    pub fn connect_matrix_changed<F: Fn() + 'static>(&self, cb: F) -> SignalHandlerId {
        self.connect_local("matrix-changed", false, move |_| {
            cb();
            None
        })
        .unwrap()
    }

    pub fn max_brightness(&self) -> i32 {
        *self.inner().max_brightness
    }

    pub async fn nelson(&self) -> Result<Nelson, String> {
        self.thread_client().nelson(self.board()).await
    }

    pub async fn led_save(&self) -> Result<(), String> {
        if self.inner().led_save_blocked.get() {
            return Ok(());
        }
        if self.has_led_save() && self.inner().leds_changed.get() {
            self.thread_client().led_save(self.board()).await?;
            self.inner().leds_changed.set(false);
            debug!("led_save");
        }
        Ok(())
    }

    pub fn block_led_save(&self) {
        self.inner().led_save_blocked.set(true);
    }

    pub fn unblock_led_save(&self) {
        self.inner().led_save_blocked.set(false);
    }

    pub fn is_fake(&self) -> bool {
        *self.inner().is_fake
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
