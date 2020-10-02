#![windows_subsystem = "windows"]

use gio::prelude::*;
use gtk::prelude::*;
use serde_json::Value;
use std::{
    cell::RefCell,
    char,
    collections::HashMap,
    env,
    fs,
    io,
    path::{
        Path,
    },
    process,
    rc::Rc,
    str::{
        self,
        FromStr
    },
};
use system76_keyboard_configurator::{
    daemon::{
        Daemon,
        DaemonClient,
        DaemonServer,
    },
};

#[derive(Clone, Debug)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Clone, Debug)]
struct Key {
    // Logical position (row, column)
    logical: (u8, u8),
    // Logical name (something like K01, where 0 is the row and 1 is the column)
    logical_name: String,
    // Physical position and size
    physical: Rect,
    // Physical key name (what is printed on the keycap)
    physical_name: String,
    // Electrical mapping (output, input)
    electrical: (u8, u8),
    // Electrical name (output, input)
    electrical_name: String,
    // Currently loaded scancodes and their names
    scancodes: Vec<(u16, String)>,
    // Background color
    background_color: String,
    // Foreground color
    foreground_color: String,
    // GTK buttons by page
    //TODO: clean up this crap
    gtk: HashMap<String, gtk::Button>,
}

impl Key {
    fn css(&self) -> String {
        format!(
r#"
button {{
    background-image: none;
    background-color: {};
    border-image: none;
    box-shadow: none;
    color: {};
    margin: 0;
    padding: 0;
    text-shadow: none;
    -gtk-icon-effect: none;
    -gtk-icon-shadow: none;
}}

.selected {{
    border-color: #fbb86c;
    border-width: 4px;
}}
"#,
            self.background_color,
            self.foreground_color,
        )
    }

    fn select(&self, picker: &Picker, layer: usize) {
        for (_page, button) in self.gtk.iter() {
            button.get_style_context().add_class("selected");
        }
        if let Some((_scancode, scancode_name)) = self.scancodes.get(layer) {
            if let Some(picker_key) = picker.keys.get(scancode_name) {
                if let Some(button) = &*picker_key.gtk.borrow() {
                    button.get_style_context().add_class("selected");
                }
            }
        }
    }

    fn deselect(&self, picker: &Picker, layer: usize) {
        for (_page, button) in self.gtk.iter() {
            button.get_style_context().remove_class("selected");
        }
        if let Some((_scancode, scancode_name)) = self.scancodes.get(layer) {
            if let Some(picker_key) = picker.keys.get(scancode_name) {
                if let Some(ref button) = &*picker_key.gtk.borrow() {
                    button.get_style_context().remove_class("selected");
                }
            }
        }
    }

    fn refresh(&self, picker: &Picker) {
        for (page, button) in self.gtk.iter() {
            button.set_label(match page.as_str() {
                "Layer 1" => {
                    let scancode_name = &self.scancodes[0].1;
                    if let Some(picker_key) = picker.keys.get(scancode_name) {
                        &picker_key.text
                    } else {
                        scancode_name
                    }
                },
                "Layer 2" => {
                    let scancode_name = &self.scancodes[1].1;
                    if let Some(picker_key) = picker.keys.get(scancode_name) {
                        &picker_key.text
                    } else {
                        scancode_name
                    }
                },
                "Keycaps" => &self.physical_name,
                "Logical" => &self.logical_name,
                "Electrical" => &self.electrical_name,
                _ => "",
            });
        }
    }
}

pub struct PickerKey {
    /// Symbolic name of the key
    name: String,
    /// Text on key
    text: String,
    // GTK button
    //TODO: clean up this crap
    gtk: RefCell<Option<gtk::Button>>,
}

pub struct PickerGroup {
    /// Name of the group
    name: String,
    /// Number of keys to show in each row
    cols: i32,
    /// Width of each key in this group
    width: i32,
    /// Name of keys in this group
    keys: Vec<Rc<PickerKey>>,
}

pub struct Picker {
    groups: Vec<PickerGroup>,
    keys: HashMap<String, Rc<PickerKey>>,
}

impl Picker {
    fn new() -> Self {
        const DEFAULT_COLS: i32 = 3;

        let mut groups = Vec::new();
        let mut keys = HashMap::new();

        let mut is_group = true;
        let picker_csv = include_str!("../layouts/picker.csv");
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(picker_csv.as_bytes());
        for record_res in reader.records() {
            let record = record_res.expect("Failed to parse picker.csv");

            let name = record.get(0).unwrap_or("");
            if name.is_empty() {
                is_group = true;
            } else if is_group {
                is_group = false;

                let cols_str = record.get(1).unwrap_or("");
                let cols = match cols_str.parse::<i32>() {
                    Ok(ok) => ok,
                    Err(err) => {
                        eprintln!("Failed to parse column count '{}': {}", cols_str, err);
                        DEFAULT_COLS
                    }
                };

                let width_str = record.get(2).unwrap_or("");
                let width = match width_str.parse::<i32>() {
                    Ok(ok) => ok,
                    Err(err) => {
                        eprintln!("Failed to parse width '{}': {}", width_str, err);
                        1
                    }
                };

                let group = PickerGroup {
                    name: name.to_string(),
                    cols,
                    width,
                    keys: Vec::new(),
                };

                groups.push(group);
            } else {
                let top = record.get(1).unwrap_or("");
                let bottom = record.get(2).unwrap_or("");

                let key = Rc::new(PickerKey {
                    name: name.to_string(),
                    text: if bottom.is_empty() {
                        top.to_string()
                    } else {
                        format!("{}\n{}", top, bottom)
                    },
                    gtk: RefCell::new(None),
                });

                groups.last_mut().map(|group| {
                    group.keys.push(key.clone());
                });

                keys.insert(name.to_string(), key);
            }
        }

        Self { groups, keys }
    }
}

pub struct Keyboard {
    daemon_opt: RefCell<Option<Box<dyn Daemon>>>,
    daemon_board: usize,
    keymap: Vec<(String, u16)>,
    keys: RefCell<Vec<Key>>,
    page: RefCell<u32>,
    picker: Picker,
    selected: RefCell<Option<usize>>,
}

impl Keyboard {
    fn new<P: AsRef<Path>>(dir: P, daemon_opt: Option<Box<dyn Daemon>>, daemon_board: usize) -> Rc<Self> {
        let dir = dir.as_ref();

        let keymap_csv = fs::read_to_string(dir.join("keymap.csv"))
            .expect("Failed to load keymap.csv");
        let layout_csv = fs::read_to_string(dir.join("layout.csv"))
            .expect("Failed to load layout.csv");
        let physical_json = fs::read_to_string(dir.join("physical.json"))
            .expect("Failed to load physical.json");
        Self::new_data(&keymap_csv, &layout_csv, &physical_json, daemon_opt, daemon_board)
    }

    fn new_board(board: &str, daemon_opt: Option<Box<dyn Daemon>>, daemon_board: usize) -> Option<Rc<Self>> {
        macro_rules! keyboard {
            ($board:expr) => (if board == $board {
                let keymap_csv = include_str!(concat!("../layouts/", $board, "/keymap.csv"));
                let layout_csv = include_str!(concat!("../layouts/", $board, "/layout.csv"));
                let physical_json = include_str!(concat!("../layouts/", $board, "/physical.json"));
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

    fn new_data(keymap_csv: &str, layout_csv: &str, physical_json: &str, mut daemon_opt: Option<Box<dyn Daemon>>, daemon_board: usize) -> Rc<Self> {
        let mut keymap = Vec::new();
        let mut scancode_names = HashMap::new();
        scancode_names.insert(0, "NONE");
        for line in keymap_csv.lines() {
            let mut parts = line.split(',');
            let scancode_name = parts.next().expect("Failed to read scancode name");
            let scancode_str = parts.next().expect("Failed to read scancode");
            let scancode_trim = scancode_str.trim_start_matches("0x");
            let scancode = u16::from_str_radix(scancode_trim, 16).expect("Failed to parse scancode");
            keymap.push((scancode_name.to_string(), scancode));
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
                                        let scancode = if let Some(ref mut daemon) = daemon_opt {
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
            daemon_opt: RefCell::new(daemon_opt),
            daemon_board,
            keymap,
            keys: RefCell::new(keys),
            page: RefCell::new(0),
            picker: Picker::new(),
            selected: RefCell::new(None),
        })
    }

    fn layer(&self) -> usize {
        //TODO: make this more robust
        match *self.page.borrow() {
            0 => 0, // Layer 1
            1 => 1, // Layer 2
            _ => 0, // Any other page selects Layer 1
        }
    }

    fn picker(self: Rc<Self>) -> gtk::Box {
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

        let style_provider = gtk::CssProvider::new();
        style_provider.load_from_data(&PICKER_CSS.as_bytes()).expect("Failed to parse css");

        let picker_vbox = gtk::Box::new(gtk::Orientation::Vertical, 32);
        let mut picker_hbox_opt: Option<gtk::Box> = None;
        let mut picker_col = 0;
        let picker_cols = DEFAULT_COLS;

        for group in self.picker.groups.iter() {
            let vbox = gtk::Box::new(gtk::Orientation::Vertical, 4);
            let mut hbox_opt: Option<gtk::Box> = None;
            let mut col = 0;

            let label = gtk::Label::new(Some(&group.name));
            label.set_halign(gtk::Align::Start);
            label.set_margin_bottom(8);
            vbox.add(&label);

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
                let button = gtk::Button::new();
                button.set_hexpand(false);
                button.set_size_request(48 * group.width, 48);
                button.set_label(&key.text);

                let style_context = button.get_style_context();
                style_context.add_provider(&style_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);

                // Check that scancode is available for the keyboard
                button.set_sensitive(false);
                for (scancode_name, _scancode) in self.keymap.iter() {
                    if key.name.as_str() == scancode_name {
                        button.set_sensitive(true);
                        break;
                    }
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
                        for (scancode_name, scancode) in kb.keymap.iter() {
                            if name.as_str() == scancode_name {
                                k.deselect(&kb.picker, layer);
                                k.scancodes[layer] = (*scancode, scancode_name.clone());
                                k.refresh(&kb.picker);
                                k.select(&kb.picker, layer);
                                found = true;
                                break;
                            }
                        }
                        if ! found {
                            return;
                        }
                        println!("  set {}, {}, {} to {:04X}", layer, k.electrical.0, k.electrical.1, k.scancodes[layer].0);
                        if let Some(ref mut daemon) = *kb.daemon_opt.borrow_mut() {
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

    fn gtk(self: Rc<Self>) -> gtk::Box {
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);

        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        vbox.add(&hbox);

        let notebook = gtk::Notebook::new();
        {
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
        }
        vbox.add(&notebook);

        {
            let label = gtk::Label::new(Some("Brightness:"));
            label.set_halign(gtk::Align::Start);
            hbox.add(&label);
        }

        let max_brightness = {
            let path = "/sys/class/leds/system76_acpi::kbd_backlight/max_brightness";
            match fs::read_to_string(&path) {
                Ok(string) => {
                    let trimmed = string.trim();
                    match trimmed.parse::<u32>() {
                        Ok(u32) => u32 as f64,
                        Err(err) => {
                            eprintln!("Failed to parse keyboard max brightness '{}': {}", trimmed, err);
                            100.0
                        }
                    }
                },
                Err(err) => {
                    eprintln!("Failed to read keyboard max brightness: {}", err);
                    100.0
                }
            }
        };

        let brightness_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, max_brightness, 1.0);
        brightness_scale.set_halign(gtk::Align::Fill);
        brightness_scale.set_size_request(200, 0);
        brightness_scale.connect_value_changed(|this| {
            let value = this.get_value();
            let string = format!("{}", value);
            println!("{}", value);

            let path = "/sys/class/leds/system76_acpi::kbd_backlight/brightness";
            match fs::write(path, &string) {
                Ok(()) => (),
                Err(err) => {
                    eprintln!("Failed to write keyboard brightness: {}", err);
                }
            }
        });
        hbox.add(&brightness_scale);

        {
            let label = gtk::Label::new(Some("Color:"));
            label.set_halign(gtk::Align::Start);
            hbox.add(&label);
        }

        let color_rgba = {
            let path = "/sys/class/leds/system76_acpi::kbd_backlight/color";
            match fs::read_to_string(&path) {
                Ok(string) => {
                    let trimmed = string.trim();
                    let formatted = format!("#{}", trimmed);
                    match gdk::RGBA::from_str(&formatted) {
                        Ok(rgba) => rgba,
                        Err(err) => {
                            eprintln!("Failed to parse keyboard color '{}': {:?}", formatted, err);
                            gdk::RGBA::black()
                        }
                    }
                },
                Err(err) => {
                    eprintln!("Failed to read keyboard color: {}", err);
                    gdk::RGBA::black()
                }
            }
        };

        let color_button = gtk::ColorButton::with_rgba(&color_rgba);
        color_button.set_halign(gtk::Align::Fill);
        color_button.connect_color_set(|this| {
            let rgba = this.get_rgba();
            let r = (rgba.red * 255.0) as u8;
            let g = (rgba.green * 255.0) as u8;
            let b = (rgba.blue * 255.0) as u8;
            let string = format!("{:02X}{:02X}{:02X}", r, g, b);
            println!("{:?} => {}", rgba, string);

            let path = "/sys/class/leds/system76_acpi::kbd_backlight/color";
            match fs::write(path, &string) {
                Ok(()) => (),
                Err(err) => {
                    eprintln!("Failed to write keyboard color: {}", err);
                }
            }
        });
        hbox.add(&color_button);

        for page in &[
            "Layer 1",
            "Layer 2",
            "Keycaps",
            "Logical",
            "Electrical"
        ] {
            let page_label = gtk::Label::new(Some(page));
            let fixed = gtk::Fixed::new();
            notebook.append_page(&fixed, Some(&page_label));

            let keys_len = self.keys.borrow().len();
            for i in 0..keys_len {
                let button = {
                    let keys = self.keys.borrow();
                    let k = &keys[i];

                    let scale = 64.0;
                    let margin = 2;
                    let x = (k.physical.x * scale) as i32 + margin;
                    let y = -(k.physical.y * scale) as i32 + margin;
                    let w = (k.physical.w * scale) as i32 - margin * 2;
                    let h = (k.physical.h * scale) as i32 - margin * 2;

                    let button = gtk::Button::new();
                    button.set_focus_on_click(false);
                    button.set_size_request(w, h);
                    {
                        let css = k.css();
                        let style_provider = gtk::CssProvider::new();
                        style_provider.load_from_data(css.as_bytes()).expect("Failed to parse css");

                        let style_context = button.get_style_context();
                        style_context.add_provider(&style_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);
                    }
                    fixed.put(&button, x, y);
                    button
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
                k.gtk.insert(page.to_string(), button);
                k.refresh(&self.picker);
            }
        }

        vbox
    }
}

//TODO: allow multiple keyboards
fn main_keyboard(app: &gtk::Application, keyboard: Rc<Keyboard>) {
    let window = gtk::ApplicationWindow::new(app);

    window.set_title("Keyboard Layout");
    window.set_border_width(10);
    window.set_position(gtk::WindowPosition::Center);
    window.set_default_size(1024, 768);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 32);
    vbox.add(&keyboard.clone().gtk());
    vbox.add(&keyboard.clone().picker());

    let scrolled_window = gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
    scrolled_window.add(&vbox);
    window.add(&scrolled_window);

    window.set_focus::<gtk::Widget>(None);
    window.show_all();

    window.connect_destroy(|_| {
        eprintln!("Window close");
        gtk::main_quit();
    });
}

fn main_app(app: &gtk::Application, mut daemon: Box<dyn Daemon>) {
    let boards = daemon.boards().expect("Failed to load boards");
    let i = 0;
    if let Some(board) = boards.get(i) {
        if let Some(keyboard) = Keyboard::new_board(board, Some(daemon), i) {
            main_keyboard(app, keyboard);
            return;
        } else {
            eprintln!("Failed to locate layout for '{}'", board);
        }
    }

    eprintln!("Failed to locate any keyboards, showing demo");
    let keyboard = Keyboard::new_board("system76/launch_alpha_2", None, 0)
        .expect("Failed to load demo layout");
    main_keyboard(app, keyboard);
}

fn daemon_server() -> Result<DaemonServer<io::Stdin, io::Stdout>, String> {
    DaemonServer::new(io::stdin(), io::stdout())
}

#[cfg(target_os = "linux")]
fn with_daemon<F: Fn(Box<dyn Daemon>)>(f: F) {
    use std::{
        process::{
            Command,
            Stdio,
        },
    };

    if unsafe { libc::geteuid() == 0 } {
        eprintln!("Already running as root");
        let server = daemon_server().expect("Failed to create server");
        f(Box::new(server));
        return;
    }

    // Use pkexec to spawn daemon as superuser
    eprintln!("Not running as root, spawning daemon with pkexec");
    let mut command = Command::new("pkexec");

    // Use canonicalized command name
    let command_name = match env::var("APPIMAGE") {
        Ok(ok) => ok,
        Err(_) => env::args().nth(0).expect("Failed to get command name"),
    };
    let command_path = fs::canonicalize(command_name).expect("Failed to canonicalize command");
    command.arg(command_path);
    command.arg("--daemon");

    // Pipe stdin and stdout
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());

    let mut child = command.spawn().expect("Failed to spawn daemon");

    let stdin = child.stdin.take().expect("Failed to get stdin of daemon");
    let stdout = child.stdout.take().expect("Failed to get stdout of daemon");

    f(Box::new(DaemonClient::new(stdout, stdin)));

    let status = child.wait().expect("Failed to wait for daemon");
    if ! status.success() {
        panic!("Failed to run daemon with exit status {:?}", status);
    }
}

#[cfg(not(target_os = "linux"))]
fn with_daemon<F: Fn(Box<dyn Daemon>)>(f: F) {
    let server = daemon_server().expect("Failed to create server");
    f(Box::new(server));
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    for arg in args.iter().skip(1) {
        if arg.as_str() == "--daemon" {
            let server = daemon_server().expect("Failed to create server");
            server.run().expect("Failed to run server");
            return;
        }
    }

    let application =
        gtk::Application::new(Some("com.system76.keyboard-layout"), Default::default())
            .expect("Failed to create gtk::Application");

    application.connect_activate(move |app| {
        if let Some(window) = app.get_active_window() {
            //TODO
            eprintln!("Focusing current window");
            window.present();
        } else {
            with_daemon(|daemon| {
                main_app(app, daemon);
                //TODO: is this the best way to keep the daemon running?
                gtk::main();
            });
        }
    });

    process::exit(application.run(&args));
}
