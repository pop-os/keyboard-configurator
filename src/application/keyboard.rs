use cascade::cascade;
use glib::object::WeakRef;
use gtk::prelude::*;
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

pub struct Keyboard {
    daemon: Rc<dyn Daemon>,
    daemon_board: usize,
    keymap: HashMap<String, u16>,
    keys: RefCell<Vec<Key>>,
    page: Cell<Page>,
    picker: RefCell<WeakRef<Picker>>,
    selected: Cell<Option<usize>>,
}

impl Keyboard {
    pub fn new<P: AsRef<Path>>(dir: P, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Rc<Self> {
        let dir = dir.as_ref();

        let keymap_csv = fs::read_to_string(dir.join("keymap.csv"))
            .expect("Failed to load keymap.csv");
        let layout_csv = fs::read_to_string(dir.join("layout.csv"))
            .expect("Failed to load layout.csv");
        let physical_json = fs::read_to_string(dir.join("physical.json"))
            .expect("Failed to load physical.json");
        Self::new_data(&keymap_csv, &layout_csv, &physical_json, daemon, daemon_board)
    }

    fn new_layout(layout: Layout, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Rc<Self> {
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

        Rc::new(Self {
            daemon,
            daemon_board,
            keymap: layout.keymap,
            keys: RefCell::new(keys),
            page: Cell::new(Page::Layer1),
            picker: RefCell::new(WeakRef::new()),
            selected: Cell::new(None),
        })
    }

    pub fn new_board(board: &str, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Option<Rc<Self>> {
        Layout::from_board(board).map(|layout|
            Self::new_layout(layout, daemon, daemon_board)
        )
    }

    fn new_data(keymap_csv: &str, layout_csv: &str, physical_json: &str, daemon: Rc<dyn Daemon>, daemon_board: usize) -> Rc<Self> {
        let layout = Layout::from_data(keymap_csv, layout_csv, physical_json);
        Self::new_layout(layout, daemon, daemon_board)
    }

    pub fn layer(&self) -> usize {
        //TODO: make this more robust
        match self.page.get() {
            Page::Layer1 => 0,
            Page::Layer2 => 1,
            _ => 0, // Any other page selects Layer 1
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected.get()
    }

    pub fn has_scancode(&self, scancode_name: &str) -> bool {
        self.keymap.contains_key(scancode_name)
    }

    pub fn keymap_set(&self, picker: &Picker, key_index: usize, layer: usize, scancode_name: &str) {
        // XXX avoid reference to Picker
        let mut keys = self.keys.borrow_mut();
        let k = &mut keys[key_index];
        let mut found = false;
        if let Some(scancode) = self.keymap.get(scancode_name) {
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
        if let Err(err) = self.daemon.keymap_set(
            self.daemon_board,
            layer as u8,
            k.electrical.0,
            k.electrical.1,
            k.scancodes[layer].0,
        ) {
            eprintln!("Failed to set keymap: {:?}", err);
        }

    }

    pub fn gtk(self: Rc<Self>) -> gtk::Box {
        let stack = cascade! {
            gtk::Stack::new();
            ..set_transition_duration(0);
        };
        let kb = self.clone();
        stack.connect_property_visible_child_notify(move |stack| {
            let picker = match kb.picker.borrow().upgrade() {
                Some(picker) => picker,
                None => { return; },
            };

            let page: Option<Page> = match stack.get_visible_child() {
                Some(child) => unsafe { child.get_data("keyboard_confurator_page").cloned() },
                None => None,
            };

            println!("{:?}", page);
            let last_layer = kb.layer();
            kb.page.set(page.unwrap_or(Page::Layer1));
            let layer = kb.layer();
            if layer != last_layer {
                if let Some(i) = kb.selected.get() {
                    let keys = kb.keys.borrow();
                    let k = &keys[i];
                    k.deselect(&picker, last_layer);
                    k.select(&picker, layer);
                }
            }
        });

        let brightness_label = cascade! {
            gtk::Label::new(Some("Brightness:"));
            ..set_halign(gtk::Align::Start);
        };

        let max_brightness = match self.daemon.max_brightness(self.daemon_board) {
            Ok(value) => value as f64,
            Err(err) => {
                eprintln!("{}", err);
                100.0
            }
        };

        let brightness_scale = cascade! {
            gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, max_brightness, 1.0);
            ..set_halign(gtk::Align::Fill);
            ..set_size_request(200, 0);
        };
        let kb = self.clone();
        brightness_scale.connect_value_changed(move |this| {
            let value = this.get_value() as i32;
            if let Err(err) = kb.daemon.set_brightness(kb.daemon_board, value) {
                eprintln!("{}", err);
            }
            println!("{}", value);

        });

        let color_label = cascade! {
            gtk::Label::new(Some("Color:"));
            ..set_halign(gtk::Align::Start);
        };

        let color_keyboard = ColorKeyboard::new_daemon(self.daemon.clone(), self.daemon_board);
        let color_button = KeyboardColorButton::new(color_keyboard);
        color_button.set_valign(gtk::Align::Center);

        for page in Page::iter_all() {
            let fixed = gtk::Fixed::new();
            stack.add_titled(&fixed, page.name(), page.name());

            // TODO: Replace with something type-safe
            unsafe { fixed.set_data("keyboard_confurator_page", page) };

            let keys_len = self.keys.borrow().len();
            for i in 0..keys_len {
                let (button, label) = {
                    let keys = self.keys.borrow();
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

                {
                    let kb = self.clone();
                    button.connect_clicked(move |_| {
                        let picker = match kb.picker.borrow().upgrade() {
                            Some(picker) => picker,
                            None => { return; },
                        };

                        let keys = kb.keys.borrow();

                        if let Some(selected) = kb.selected.replace(None) {
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

                        kb.selected.set(Some(i));
                    });
                }

                let mut keys = self.keys.borrow_mut();
                let k = &mut keys[i];
                k.gtk.insert(page, (button, label));
                if let Some(picker) = self.picker.borrow().upgrade() {
                    k.refresh(&picker);
                }
            }
        }

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
            ..add(&color_button);
        };

        let vbox = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 8);
            ..add(&toolbar);
            ..add(&hbox);
            ..add(&stack);
        };

        vbox
    }

    pub(super) fn set_picker(&self, picker: Option<&Picker>) {
        // This function is called by Picker::set_keyboard()
        *self.picker.borrow_mut() = match picker {
            Some(picker) => picker.downgrade(),
            None => WeakRef::new(),
        };
    }
}
