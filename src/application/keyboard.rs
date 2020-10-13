use cascade::cascade;
use gtk::prelude::*;
use serde_json::Value;
use std::{
    cell::RefCell,
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
use super::rect::Rect;

pub struct Keyboard {
    daemon_opt: Option<Rc<dyn Daemon>>,
    daemon_board: usize,
    keymap: HashMap<String, u16>,
    keys: RefCell<Vec<Key>>,
    page: RefCell<u32>,
    picker: Picker,
    selected: RefCell<Option<usize>>,
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

        let v: Value = serde_json::from_str(&physical_json).unwrap();
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

        if let Value::Array(rows) = v {
            for row in rows {
                match row {
                    Value::Array(cols) => {
                        for col in cols {
                            match col {
                                Value::Object(o) => {
                                    println!("Key metadata {:?}", o);
                                    if let Some(x_v) = o.get("x") {
                                        if let Value::Number(x_n) = x_v {
                                            if let Some(x_f) = x_n.as_f64() {
                                                x += x_f;
                                            }
                                        }
                                    }
                                    if let Some(y_v) = o.get("y") {
                                        if let Value::Number(y_n) = y_v {
                                            if let Some(y_f) = y_n.as_f64() {
                                                y -= y_f;
                                            }
                                        }
                                    }
                                    if let Some(w_v) = o.get("w") {
                                        if let Value::Number(w_n) = w_v {
                                            if let Some(w_f) = w_n.as_f64() {
                                                w = w_f;
                                            }
                                        }
                                    }
                                    if let Some(h_v) = o.get("h") {
                                        if let Value::Number(h_n) = h_v {
                                            if let Some(h_f) = h_n.as_f64() {
                                                h = h_f;
                                            }
                                        }
                                    }
                                    if let Some(c_v) = o.get("c") {
                                        if let Value::String(c_s) = c_v {
                                            background_color = c_s.clone();
                                        }
                                    }
                                    if let Some(t_v) = o.get("t") {
                                        if let Value::String(t_s) = t_v {
                                            //TODO: support using different color per line?
                                            //Is this even possible in GTK?
                                            if let Some(t_l) = t_s.lines().next() {
                                                foreground_color = t_l.to_string();
                                            }
                                        }
                                    }
                                },
                                Value::String(s) => {
                                    println!("Key {}, {} = {:?}", x, y, s);

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
                                        physical_name: s,
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
                                _ => (),
                            }
                        }

                        x = 0.0;
                        y -= 1.0;

                        col_i = 0;
                        row_i += 1;
                    },
                    _ => (),
                }
            }
        }

        Rc::new(Self {
            daemon_opt,
            daemon_board,
            keymap,
            keys: RefCell::new(keys),
            page: RefCell::new(0),
            picker: Picker::new(),
            selected: RefCell::new(None),
        })
    }

    pub fn layer(&self) -> usize {
        //TODO: make this more robust
        match *self.page.borrow() {
            0 => 0, // Layer 1
            1 => 1, // Layer 2
            _ => 0, // Any other page selects Layer 1
        }
    }

    pub fn picker(self: Rc<Self>) -> gtk::Box {
        const DEFAULT_COLS: i32 = 3;
        const PICKER_CSS: &'static str =
r#"
button {
    margin: 0;
    padding: 0;
}

.selected {
    border-color: #fbb86c;
    border-width: 4px;
}
"#;

        let style_provider = cascade! {
            gtk::CssProvider::new();
            ..load_from_data(&PICKER_CSS.as_bytes()).expect("Failed to parse css");
        };

        let picker_vbox = gtk::Box::new(gtk::Orientation::Vertical, 32);
        let mut picker_hbox_opt: Option<gtk::Box> = None;
        let mut picker_col = 0;
        let picker_cols = DEFAULT_COLS;

        for group in self.picker.groups.iter() {
            let mut hbox_opt: Option<gtk::Box> = None;
            let mut col = 0;

            let label = cascade! {
                gtk::Label::new(Some(&group.name));
                ..set_halign(gtk::Align::Start);
                ..set_margin_bottom(8);
            };

            let vbox = cascade! {
                gtk::Box::new(gtk::Orientation::Vertical, 4);
                ..add(&label);
            };

            let picker_hbox = match picker_hbox_opt.take() {
                Some(some) => some,
                None => {
                    let picker_hbox = gtk::Box::new(gtk::Orientation::Horizontal, 64);
                    picker_vbox.add(&picker_hbox);
                    picker_hbox
                }
            };

            picker_hbox.add(&vbox);

            picker_col += 1;
            if picker_col >= picker_cols {
                picker_col = 0;
            } else {
                picker_hbox_opt = Some(picker_hbox);
            }

            for key in group.keys.iter() {
                let label = cascade! {
                    gtk::Label::new(Some(&key.text));
                    ..set_line_wrap(true);
                    ..set_max_width_chars(1);
                    ..set_justify(gtk::Justification::Center);
                };

                let button = cascade! {
                    gtk::Button::new();
                    ..set_size_request(48 * group.width, 48);
                    ..get_style_context().add_provider(&style_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);
                    ..add(&label);
                };

                // Check that scancode is available for the keyboard
                button.set_sensitive(false);
                if let Some(_scancode) = self.keymap.get(key.name.as_str()) {
                    button.set_sensitive(true);
                }

                let kb = self.clone();
                let name = key.name.to_string();
                button.connect_clicked(move |_| {
                    let layer = kb.layer();

                    println!("Clicked {} layer {}", name, layer);
                    if let Some(i) = *kb.selected.borrow() {
                        let mut keys = kb.keys.borrow_mut();
                        let k = &mut keys[i];
                        let mut found = false;
                        if let Some(scancode) = kb.keymap.get(name.as_str()) {
                            k.deselect(&kb.picker, layer);
                            k.scancodes[layer] = (*scancode, name.clone());
                            k.refresh(&kb.picker);
                            k.select(&kb.picker, layer);
                            found = true;
                        }
                        if ! found {
                            return;
                        }
                        println!("  set {}, {}, {} to {:04X}", layer, k.electrical.0, k.electrical.1, k.scancodes[layer].0);
                        if let Some(ref daemon) = kb.daemon_opt {
                            if let Err(err) = daemon.keymap_set(kb.daemon_board, layer as u8, k.electrical.0, k.electrical.1, k.scancodes[layer].0) {
                                eprintln!("Failed to set keymap: {:?}", err);
                            }
                        }
                    }
                });

                let hbox = match hbox_opt.take() {
                    Some(some) => some,
                    None => {
                        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                        vbox.add(&hbox);
                        hbox
                    }
                };

                hbox.add(&button);

                *key.gtk.borrow_mut() = Some(button);

                col += 1;
                if col >= group.cols {
                    col = 0;
                } else {
                    hbox_opt = Some(hbox);
                }
            }
        }

        picker_vbox
    }

    pub fn gtk(self: Rc<Self>) -> gtk::Box {
        let notebook = gtk::Notebook::new();
        let kb = self.clone();
        notebook.connect_switch_page(move |_, _, page| {
            println!("{}", page);
            let last_layer = kb.layer();
            *kb.page.borrow_mut() = page;
            let layer = kb.layer();
            if layer != last_layer {
                if let Some(i) = *kb.selected.borrow() {
                    let keys = kb.keys.borrow();
                    let k = &keys[i];
                    k.deselect(&kb.picker, last_layer);
                    k.select(&kb.picker, layer);
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
        let color_button = KeyboardColorButton::new(color_keyboard).widget().clone();
        color_button.set_valign(gtk::Align::Center);

        for page in Page::iter_all() {
            let page_label = gtk::Label::new(Some(page.name()));
            let fixed = gtk::Fixed::new();
            notebook.append_page(&fixed, Some(&page_label));

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
                        ..set_justify(gtk::Justification::Center);
                    };

                    let button = cascade! {
                        gtk::Button::new();
                        ..set_focus_on_click(false);
                        ..set_size_request(w, h);
                        ..get_style_context().add_provider(&style_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);
                        ..add(&label);
                    };

                    fixed.put(&button, x, y);

                    (button, label)
                };

                {
                    let kb = self.clone();
                    button.connect_clicked(move |_| {
                        let keys = kb.keys.borrow();

                        if let Some(selected) = kb.selected.borrow_mut().take() {
                            keys[selected].deselect(&kb.picker, kb.layer());
                            if i == selected {
                                // Allow deselect
                                return;
                            }
                        }

                        {
                            let k = &keys[i];
                            println!("{:#?}", k);
                            k.select(&kb.picker, kb.layer());
                        }

                        *kb.selected.borrow_mut() = Some(i);
                    });
                }

                let mut keys = self.keys.borrow_mut();
                let k = &mut keys[i];
                k.gtk.insert(page, (button, label));
                k.refresh(&self.picker);
            }
        }

        let hbox = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 8);
            ..add(&brightness_label);
            ..add(&brightness_scale);
            ..add(&color_label);
            ..add(&color_button);
        };

        let vbox = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 8);
            ..add(&hbox);
            ..add(&notebook);
        };

        vbox
    }
}