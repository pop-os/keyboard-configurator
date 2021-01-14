use cascade::cascade;
use gio::prelude::*;
use glib::object::WeakRef;
use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::unsync::OnceCell;
use std::{
    cell::{
        Cell,
        RefCell,
    },
    collections::HashMap,
    f64::consts::PI,
    ffi::OsStr,
    fs::{self, File},
    path::{
        Path,
    },
    rc::Rc,
    str,
};

use crate::color::Rgb;
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
    action_group: gio::SimpleActionGroup,
    board: OnceCell<String>,
    daemon: OnceCell<Rc<dyn Daemon>>,
    daemon_board: OnceCell<usize>,
    default_layout: OnceCell<KeyMap>,
    keymap: OnceCell<HashMap<String, u16>>,
    keys: OnceCell<Box<[Key]>>,
    load_action: gio::SimpleAction,
    page: Cell<Page>,
    picker: RefCell<WeakRef<Picker>>,
    selected: Cell<Option<usize>>,
    color_button_bin: gtk::Frame,
    brightness_scale: gtk::Scale,
    save_action: gio::SimpleAction,
    reset_action: gio::SimpleAction,
    hbox: gtk::Box,
    stack: gtk::Stack,
}

impl ObjectSubclass for KeyboardInner {
    const NAME: &'static str = "S76Keyboard";

    type ParentType = gtk::Box;
    type Type = Keyboard;

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

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


        let load_action = cascade! {
            gio::SimpleAction::new("load", None);
        };

        let save_action = cascade! {
            gio::SimpleAction::new("save", None);
        };

        let reset_action = cascade! {
            gio::SimpleAction::new("reset", None);
        };

        let action_group = cascade! {
            gio::SimpleActionGroup::new();
            ..add_action(&load_action);
            ..add_action(&save_action);
            ..add_action(&reset_action);
        };

        let hbox = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 8);
            ..add(&brightness_label);
            ..add(&brightness_scale);
            ..add(&color_label);
            ..add(&color_button_bin);
        };

        Self {
            action_group,
            board: OnceCell::new(),
            daemon: OnceCell::new(),
            daemon_board: OnceCell::new(),
            default_layout: OnceCell::new(),
            keymap: OnceCell::new(),
            keys: OnceCell::new(),
            load_action,
            page: Cell::new(Page::Layer1),
            picker: RefCell::new(WeakRef::new()),
            save_action,
            reset_action,
            selected: Cell::new(None),
            color_button_bin,
            brightness_scale,
            hbox,
            stack,
        }
    }
}

impl ObjectImpl for KeyboardInner {
    fn constructed(&self, keyboard: &Keyboard) {
        self.parent_constructed(keyboard);

        keyboard.set_orientation(gtk::Orientation::Vertical);
        keyboard.set_spacing(8);
        keyboard.add(&keyboard.inner().hbox);
        keyboard.add(&keyboard.inner().stack);
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
    #[allow(dead_code)]
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
        let keyboard: Self = glib::Object::new(&[]).unwrap();

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

        let brightness = match keyboard.daemon().brightness(keyboard.daemon_board()) {
            Ok(value) => value as f64,
            Err(err) => {
                eprintln!("{}", err);
                0.0
            }
        };
        keyboard.inner().brightness_scale.set_value(brightness);

        keyboard.add_pages();
        keyboard.connect_signals();

        keyboard
    }

    pub fn new_board(board: &str, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Option<Self> {
        Layout::from_board(board).map(|layout|
            Self::new_layout(board, layout, daemon, daemon_board)
        )
    }

    #[allow(dead_code)]
    fn new_data(board: &str, default_json: &str, keymap_csv: &str, layout_csv: &str, physical_json: &str, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Self {
        let layout = Layout::from_data(default_json, keymap_csv, layout_csv, physical_json);
        Self::new_layout(board, layout, daemon, daemon_board)
    }

    fn inner(&self) -> &KeyboardInner {
        KeyboardInner::from_instance(self)
    }

    pub fn action_group(&self) -> &gio::ActionGroup {
        self.inner().action_group.upcast_ref()
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

    pub fn layer(&self) -> Option<usize> {
        match self.inner().page.get() {
            Page::Layer1 => Some(0),
            Page::Layer2 => Some(1),
            _ => None
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.inner().selected.get()
    }

    pub fn stack(&self) -> &gtk::Stack {
        &self.inner().stack
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
                    Some(child) => unsafe { child.get_data("keyboard_configurator_page").cloned() },
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

        self.inner().load_action.connect_activate(clone!(@weak kb => @default-panic, move |_, _| {
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
                        Ok(keymap) => kb.import_keymap(&keymap),
                        Err(err) => error_dialog(&kb.window().unwrap(), "Failed to import keymap", err),
                    }
                    Err(err) => error_dialog(&kb.window().unwrap(), "Failed to open file", err),
                }
            }
        }));

        self.inner().save_action.connect_activate(clone!(@weak kb => @default-panic, move |_, _| {
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

        self.inner().reset_action.connect_activate(clone!(@weak kb => @default-panic, move |_, _| {
            kb.import_keymap(kb.default_layout());
        }));
    }

    fn add_pages(&self) {
        let kb = self;

        for page in Page::iter_all() {
            const SCALE: f64 = 64.0;
            const MARGIN: f64 = 2.;
            const RADIUS: f64 = 4.;

            let (width, height) = self.keys().iter().map(|k| {
                let w = (k.physical.w + k.physical.x) * SCALE - MARGIN;
                let h = (k.physical.h - k.physical.y) * SCALE - MARGIN;
                (w as i32, h as i32)
            }).max().unwrap();

            let drawing_area = cascade!{
                gtk::DrawingArea::new();
                ..set_size_request(width, height);
                ..add_events(gdk::EventMask::BUTTON_PRESS_MASK);
            };

            drawing_area.connect_draw(clone!(@weak kb => @default-panic, move |drawing_area, cr| {
                let selected = Rgb::new(0xfb, 0xb8, 0x6c).to_floats();
                for (i, k) in kb.keys().iter().enumerate() {
                    let x = (k.physical.x * SCALE) + MARGIN;
                    let y = -(k.physical.y * SCALE) + MARGIN;
                    let w = (k.physical.w * SCALE) - MARGIN * 2.;
                    let h = (k.physical.h * SCALE) - MARGIN * 2.;

                    let bg = k.background_color.to_floats();
                    let fg = k.foreground_color.to_floats();

                    // Rounded rectangle
                    cr.new_sub_path();
                    cr.arc(x + w - RADIUS, y + RADIUS, RADIUS, -0.5 * PI, 0.);
                    cr.arc(x + w - RADIUS, y + h - RADIUS, RADIUS, 0., 0.5 * PI);
                    cr.arc(x + RADIUS, y + h - RADIUS, RADIUS, 0.5 * PI, PI);
                    cr.arc(x + RADIUS, y + RADIUS, RADIUS, PI, 1.5 * PI);
                    cr.close_path();

                    cr.set_source_rgb(bg.0, bg.1, bg.2);
                    cr.fill_preserve();

                    if kb.selected() == Some(i) {
                        cr.set_source_rgb(selected.0, selected.1, selected.2);
                        cr.set_line_width(4.);
                        cr.stroke();
                    }

                    // Draw label
                    let text = k.get_label(page);
                    let layout = cascade! {
                        drawing_area.create_pango_layout(Some(&text));
                        ..set_width((w * pango::SCALE as f64) as i32);
                        ..set_alignment(pango::Alignment::Center);
                    };
                    let text_height = layout.get_pixel_size().1 as f64;
                    cr.new_path();
                    cr.move_to(x, y + (h - text_height) / 2.);
                    cr.set_source_rgb(fg.0, fg.1, fg.2);
                    pangocairo::show_layout(cr, &layout);
                }

                Inhibit(false)
            }));

            drawing_area.connect_button_press_event(clone!(@weak kb => @default-panic, move |_drawing_area, evt| {
                let pos = evt.get_position();
                for (i, k) in kb.keys().iter().enumerate() {
                    let x = (k.physical.x * SCALE) + MARGIN;
                    let y = -(k.physical.y * SCALE) + MARGIN;
                    let w = (k.physical.w * SCALE) - MARGIN * 2.;
                    let h = (k.physical.h * SCALE) - MARGIN * 2.;

                    if (x..=x+w).contains(&pos.0) && (y..=y+h).contains(&pos.1) {
                        if kb.selected() == Some(i) {
                            kb.set_selected(None);
                        } else {
                            kb.set_selected(Some(i));
                        }
                    }
                }
                Inhibit(false)
            }));

            self.inner().stack.add_titled(&drawing_area, page.name(), page.name());

            // TODO: Replace with something type-safe
            unsafe { drawing_area.set_data("keyboard_configurator_page", page) };
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

        picker.set_selected(None);

        if let Some(i) = i {
            let k = &keys[i];
            println!("{:#?}", k);
            if let Some(layer) = self.layer() {
                if let Some((_scancode, scancode_name)) = keys[i].scancodes.borrow().get(layer) {
                    picker.set_selected(Some(scancode_name.to_string()));
                }
            }
        }

        picker.set_sensitive(self.layer() != None);

        self.inner().selected.set(i);

        self.queue_draw();
    }
}
