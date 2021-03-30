use cascade::cascade;
use glib::clone;
use glib::object::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    convert::TryFrom,
    ffi::OsStr,
    fs::File,
    rc::Rc,
    str, time,
};

use super::{show_error_dialog, Backlight, Key, KeyboardLayer, Layout, Page, Picker};
use crate::DerefCell;
use daemon::{DaemonBoard, KeyMap};

#[derive(Default)]
pub struct KeyboardInner {
    action_group: DerefCell<gio::SimpleActionGroup>,
    board: DerefCell<DaemonBoard>,
    board_name: DerefCell<String>,
    keys: DerefCell<Rc<[Key]>>,
    layout: DerefCell<Rc<Layout>>,
    page: Cell<Page>,
    picker: RefCell<WeakRef<Picker>>,
    selected: Cell<Option<usize>>,
    layer_stack: DerefCell<gtk::Stack>,
    stack: DerefCell<gtk::Stack>,
    picker_box: DerefCell<gtk::Box>,
    backlight: DerefCell<Backlight>,
    has_matrix: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for KeyboardInner {
    const NAME: &'static str = "S76Keyboard";
    type ParentType = gtk::Box;
    type Type = Keyboard;
}

impl ObjectImpl for KeyboardInner {
    fn constructed(&self, keyboard: &Keyboard) {
        self.parent_constructed(keyboard);

        let layer_stack = cascade! {
            gtk::Stack::new();
            ..set_transition_duration(0);
            ..connect_property_visible_child_notify(
                clone!(@weak keyboard => move |stack| {
                    let page = stack
                        .get_visible_child()
                        .map(|c| c.downcast_ref::<KeyboardLayer>().unwrap().page());

                    debug!("{:?}", page);
                    let last_layer = keyboard.layer();
                    keyboard.inner().page.set(page.unwrap_or(Page::Layer1));
                    let layer = keyboard.layer();
                    if layer != last_layer {
                        keyboard.set_selected(keyboard.selected());
                        keyboard.inner().backlight.set_sensitive(layer.is_some());
                        if let Some(layer) = layer {
                            keyboard.inner().backlight.set_layer(layer as u8);
                        }
                    }
                })
            );
        };

        let picker_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let stack = cascade! {
            gtk::Stack::new();
            ..add_titled(&picker_box, "keymap", "Keymap");
        };

        let stack_switcher = cascade! {
            gtk::StackSwitcher::new();
            ..set_halign(gtk::Align::Center);
            ..set_margin_top(8);
            ..set_stack(Some(&stack));
        };

        cascade! {
            keyboard;
            ..set_orientation(gtk::Orientation::Vertical);
            ..set_spacing(8);
            ..add(&stack_switcher);
            ..add(&layer_stack);
            ..add(&stack);
        };

        let action_group = cascade! {
            gio::SimpleActionGroup::new();
            ..add_action(&cascade! {
                gio::SimpleAction::new("load", None);
                ..connect_activate(clone!(@weak keyboard => move |_, _|
                    keyboard.load();
                ));
            });
            ..add_action(&cascade! {
                gio::SimpleAction::new("save", None);
                ..connect_activate(clone!(@weak keyboard => move |_, _|
                    keyboard.save();
                ));
            });
            ..add_action(&cascade! {
                gio::SimpleAction::new("reset", None);
                ..connect_activate(clone!(@weak keyboard => move |_, _|
                    keyboard.reset();
                ));
            });
        };

        self.action_group.set(action_group);
        self.layer_stack.set(layer_stack);
        self.stack.set(stack);
        self.picker_box.set(picker_box);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpec::int(
                "selected",
                "selected",
                "selected",
                -1,
                i32::MAX,
                -1,
                glib::ParamFlags::READWRITE,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        keyboard: &Keyboard,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.get_name() {
            "selected" => {
                let v: i32 = value.get_some().unwrap();
                let selected = usize::try_from(v).ok();
                keyboard.set_selected(selected);
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(
        &self,
        keyboard: &Keyboard,
        _id: usize,
        pspec: &glib::ParamSpec,
    ) -> glib::Value {
        match pspec.get_name() {
            "selected" => keyboard
                .selected()
                .map(|v| v as i32)
                .unwrap_or(-1)
                .to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for KeyboardInner {}
impl ContainerImpl for KeyboardInner {}
impl BoxImpl for KeyboardInner {}

glib::wrapper! {
    pub struct Keyboard(ObjectSubclass<KeyboardInner>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

impl Keyboard {
    fn new_layout(
        board_name: &str,
        layout: Layout,
        board: DaemonBoard,
        debug_layers: bool,
    ) -> Self {
        let keyboard: Self = glib::Object::new(&[]).unwrap();
        let layout = Rc::new(layout);

        let mut keys = layout.keys();
        for key in keys.iter_mut() {
            for layer in 0..layout.meta.num_layers {
                debug!("  Layer {}", layer);
                let scancode = match board.keymap_get(layer, key.electrical.0, key.electrical.1) {
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

        let keys: Rc<[Key]> = keys.into_boxed_slice().into();

        let backlight = cascade! {
            Backlight::new(board.clone(), keys.clone(), layout.clone());
            ..set_halign(gtk::Align::Center);
        };
        keyboard
            .bind_property("selected", &backlight, "selected")
            .build();
        keyboard
            .inner()
            .stack
            .add_titled(&backlight, "leds", "LEDs");

        keyboard.inner().keys.set(keys);
        keyboard.inner().has_matrix.set(board.matrix_get().is_ok());
        keyboard.inner().board.set(board);
        keyboard.inner().board_name.set(board_name.to_string());
        keyboard.inner().layout.set(layout);
        keyboard.inner().backlight.set(backlight);

        keyboard.add_pages(debug_layers);

        glib::timeout_add_local(
            time::Duration::from_millis(50),
            clone!(@weak keyboard => @default-return glib::Continue(false), move || {
                glib::Continue(keyboard.refresh())
            }),
        );

        keyboard
    }

    pub fn new_board(board_name: &str, board: DaemonBoard, debug_layers: bool) -> Option<Self> {
        Layout::from_board(board_name)
            .map(|layout| Self::new_layout(board_name, layout, board, debug_layers))
    }

    fn inner(&self) -> &KeyboardInner {
        KeyboardInner::from_instance(self)
    }

    pub fn action_group(&self) -> &gio::ActionGroup {
        self.inner().action_group.upcast_ref()
    }

    fn board_name(&self) -> &str {
        &self.inner().board_name
    }

    fn board(&self) -> &DaemonBoard {
        &self.inner().board
    }

    pub fn display_name(&self) -> String {
        let name = &self.layout().meta.display_name;
        let board_name = self.board_name().splitn(2, "/").skip(1).next().unwrap();
        if self.board().0.is_fake() {
            format!("{} ({}, fake)", name, board_name)
        } else {
            format!("{} ({})", name, board_name)
        }
    }

    fn layout(&self) -> &Layout {
        &self.inner().layout
    }

    fn window(&self) -> Option<gtk::Window> {
        self.get_toplevel()?.downcast().ok()
    }

    pub fn layer(&self) -> Option<usize> {
        self.inner().page.get().layer()
    }

    pub fn selected(&self) -> Option<usize> {
        self.inner().selected.get()
    }

    pub fn layer_stack(&self) -> &gtk::Stack {
        &self.inner().layer_stack
    }

    pub fn has_scancode(&self, scancode_name: &str) -> bool {
        self.layout().keymap.contains_key(scancode_name)
    }

    pub fn keys(&self) -> &Rc<[Key]> {
        &self.inner().keys
    }

    pub fn keymap_set(&self, key_index: usize, layer: usize, scancode_name: &str) {
        let k = &self.keys()[key_index];
        let mut found = false;
        if let Some(scancode) = self.layout().keymap.get(scancode_name) {
            k.scancodes.borrow_mut()[layer] = (*scancode, scancode_name.to_string());
            found = true;
        }
        if !found {
            return;
        }
        info!(
            "  set {}, {}, {} to {:04X}",
            layer,
            k.electrical.0,
            k.electrical.1,
            k.scancodes.borrow()[layer].0
        );
        if let Err(err) = self.board().keymap_set(
            layer as u8,
            k.electrical.0,
            k.electrical.1,
            k.scancodes.borrow_mut()[layer].0,
        ) {
            error!("Failed to set keymap: {:?}", err);
        }

        self.set_selected(self.selected());
    }

    pub fn export_keymap(&self) -> KeyMap {
        let mut map = HashMap::new();
        for key in self.keys().iter() {
            let scancodes = key.scancodes.borrow();
            let scancodes = scancodes.iter().map(|s| s.1.clone()).collect();
            map.insert(key.logical_name.clone(), scancodes);
        }
        KeyMap {
            board: self.board_name().to_string(),
            map,
        }
    }

    pub fn import_keymap(&self, keymap: &KeyMap) {
        // TODO: don't block UI thread
        // TODO: Ideally don't want this function to be O(Keys^2)

        if keymap.board != self.board_name() {
            show_error_dialog(
                &self.window().unwrap(),
                "Failed to import keymap",
                format!("Keymap is for board '{}'", keymap.board),
            );
            return;
        }

        for (k, v) in keymap.map.iter() {
            let n = self
                .keys()
                .iter()
                .position(|i| &i.logical_name == k)
                .unwrap();
            for (layer, scancode_name) in v.iter().enumerate() {
                self.keymap_set(n, layer, scancode_name);
            }
        }
    }

    fn load(&self) {
        let filter = cascade! {
            gtk::FileFilter::new();
            ..set_name(Some("JSON"));
            ..add_pattern("*.json");
        };

        let chooser = cascade! {
            gtk::FileChooserNative::new::<gtk::Window>(Some("Load Layout"), None, gtk::FileChooserAction::Open, Some("Load"), Some("Cancel"));
            ..add_filter(&filter);
        };

        if chooser.run() == gtk::ResponseType::Accept {
            let path = chooser.get_filename().unwrap();
            match File::open(&path) {
                Ok(file) => match KeyMap::from_reader(file) {
                    Ok(keymap) => self.import_keymap(&keymap),
                    Err(err) => {
                        show_error_dialog(&self.window().unwrap(), "Failed to import keymap", err)
                    }
                },
                Err(err) => show_error_dialog(&self.window().unwrap(), "Failed to open file", err),
            }
        }
    }

    fn save(&self) {
        let filter = cascade! {
            gtk::FileFilter::new();
            ..set_name(Some("JSON"));
            ..add_pattern("*.json");
        };

        let chooser = cascade! {
            gtk::FileChooserNative::new::<gtk::Window>(Some("Save Layout"), None, gtk::FileChooserAction::Save, Some("Save"), Some("Cancel"));
            ..add_filter(&filter);
        };

        if chooser.run() == gtk::ResponseType::Accept {
            let mut path = chooser.get_filename().unwrap();
            match path.extension() {
                None => {
                    path.set_extension(OsStr::new("json"));
                }
                Some(ext) if ext == OsStr::new("json") => {}
                Some(ext) => {
                    let mut ext = ext.to_owned();
                    ext.push(".json");
                    path.set_extension(&ext);
                }
            }
            let keymap = self.export_keymap();

            match File::create(&path) {
                Ok(file) => match keymap.to_writer_pretty(file) {
                    Ok(()) => {}
                    Err(err) => {
                        show_error_dialog(&self.window().unwrap(), "Failed to export keymap", err)
                    }
                },
                Err(err) => show_error_dialog(&self.window().unwrap(), "Failed to open file", err),
            }
        }
    }

    fn reset(&self) {
        self.import_keymap(&self.layout().default);
    }

    fn add_pages(&self, debug_layers: bool) {
        let layer_stack = &*self.inner().layer_stack;

        for (i, page) in Page::iter_all().enumerate() {
            if !debug_layers && page.is_debug() {
                continue;
            } else if let Some(layer) = page.layer() {
                if layer >= self.layout().meta.num_layers.into() {
                    continue;
                }
            }

            let keyboard_layer = KeyboardLayer::new(page, self.keys().clone());
            self.bind_property("selected", &keyboard_layer, "selected")
                .flags(glib::BindingFlags::BIDIRECTIONAL)
                .build();
            layer_stack.add_titled(&keyboard_layer, page.name(), page.name());

            self.inner().action_group.add_action(&cascade! {
                gio::SimpleAction::new(&format!("page{}", i), None);
                ..connect_activate(clone!(@weak layer_stack, @weak keyboard_layer => move |_, _|
                    layer_stack.set_visible_child(&keyboard_layer);
                ));
            });
        }
    }

    pub(super) fn set_picker(&self, picker: Option<&Picker>) {
        // This function is called by Picker::set_keyboard()
        *self.inner().picker.borrow_mut() = match picker {
            Some(picker) => {
                if let Some(widget) = picker.get_parent() {
                    widget.downcast::<gtk::Container>().unwrap().remove(picker);
                }
                self.inner().picker_box.add(picker);
                picker.set_sensitive(self.selected().is_some() && self.layer() != None);
                picker.downgrade()
            }
            None => WeakRef::new(),
        };
    }

    fn set_selected(&self, i: Option<usize>) {
        let picker = match self.inner().picker.borrow().upgrade() {
            Some(picker) => picker,
            None => {
                return;
            }
        };
        let keys = self.keys();

        picker.set_selected(None);

        if let Some(i) = i {
            let k = &keys[i];
            debug!("{:#?}", k);
            if let Some(layer) = self.layer() {
                if let Some((_scancode, scancode_name)) = keys[i].scancodes.borrow().get(layer) {
                    picker.set_selected(Some(scancode_name.to_string()));
                }
            }
        }

        picker.set_sensitive(i.is_some() && self.layer() != None);

        self.inner().selected.set(i);

        self.queue_draw();
        self.notify("selected");
    }

    fn redraw(&self) {
        self.queue_draw();
        // TODO: clean up this hack to only redraw keyboard on main page
        if let Some(parent) = self.get_parent() {
            parent.queue_draw();
            if let Some(grandparent) = parent.get_parent() {
                grandparent.queue_draw();
            }
        }
    }

    fn refresh(&self) -> bool {
        if !self.inner().has_matrix.get() {
            return false;
        }

        let window = match self.window() {
            Some(some) => some,
            None => return true,
        };

        if !window.is_active() {
            return true;
        }

        match self.board().matrix_get() {
            Ok(matrix) => {
                let mut changed = false;
                for key in self.keys().iter() {
                    let pressed = matrix
                        .get(key.electrical.0 as usize, key.electrical.1 as usize)
                        .unwrap_or(false);
                    changed |= key.pressed.replace(pressed) != pressed;
                }
                if changed {
                    let keyboard = self;
                    keyboard.redraw();
                    // Sometimes the redraw is missed, so send it again in 10ms
                    glib::timeout_add_local(
                        time::Duration::from_millis(10),
                        clone!(@weak keyboard => @default-return glib::Continue(false), move || {
                            keyboard.redraw();
                            glib::Continue(false)
                        }),
                    );
                }
                true
            }
            Err(err) => {
                error!("Failed to get matrix: {}", err);
                false
            }
        }
    }
}
