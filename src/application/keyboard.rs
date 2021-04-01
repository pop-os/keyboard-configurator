use cascade::cascade;
use glib::clone;
use glib::object::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{
    cell::{Cell, RefCell},
    convert::TryFrom,
    ffi::OsStr,
    fs::File,
    str, time,
};

use super::{show_error_dialog, Backlight, KeyboardLayer, Page, Picker};
use crate::DerefCell;
use backend::{DaemonBoard, KeyMap, Layout};

#[derive(Default)]
pub struct KeyboardInner {
    action_group: DerefCell<gio::SimpleActionGroup>,
    board: DerefCell<DaemonBoard>,
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
    pub fn new(board: DaemonBoard, debug_layers: bool) -> Self {
        let keyboard: Self = glib::Object::new(&[]).unwrap();

        let backlight = cascade! {
            Backlight::new(board.clone());
            ..set_halign(gtk::Align::Center);
        };
        keyboard
            .bind_property("selected", &backlight, "selected")
            .build();
        keyboard
            .inner()
            .stack
            .add_titled(&backlight, "leds", "LEDs");

        keyboard.inner().has_matrix.set(board.matrix_get().is_ok());
        keyboard.inner().board.set(board);
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

    fn inner(&self) -> &KeyboardInner {
        KeyboardInner::from_instance(self)
    }

    pub fn action_group(&self) -> &gio::ActionGroup {
        self.inner().action_group.upcast_ref()
    }

    pub fn board(&self) -> &DaemonBoard {
        &self.inner().board
    }

    pub fn display_name(&self) -> String {
        let name = &self.layout().meta.display_name;
        let model = self.board().model().splitn(2, '/').nth(1).unwrap();
        if self.board().is_fake() {
            format!("{} ({}, fake)", name, model)
        } else {
            format!("{} ({})", name, model)
        }
    }

    fn layout(&self) -> &Layout {
        &self.inner().board.layout()
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

    pub fn keymap_set(&self, key_index: usize, layer: usize, scancode_name: &str) {
        if let Err(err) = self.board().keys()[key_index].set_scancode(layer, scancode_name) {
            error!("Failed to set keymap: {:?}", err);
        }

        self.set_selected(self.selected());
    }

    pub fn export_keymap(&self) -> KeyMap {
        self.board().export_keymap()
    }

    pub fn import_keymap(&self, keymap: &KeyMap) {
        // TODO: don't block UI thread
        // TODO: Ideally don't want this function to be O(Keys^2)

        if keymap.board != self.board().model() {
            show_error_dialog(
                &self.window().unwrap(),
                "Failed to import keymap",
                format!("Keymap is for board '{}'", keymap.board),
            );
            return;
        }

        for (k, v) in keymap.map.iter() {
            let n = self
                .board()
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

            let keyboard_layer = KeyboardLayer::new(page, self.board().clone());
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
        let keys = self.board().keys();

        picker.set_selected(None);

        if let Some(i) = i {
            let k = &keys[i];
            debug!("{:#?}", k);
            if let Some(layer) = self.layer() {
                if let Some((_scancode, scancode_name)) = keys[i].get_scancode(layer) {
                    picker.set_selected(Some(scancode_name));
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
                for key in self.board().keys().iter() {
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
