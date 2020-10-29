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
    fs,
    path::{
        Path,
    },
    rc::Rc,
    str,
};

use crate::daemon::Daemon;
use crate::keyboard::Keyboard as ColorKeyboard;
use crate::keyboard_color_button::KeyboardColorButton;
use super::key::Key;
use super::layout::Layout;
use super::page::Page;
use super::picker::Picker;

pub struct KeyboardInner {
    daemon: OnceCell<Rc<dyn Daemon>>,
    daemon_board: OnceCell<usize>,
    keymap: OnceCell<HashMap<String, u16>>,
    keys: RefCell<Vec<Key>>,
    page: Cell<Page>,
    picker: RefCell<WeakRef<Picker>>,
    selected: Cell<Option<usize>>,
    color_button_bin: gtk::Frame,
    brightness_scale: gtk::Scale,
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

        let toolbar = cascade!{
            gtk::Box::new(gtk::Orientation::Horizontal, 8);
            ..set_center_widget(Some(&stack_switcher));
        };

        let hbox = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 8);
            ..add(&brightness_label);
            ..add(&brightness_scale);
            ..add(&color_label);
            ..add(&color_button_bin);
        };

        Self {
            daemon: OnceCell::new(),
            daemon_board: OnceCell::new(),
            keymap: OnceCell::new(),
            keys: RefCell::new(Vec::new()),
            page: Cell::new(Page::Layer1),
            picker: RefCell::new(WeakRef::new()),
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
    pub fn new<P: AsRef<Path>>(dir: P, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Self {
        let dir = dir.as_ref();

        let keymap_csv = fs::read_to_string(dir.join("keymap.csv"))
            .expect("Failed to load keymap.csv");
        let layout_csv = fs::read_to_string(dir.join("layout.csv"))
            .expect("Failed to load layout.csv");
        let physical_json = fs::read_to_string(dir.join("physical.json"))
            .expect("Failed to load physical.json");
        Self::new_data(&keymap_csv, &layout_csv, &physical_json, daemon, daemon_board)
    }

    fn new_layout(layout: Layout, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Self {
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

                key.scancodes.push((scancode, scancode_name));
            }
        }
        *keyboard.inner().keys.borrow_mut() = keys;

        let _ = keyboard.inner().daemon.set(daemon);
        let _ = keyboard.inner().daemon_board.set(daemon_board);
        let _ = keyboard.inner().keymap.set(layout.keymap);

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
            Self::new_layout(layout, daemon, daemon_board)
        )
    }

    fn new_data(keymap_csv: &str, layout_csv: &str, physical_json: &str, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Self {
        let layout = Layout::from_data(keymap_csv, layout_csv, physical_json);
        Self::new_layout(layout, daemon, daemon_board)
    }

    fn inner(&self) -> &KeyboardInner {
        KeyboardInner::from_instance(self)
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

    pub fn keymap_set(&self, picker: &Picker, key_index: usize, layer: usize, scancode_name: &str) {
        // XXX avoid reference to Picker
        let mut keys = self.inner().keys.borrow_mut();
        let k = &mut keys[key_index];
        let mut found = false;
        if let Some(scancode) = self.keymap().get(scancode_name) {
            k.deselect(&picker, layer);
            k.scancodes[layer] = (*scancode, scancode_name.to_string());
            k.refresh(&picker);
            k.select(&picker, layer);
            found = true;
        }
        if !found {
            return;
        }
        println!(
            "  set {}, {}, {} to {:04X}",
            layer, k.electrical.0, k.electrical.1, k.scancodes[layer].0
        );
        if let Err(err) = self.daemon().keymap_set(
            self.daemon_board(),
            layer as u8,
            k.electrical.0,
            k.electrical.1,
            k.scancodes[layer].0,
        ) {
            eprintln!("Failed to set keymap: {:?}", err);
        }

    }

    fn connect_signals(&self) {
        let kb = self;

        self.inner().stack.connect_property_visible_child_notify(clone!(@weak kb => @default-panic, move |stack| {
            let picker = match kb.inner().picker.borrow().upgrade() {
                Some(picker) => picker,
                None => { return; },
            };

            let page: Option<Page> = match stack.get_visible_child() {
                Some(child) => unsafe { child.get_data("keyboard_confurator_page").cloned() },
                None => None,
            };

            println!("{:?}", page);
            let last_layer = kb.layer();
            kb.inner().page.set(page.unwrap_or(Page::Layer1));
            let layer = kb.layer();
            if layer != last_layer {
                if let Some(i) = kb.inner().selected.get() {
                    let keys = kb.inner().keys.borrow();
                    let k = &keys[i];
                    k.deselect(&picker, last_layer);
                    k.select(&picker, layer);
                }
            }
        }));

        self.inner().brightness_scale.connect_value_changed(clone!(@weak kb => @default-panic, move |this| {
            let value = this.get_value() as i32;
            if let Err(err) = kb.daemon().set_brightness(kb.daemon_board(), value) {
                eprintln!("{}", err);
            }
            println!("{}", value);

        }));
    }

    fn add_pages(&self) {
        let kb = self;

        for page in Page::iter_all() {
            let fixed = gtk::Fixed::new();
            self.inner().stack.add_titled(&fixed, page.name(), page.name());

            // TODO: Replace with something type-safe
            unsafe { fixed.set_data("keyboard_confurator_page", page) };

            let keys_len = self.inner().keys.borrow().len();
            for i in 0..keys_len {
                let (button, label) = {
                    let keys = self.inner().keys.borrow();
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
                    let picker = match kb.inner().picker.borrow().upgrade() {
                        Some(picker) => picker,
                        None => { return; },
                    };

                    let keys = kb.inner().keys.borrow();

                    if let Some(selected) = kb.inner().selected.replace(None) {
                        keys[selected].deselect(&picker, kb.layer());
                        if i == selected {
                            // Allow deselect
                            return;
                        }
                    }

                    {
                        let k = &keys[i];
                        println!("{:#?}", k);
                        k.select(&picker, kb.layer());
                    }

                    kb.inner().selected.set(Some(i));
                }));

                let mut keys = self.inner().keys.borrow_mut();
                let k = &mut keys[i];
                k.gtk.insert(page, (button, label));
            }
        }
    }

    pub(super) fn set_picker(&self, picker: Option<&Picker>) {
        // This function is called by Picker::set_keyboard()
        *self.inner().picker.borrow_mut() = match picker {
            Some(picker) => picker.downgrade(),
            None => WeakRef::new(),
        };

        for k in self.inner().keys.borrow().iter() {
            if let Some(picker) = self.inner().picker.borrow().upgrade() {
                k.refresh(&picker);
            }
        }
    }
}
