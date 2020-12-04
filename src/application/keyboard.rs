use cascade::cascade;
use glib::object::WeakRef;
use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use glib::translate::{FromGlibPtrFull, ToGlib, ToGlibPtr};
use once_cell::unsync::OnceCell;
use std::{
    cell::{
        Cell,
        RefCell,
    },
    collections::HashMap,
    ffi::OsStr,
    fs::{self, File},
    path::{
        Path,
    },
    rc::Rc,
    str,
};

use crate::daemon::Daemon;
use crate::keyboard::Keyboard as ColorKeyboard;
use crate::keyboard_color_button::KeyboardColorButton;
use crate::keymap::KeyMap;
use super::error_dialog::error_dialog;
use super::key::Key;
use super::layout::Layout;
use super::page::Page;
use super::picker::Picker;

pub struct KeyboardInner {
    board: OnceCell<String>,
    daemon: OnceCell<Rc<dyn Daemon>>,
    daemon_board: OnceCell<usize>,
    default_layout: OnceCell<KeyMap>,
    keymap: OnceCell<HashMap<String, u16>>,
    keys: OnceCell<Box<[Key]>>,
    load_button: gtk::Button,
    page: Cell<Page>,
    picker: RefCell<WeakRef<Picker>>,
    selected: Cell<Option<usize>>,
    color_button_bin: gtk::Frame,
    brightness_scale: gtk::Scale,
    save_button: gtk::Button,
    reset_button: gtk::Button,
    toolbar: gtk::Box,
    hbox: gtk::Box,
    stack: gtk::Stack,
}

impl ObjectSubclass for KeyboardInner {
    const NAME: &'static str = "S76Keyboard";

    type ParentType = gtk::Box;

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn new() -> Self {
        let stack = cascade! {
            gtk::Stack::new();
            ..set_transition_duration(0);
        };

        let brightness_label = cascade! {
            gtk::Label::new(Some("Brightness:"));
            ..set_halign(gtk::Align::Start);
        };

        let brightness_scale = cascade! {
            gtk::Scale::new::<gtk::Adjustment>(gtk::Orientation::Horizontal, None);
            ..set_increments(1.0, 1.0);
            ..set_halign(gtk::Align::Fill);
            ..set_size_request(200, 0);
        };

        let color_label = cascade! {
            gtk::Label::new(Some("Color:"));
            ..set_halign(gtk::Align::Start);
        };

        // XXX add support to ColorButton for changing keyboard
        let color_button_bin = cascade!{
            gtk::Frame::new(None);
            ..set_shadow_type(gtk::ShadowType::None);
            ..set_valign(gtk::Align::Center);
        };

        let stack_switcher = cascade! {
            gtk::StackSwitcher::new();
            ..set_stack(Some(&stack));
        };

        let toolbar = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 8);
            ..set_center_widget(Some(&stack_switcher));
        };

        let load_button = cascade! {
            gtk::Button::with_label("Load");
            ..set_valign(gtk::Align::Center);
        };

        let save_button = cascade! {
            gtk::Button::with_label("Save");
            ..set_valign(gtk::Align::Center);
        };

        let reset_button = cascade! {
            gtk::Button::with_label("Reset");
            ..set_valign(gtk::Align::Center);
        };

        let hbox = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 8);
            ..add(&brightness_label);
            ..add(&brightness_scale);
            ..add(&color_label);
            ..add(&color_button_bin);
            ..add(&load_button);
            ..add(&save_button);
            ..add(&reset_button);
        };

        Self {
            board: OnceCell::new(),
            daemon: OnceCell::new(),
            daemon_board: OnceCell::new(),
            default_layout: OnceCell::new(),
            keymap: OnceCell::new(),
            keys: OnceCell::new(),
            load_button,
            page: Cell::new(Page::Layer1),
            picker: RefCell::new(WeakRef::new()),
            save_button,
            reset_button,
            selected: Cell::new(None),
            color_button_bin,
            brightness_scale,
            toolbar,
            hbox,
            stack,
        }
    }
}

impl ObjectImpl for KeyboardInner {
    glib_object_impl!();

    fn constructed(&self, obj: &glib::Object) {
        self.parent_constructed(obj);

        let keyboard: &Keyboard = obj.downcast_ref().unwrap();
        keyboard.set_orientation(gtk::Orientation::Vertical);
        keyboard.set_spacing(8);
        keyboard.add(&keyboard.inner().toolbar);
        keyboard.add(&keyboard.inner().hbox);
        keyboard.add(&keyboard.inner().stack);
    }
}

impl WidgetImpl for KeyboardInner {}
impl ContainerImpl for KeyboardInner {}
impl BoxImpl for KeyboardInner {}

glib_wrapper! {
    pub struct Keyboard(
        Object<subclass::simple::InstanceStruct<KeyboardInner>,
        subclass::simple::ClassStruct<KeyboardInner>, KeyboardClass>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;

    match fn {
        get_type => || KeyboardInner::get_type().to_glib(),
    }
}

impl Keyboard {
    pub fn new<P: AsRef<Path>>(dir: P, board: &str, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Self {
        let dir = dir.as_ref();

        let default_json = fs::read_to_string(dir.join("default_json"))
            .expect("Failed to load keymap.csv");
        let keymap_csv = fs::read_to_string(dir.join("keymap.csv"))
            .expect("Failed to load keymap.csv");
        let layout_csv = fs::read_to_string(dir.join("layout.csv"))
            .expect("Failed to load layout.csv");
        let physical_json = fs::read_to_string(dir.join("physical.json"))
            .expect("Failed to load physical.json");
        Self::new_data(board, &default_json, &keymap_csv, &layout_csv, &physical_json, daemon, daemon_board)
    }

    fn new_layout(board: &str, layout: Layout, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Self {
        let keyboard: Self = glib::Object::new(Self::static_type(), &[])
            .unwrap()
            .downcast()
            .unwrap();

        let mut keys = layout.keys();
        for key in keys.iter_mut() {
            for layer in 0..2 {
                println!("  Layer {}", layer);
                let scancode = match daemon.keymap_get(daemon_board, layer, key.electrical.0, key.electrical.1) {
                    Ok(value) => value,
                    Err(err) => {
                        eprintln!("Failed to read scancode: {:?}", err);
                        0
                    }
                };
                println!("    Scancode: {:04X}", scancode);

                let scancode_name = match layout.scancode_names.get(&scancode) {
                    Some(some) => some.to_string(),
                    None => String::new(),
                };
                println!("    Scancode Name: {}", scancode_name);

                key.scancodes.borrow_mut().push((scancode, scancode_name));
            }
        }

        let _ = keyboard.inner().keys.set(keys.into_boxed_slice());

        let _ = keyboard.inner().board.set(board.to_string());
        let _ = keyboard.inner().daemon.set(daemon);
        let _ = keyboard.inner().daemon_board.set(daemon_board);
        let _ = keyboard.inner().keymap.set(layout.keymap);
        let _ = keyboard.inner().default_layout.set(layout.default);

        let color_keyboard = ColorKeyboard::new_daemon(keyboard.daemon().clone(), keyboard.daemon_board());
        let color_button = KeyboardColorButton::new(color_keyboard);
        keyboard.inner().color_button_bin.add(&color_button);

        let max_brightness = match keyboard.daemon().max_brightness(keyboard.daemon_board()) {
            Ok(value) => value as f64,
            Err(err) => {
                eprintln!("{}", err);
                100.0
            }
        };
        keyboard.inner().brightness_scale.set_range(0.0, max_brightness);

        keyboard.add_pages();
        keyboard.connect_signals();

        keyboard
    }

    pub fn new_board(board: &str, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Option<Self> {
        Layout::from_board(board).map(|layout|
            Self::new_layout(board, layout, daemon, daemon_board)
        )
    }

    fn new_data(board: &str, default_json: &str, keymap_csv: &str, layout_csv: &str, physical_json: &str, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Self {
        let layout = Layout::from_data(default_json, keymap_csv, layout_csv, physical_json);
        Self::new_layout(board, layout, daemon, daemon_board)
    }

    fn inner(&self) -> &KeyboardInner {
        KeyboardInner::from_instance(self)
    }

    fn board(&self) -> &str {
        self.inner().board.get().unwrap()
    }

    fn daemon(&self) -> &Rc<dyn Daemon> {
        self.inner().daemon.get().unwrap()
    }

    fn daemon_board(&self) -> usize {
        *self.inner().daemon_board.get().unwrap()
    }

    fn keymap(&self) -> &HashMap<String, u16> {
        self.inner().keymap.get().unwrap()
    }

    fn default_layout(&self) -> &KeyMap {
        self.inner().default_layout.get().unwrap()
    }

    fn window(&self) -> Option<gtk::Window> {
        self.get_toplevel()?.downcast().ok()
    }

    pub fn layer(&self) -> usize {
        //TODO: make this more robust
        match self.inner().page.get() {
            Page::Layer1 => 0,
            Page::Layer2 => 1,
            _ => 0, // Any other page selects Layer 1
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.inner().selected.get()
    }

    pub fn has_scancode(&self, scancode_name: &str) -> bool {
        self.keymap().contains_key(scancode_name)
    }

    fn keys(&self) -> &[Key] {
        self.inner().keys.get().unwrap()
    }

    pub fn keymap_set(&self, key_index: usize, layer: usize, scancode_name: &str) {
        let k = &self.keys()[key_index];
        let mut found = false;
        if let Some(scancode) = self.keymap().get(scancode_name) {
            k.scancodes.borrow_mut()[layer] = (*scancode, scancode_name.to_string());
            k.refresh();
            found = true;
        }
        if !found {
            return;
        }
        println!(
            "  set {}, {}, {} to {:04X}",
            layer, k.electrical.0, k.electrical.1, k.scancodes.borrow()[layer].0
        );
        if let Err(err) = self.daemon().keymap_set(
            self.daemon_board(),
            layer as u8,
            k.electrical.0,
            k.electrical.1,
            k.scancodes.borrow_mut()[layer].0,
        ) {
            eprintln!("Failed to set keymap: {:?}", err);
        }

        self.set_selected(self.selected());
    }

    pub fn export_keymap(&self) -> KeyMap {
        let mut map = HashMap::new();
        for key in self.keys() {
            let scancodes = key.scancodes.borrow();
            let scancodes = scancodes.iter().map(|s| s.1.clone()).collect();
            map.insert(key.logical_name.clone(), scancodes);
        }
        KeyMap {
            board: self.board().to_string(),
            map: map,
        }
    }

    pub fn import_keymap(&self, keymap: &KeyMap) {
        // TODO: don't block UI thread
        // TODO: Ideally don't want this function to be O(Keys^2)

        if &keymap.board != self.board() {
            error_dialog(&self.window().unwrap(),
                         "Failed to import keymap",
                         format!("Keymap is for board '{}'", keymap.board));
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

    fn connect_signals(&self) {
        let kb = self;

        self.inner().stack.connect_property_visible_child_notify(
            clone!(@weak kb => @default-panic, move |stack| {
                let page: Option<Page> = match stack.get_visible_child() {
                    Some(child) => unsafe { child.get_data("keyboard_confurator_page").cloned() },
                    None => None,
                };

                println!("{:?}", page);
                let last_layer = kb.layer();
                kb.inner().page.set(page.unwrap_or(Page::Layer1));
                if kb.layer() != last_layer {
                    kb.set_selected(kb.selected());
                }
            }),
        );

        self.inner().brightness_scale.connect_value_changed(
            clone!(@weak kb => @default-panic, move |this| {
                let value = this.get_value() as i32;
                if let Err(err) = kb.daemon().set_brightness(kb.daemon_board(), value) {
                    eprintln!("{}", err);
                }
                println!("{}", value);
            }),
        );

        self.inner().load_button.connect_clicked(clone!(@weak kb => @default-panic, move |_button| {
            let filter = cascade! {
                gtk::FileFilter::new();
                ..set_name(Some("JSON"));
                ..add_mime_type("application/json");
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
                        Ok(keymap) => kb.import_keymap(&keymap),
                        Err(err) => error_dialog(&kb.window().unwrap(), "Failed to import keymap", err),
                    }
                    Err(err) => error_dialog(&kb.window().unwrap(), "Failed to open file", err),
                }
            }
        }));

        self.inner().save_button.connect_clicked(clone!(@weak kb => @default-panic, move |_button| {
            let filter = cascade! {
                gtk::FileFilter::new();
                ..set_name(Some("JSON"));
                ..add_mime_type("application/json");
                ..add_pattern("*.json");
            };

            let chooser = cascade! {
                gtk::FileChooserNative::new::<gtk::Window>(Some("Save Layout"), None, gtk::FileChooserAction::Save, Some("Save"), Some("Cancel"));
                ..add_filter(&filter);
            };

            if chooser.run() == gtk::ResponseType::Accept {
                let mut path = chooser.get_filename().unwrap();
                match path.extension() {
                    None => { path.set_extension(OsStr::new("json")); }
                    Some(ext) if ext == OsStr::new("json") => {}
                    Some(ext) => {
                        let mut ext = ext.to_owned();
                        ext.push(".json");
                        path.set_extension(&ext);
                    }
                }
                let keymap = kb.export_keymap();

                match File::create(&path) {
                    Ok(file) => match keymap.to_writer_pretty(file) {
                        Ok(()) => {},
                        Err(err) => error_dialog(&kb.window().unwrap(), "Failed to export keymap", err),
                    }
                    Err(err) => error_dialog(&kb.window().unwrap(), "Failed to open file", err),
                }
            }
        }));

        self.inner().reset_button.connect_clicked(clone!(@weak kb => @default-panic, move |_button| {
            kb.import_keymap(kb.default_layout());
        }));
    }

    fn add_pages(&self) {
        let kb = self;

        for page in Page::iter_all() {
            let fixed = gtk::Fixed::new();
            self.inner().stack.add_titled(&fixed, page.name(), page.name());

            // TODO: Replace with something type-safe
            unsafe { fixed.set_data("keyboard_confurator_page", page) };

            let keys_len = self.keys().len();
            for i in 0..keys_len {
                let (button, label) = {
                    let keys = self.keys();
                    let k = &keys[i];

                    let scale = 64.0;
                    let margin = 2;
                    let x = (k.physical.x * scale) as i32 + margin;
                    let y = -(k.physical.y * scale) as i32 + margin;
                    let w = (k.physical.w * scale) as i32 - margin * 2;
                    let h = (k.physical.h * scale) as i32 - margin * 2;

                    let css = k.css();
                    let style_provider = cascade! {
                        gtk::CssProvider::new();
                        ..load_from_data(css.as_bytes()).expect("Failed to parse css");
                    };

                    let label = cascade! {
                        gtk::Label::new(None);
                        ..set_line_wrap(true);
                        ..set_margin_start(5);
                        ..set_margin_end(5);
                        ..set_justify(gtk::Justification::Center);
                    };

                    let button = cascade! {
                        gtk::Button::new();
                        ..set_focus_on_click(false);
                        ..set_size_request(w, h);
                        ..get_style_context().add_provider(&style_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
                        ..add(&label);
                    };

                    fixed.put(&button, x, y);

                    (button, label)
                };

                button.connect_clicked(clone!(@weak kb => @default-panic, move |_| {
                    // Deselect
                    if kb.inner().selected.get() == Some(i) {
                        kb.set_selected(None);
                    } else {
                        kb.set_selected(Some(i));
                    }
                }));

                let k = &self.keys()[i];
                k.gtk.borrow_mut().insert(page, (button, label));
            }
        }

        for k in self.keys() {
            k.refresh();
        }
    }

    pub(super) fn set_picker(&self, picker: Option<&Picker>) {
        // This function is called by Picker::set_keyboard()
        *self.inner().picker.borrow_mut() = match picker {
            Some(picker) => picker.downgrade(),
            None => WeakRef::new(),
        };
    }

    fn set_selected(&self, i: Option<usize>) {
        let picker = match self.inner().picker.borrow().upgrade() {
            Some(picker) => picker,
            None => { return; },
        };
        let keys = self.keys();

        if let Some(selected) = self.selected() {
            for (_page, (button, _label)) in keys[selected].gtk.borrow().iter() {
                button.get_style_context().remove_class("selected");
            }
            picker.set_selected(None);
        }

        if let Some(i) = i {
            let k = &keys[i];
            println!("{:#?}", k);
            for (_page, (button, _label)) in keys[i].gtk.borrow().iter() {
                button.get_style_context().add_class("selected");
            }
            if let Some((_scancode, scancode_name)) = keys[i].scancodes.borrow().get(self.layer()) {
                picker.set_selected(Some(scancode_name.to_string()));
            }
        }

        self.inner().selected.set(i);
    }
}
