use futures::channel::mpsc as async_mpsc;
use once_cell::sync::{Lazy, OnceCell};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard, Weak,
    },
};

use crate::daemon::ThreadClient;
use crate::{
    Benchmark, BoardId, Daemon, Event, Key, KeyMap, KeyMapLayer, Layer, Layout, Matrix, Nelson,
    NelsonKind,
};

#[derive(Clone, Debug)]
pub enum BoardEvent {
    KeymapChanged,
    LedsChanged,
    MatrixChanged,
}

#[derive(Debug)]
struct BoardInner {
    thread_client: Arc<ThreadClient>,
    board: BoardId,
    model: String,
    version: String,
    layout: Layout,
    keys: OnceCell<Vec<Key>>,
    layers: OnceCell<Vec<Layer>>,
    max_brightness: i32,
    leds_changed: AtomicBool,
    has_led_save: bool,
    led_save_blocked: AtomicBool,
    has_matrix: bool,
    is_fake: bool,
    has_keymap: bool,
    matrix: Arc<Mutex<Matrix>>,
    updated: bool,
    event_sender: async_mpsc::UnboundedSender<Event>,
}

#[derive(Clone, Debug)]
pub struct Board(Arc<BoardInner>);

impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Board {}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Hash, Eq)]
pub enum Bootloaded {
    // Launch 2, Launch Heavy 1,
    At90usb646,
    // Launch Lite 1
    At90usb646Lite,
    // Launch 1
    AtMega32u4,
}

#[derive(Debug)]
pub(crate) struct WeakBoard(Weak<BoardInner>);

impl Board {
    pub fn new(
        daemon: &dyn Daemon,
        thread_client: Arc<ThreadClient>,
        board: BoardId,
        matrix: Arc<Mutex<Matrix>>,
        event_sender: async_mpsc::UnboundedSender<Event>,
    ) -> Result<Self, String> {
        let model = match daemon.model(board) {
            Ok(model) => model,
            Err(err) => {
                return Err(format!("Failed to get board model: {}", err));
            }
        };
        let version = daemon.version(board).unwrap_or_else(|err| {
            error!("Error getting firmware version: {}", err);
            String::new()
        });
        let layout = Layout::from_board(&model, &version)
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
        let logical = layout.layout.values().next().unwrap();
        let has_keymap = daemon.keymap_get(board, 0, logical.0, logical.1).is_ok();

        let self_ = Board(Arc::new(BoardInner {
            thread_client,
            board,
            model,
            version,
            layout,
            max_brightness,
            has_led_save,
            has_matrix,
            is_fake: daemon.is_fake(),
            has_keymap,
            keys: OnceCell::new(),
            layers: OnceCell::new(),
            leds_changed: AtomicBool::new(false),
            led_save_blocked: AtomicBool::new(false),
            matrix,
            event_sender,
            updated: is_launch_updated().unwrap_or(false),
        }));

        let keys = self_
            .layout()
            .physical
            .keys
            .iter()
            .map(|i| Key::new(daemon, &self_, i))
            .collect();
        self_.0.keys.set(keys).unwrap();

        let layers = (0..num_layers)
            .map(|layer| Layer::new(daemon, &self_, layer))
            .collect();
        self_.0.layers.set(layers).unwrap();

        Ok(self_)
    }

    pub(crate) fn send_event(&self, event: BoardEvent) {
        let _ = self
            .0
            .event_sender
            .unbounded_send(Event::Board(self.0.board, event));
    }

    pub(crate) fn set_leds_changed(&self) {
        self.0.leds_changed.store(true, Ordering::SeqCst);
        self.send_event(BoardEvent::LedsChanged);
    }

    pub fn board(&self) -> BoardId {
        self.0.board
    }

    pub(crate) fn thread_client(&self) -> &ThreadClient {
        &self.0.thread_client
    }

    pub fn model(&self) -> &str {
        &self.0.model
    }

    pub fn version(&self) -> &str {
        &self.0.version
    }

    pub fn has_matrix(&self) -> bool {
        self.0.has_matrix
    }

    pub fn max_brightness(&self) -> i32 {
        self.0.max_brightness
    }

    pub async fn benchmark(&self) -> Result<Benchmark, String> {
        self.thread_client().benchmark(self.board()).await
    }

    pub async fn nelson(&self, kind: NelsonKind) -> Result<Nelson, String> {
        self.thread_client().nelson(self.board(), kind).await
    }

    pub async fn led_save(&self) -> Result<(), String> {
        if self.0.led_save_blocked.load(Ordering::SeqCst) {
            return Ok(());
        }
        if self.has_led_save() && self.0.leds_changed.load(Ordering::SeqCst) {
            self.thread_client().led_save(self.board()).await?;
            self.0.leds_changed.store(false, Ordering::SeqCst);
            debug!("led_save");
        }
        Ok(())
    }

    pub fn block_led_save(&self) {
        self.0.led_save_blocked.store(true, Ordering::SeqCst);
    }

    pub fn unblock_led_save(&self) {
        self.0.led_save_blocked.store(false, Ordering::SeqCst);
    }

    pub fn is_fake(&self) -> bool {
        self.0.is_fake
    }

    pub fn is_lite(&self) -> bool {
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new("system76/launch_lite_.*").unwrap());
        RE.is_match(self.model())
    }

    pub fn is_updated(&self) -> bool {
        self.0.updated
    }

    pub fn has_led_save(&self) -> bool {
        self.0.has_led_save
    }

    pub fn has_keymap(&self) -> bool {
        self.0.has_keymap
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

    pub(crate) fn matrix(&self) -> MutexGuard<Matrix> {
        self.0.matrix.lock().unwrap()
    }

    pub fn export_keymap(&self) -> KeyMap {
        let mut map = HashMap::new();
        let mut key_leds = HashMap::new();
        for key in self.keys().iter() {
            let scancodes = (0..self.layout().meta.num_layers as usize)
                .map(|layer| key.get_scancode(layer).unwrap().1)
                .collect();
            map.insert(key.logical_name.clone(), scancodes);
            if !key.leds.is_empty() {
                key_leds.insert(key.logical_name.clone(), key.color());
            }
        }
        let layers = self
            .layers()
            .iter()
            .map(|layer| KeyMapLayer {
                mode: *layer.mode.lock().unwrap(),
                brightness: layer.brightness(),
                color: layer.color(),
            })
            .collect();
        KeyMap {
            model: self.model().to_string(),
            version: 1,
            map,
            key_leds,
            layers,
        }
    }

    pub async fn set_no_input(&self, no_input: bool) -> Result<(), String> {
        self.thread_client()
            .set_no_input(self.board(), no_input)
            .await
    }

    pub(crate) fn downgrade(&self) -> WeakBoard {
        WeakBoard(Arc::downgrade(&self.0))
    }
}

impl WeakBoard {
    pub fn upgrade(&self) -> Option<Board> {
        Some(Board(self.0.upgrade()?))
    }
}

pub fn is_launch_updated() -> Result<bool, String> {
    use regex::bytes::Regex;
    let stdout = Command::new("fwupdmgr")
        .args(["get-updates", "--json"])
        .output()
        .map_err(|e| format!("Failed to use fwupdmgr: {}", e))?
        .stdout;

    static RE: Lazy<Regex> = Lazy::new(|| Regex::new("Launch.* Configurable Keyboard").unwrap());
    Ok(!RE.is_match(&stdout))
}
