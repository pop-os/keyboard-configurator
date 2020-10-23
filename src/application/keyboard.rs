use cascade::cascade;
use glib::object::WeakRef;
use gtk::prelude::*;
use std::{
    cell::{
        Cell,
        RefCell,
    },
    char,
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
use super::page::Page;
use super::picker::Picker;
use super::physical_layout::{PhysicalLayout, PhysicalLayoutEntry, PhysicalKeyEnum};
use super::rect::Rect;

pub struct Keyboard {
    pub(crate) daemon_opt: Option<Rc<dyn Daemon>>,
    pub(crate) daemon_board: usize,
    pub(crate) keymap: HashMap<String, u16>,
    pub(crate) keys: RefCell<Vec<Key>>,
    page: Cell<Page>,
    picker: RefCell<WeakRef<Picker>>,
    pub(crate) selected: RefCell<Option<usize>>,
}

impl Keyboard {
    pub fn new<P: AsRef<Path>>(dir: P, daemon_opt: Option<Rc<dyn Daemon>>, daemon_board: usize) -> Rc<Self> {
        let dir = dir.as_ref();

        let keymap_csv = fs::read_to_string(dir.join("keymap.csv"))
            .expect("Failed to load keymap.csv");
        let layout_csv = fs::read_to_string(dir.join("layout.csv"))
            .expect("Failed to load layout.csv");
        let physical_json = fs::read_to_string(dir.join("physical.json"))
            .expect("Failed to load physical.json");
        Self::new_data(&keymap_csv, &layout_csv, &physical_json, daemon_opt, daemon_board)
    }

    pub fn new_board(board: &str, daemon_opt: Option<Rc<dyn Daemon>>, daemon_board: usize) -> Option<Rc<Self>> {
        macro_rules! keyboard {
            ($board:expr) => (if board == $board {
                let keymap_csv = include_str!(concat!("../../layouts/", $board, "/keymap.csv"));
                let layout_csv = include_str!(concat!("../../layouts/", $board, "/layout.csv"));
                let physical_json = include_str!(concat!("../../layouts/", $board, "/physical.json"));
                return Some(Keyboard::new_data(keymap_csv, layout_csv, physical_json, daemon_opt, daemon_board));
            });
        }

        keyboard!("system76/addw1");
        keyboard!("system76/addw2");
        keyboard!("system76/bonw14");
        keyboard!("system76/darp5");
        keyboard!("system76/darp6");
        keyboard!("system76/gaze15");
        keyboard!("system76/launch_alpha_1");
        keyboard!("system76/launch_alpha_2");
        keyboard!("system76/launch_beta_1");
        keyboard!("system76/lemp9");
        keyboard!("system76/oryp5");
        keyboard!("system76/oryp6");
        None
    }

    fn new_data(keymap_csv: &str, layout_csv: &str, physical_json: &str, daemon_opt: Option<Rc<dyn Daemon>>, daemon_board: usize) -> Rc<Self> {
        let mut keymap = HashMap::new();
        let mut scancode_names = HashMap::new();
        scancode_names.insert(0, "NONE");
        for line in keymap_csv.lines() {
            let mut parts = line.split(',');
            let scancode_name = parts.next().expect("Failed to read scancode name");
            let scancode_str = parts.next().expect("Failed to read scancode");
            let scancode_trim = scancode_str.trim_start_matches("0x");
            let scancode = u16::from_str_radix(scancode_trim, 16).expect("Failed to parse scancode");
            keymap.insert(scancode_name.to_string(), scancode);
            scancode_names.insert(scancode, scancode_name);
        }

        let mut layout = HashMap::new();
        for line in layout_csv.lines() {
            let mut parts = line.split(',');
            let logical_name = parts.next().expect("Failed to read logical name");
            let output_str = parts.next().expect("Failed to read electrical output");
            let output = output_str.parse().expect("Failed to parse electrical output");
            let input_str = parts.next().expect("Failed to read electrical input");
            let input = input_str.parse().expect("Failed to parse electrical input");
            layout.insert(logical_name, (output, input));
        }

        let physical_layout: PhysicalLayout = serde_json::from_str(&physical_json).unwrap();
        //println!("{:#?}", v);

        let mut keys = Vec::new();

        let mut row_i = 0;
        let mut col_i = 0;
        let mut x = 0.0;
        let mut y = 0.0;
        let mut w = 1.0;
        let mut h = 1.0;
        let mut background_color = "#cccccc".to_string();
        let mut foreground_color = "#000000".to_string();

        for entry in physical_layout.0 {
            if let PhysicalLayoutEntry::Row(row) = entry {
                for i in row.0 {
                    match i {
                        PhysicalKeyEnum::Meta(meta) => {
                            println!("Key metadata {:?}", meta);
                            x += meta.x;
                            y -= meta.y;
                            w = meta.w.unwrap_or(w);
                            h = meta.h.unwrap_or(h);
                            background_color = meta.c.unwrap_or(background_color);
                            if let Some(t) = meta.t {
                                //TODO: support using different color per line?
                                //Is this even possible in GTK?
                                if let Some(t_l) = t.lines().next() {
                                    foreground_color = t_l.to_string();
                                }
                            }
                        }
                        PhysicalKeyEnum::Name(name) => {
                            println!("Key {}, {} = {:?}", x, y, name);

                            let logical = (row_i as u8, col_i as u8);
                            println!("  Logical: {:?}", logical);

                            let row_char = char::from_digit(logical.0 as u32, 36)
                                .expect("Failed to convert row to char");
                            let col_char = char::from_digit(logical.1 as u32, 36)
                                .expect("Failed to convert col to char");
                            let logical_name = format!("K{}{}", row_char, col_char).to_uppercase();
                            println!("  Logical Name: {}", logical_name);

                            let electrical = layout.get(logical_name.as_str())
                                //.expect("Failed to find electrical mapping");
                                .unwrap_or(&(0, 0));
                            println!("  Electrical: {:?}", electrical);

                            let mut scancodes = Vec::new();
                            for layer in 0..2 {
                                println!("  Layer {}", layer);
                                let scancode = if let Some(ref daemon) = daemon_opt {
                                    match daemon.keymap_get(daemon_board, layer, electrical.0, electrical.1) {
                                        Ok(value) => value,
                                        Err(err) => {
                                            eprintln!("Failed to read scancode: {:?}", err);
                                            0
                                        }
                                    }
                                } else {
                                    0
                                };
                                println!("    Scancode: {:04X}", scancode);

                                let scancode_name = match scancode_names.get(&scancode) {
                                    Some(some) => some.to_string(),
                                    None => String::new(),
                                };
                                println!("    Scancode Name: {}", scancode_name);

                                scancodes.push((scancode, scancode_name));
                            }

                            keys.push(Key {
                                logical,
                                logical_name,
                                physical: Rect::new(x, y, w, h),
                                physical_name: name,
                                electrical: electrical.clone(),
                                electrical_name: format!("{}, {}", electrical.0, electrical.1),
                                scancodes,
                                background_color: background_color.clone(),
                                foreground_color: foreground_color.clone(),
                                gtk: HashMap::new(),
                            });

                            x += w;

                            w = 1.0;
                            h = 1.0;

                            col_i += 1;
                        }
                    }
                }

                x = 0.0;
                y -= 1.0;

                col_i = 0;
                row_i += 1;
            }
        }

        Rc::new(Self {
            daemon_opt,
            daemon_board,
            keymap,
            keys: RefCell::new(keys),
            page: Cell::new(Page::Layer1),
            picker: RefCell::new(WeakRef::new()),
            selected: RefCell::new(None),
        })
    }

    pub fn layer(&self) -> usize {
        //TODO: make this more robust
        match self.page.get() {
            Page::Layer1 => 0,
            Page::Layer2 => 1,
            _ => 0, // Any other page selects Layer 1
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
                if let Some(i) = *kb.selected.borrow() {
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

        let max_brightness = if let Some(ref daemon) = self.daemon_opt {
            match daemon.max_brightness(self.daemon_board) {
                Ok(value) => value as f64,
                Err(err) => {
                    eprintln!("{}", err);
                    100.0
                }
            }
        } else {
            100.0
        };

        let brightness_scale = cascade! {
            gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, max_brightness, 1.0);
            ..set_halign(gtk::Align::Fill);
            ..set_size_request(200, 0);
        };
        let kb = self.clone();
        brightness_scale.connect_value_changed(move |this| {
            let value = this.get_value() as i32;
            if let Some(ref daemon) = kb.daemon_opt {
                if let Err(err) = daemon.set_brightness(kb.daemon_board, value) {
                    eprintln!("{}", err);
                }
            }
            println!("{}", value);

        });

        let color_label = cascade! {
            gtk::Label::new(Some("Color:"));
            ..set_halign(gtk::Align::Start);
        };

        let color_keyboard = if let Some(ref daemon) = self.daemon_opt {
            ColorKeyboard::new_daemon(daemon.clone(), self.daemon_board)
        } else {

            ColorKeyboard::new_dummy()
        };
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

                        if let Some(selected) = kb.selected.borrow_mut().take() {
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

                        *kb.selected.borrow_mut() = Some(i);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_board() {
        gtk::init().unwrap();
        for i in &[
            "system76/addw1",
            "system76/addw2",
            "system76/bonw14",
            "system76/darp5",
            "system76/darp6",
            "system76/gaze15",
            "system76/launch_alpha_1",
            "system76/launch_alpha_2",
            "system76/lemp9",
            "system76/oryp5",
            "system76/oryp6",
        ] {
            Keyboard::new_board(i, None, 0).unwrap();
        }
    }
}
