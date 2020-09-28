use ectool::{AccessDriver, Ec};
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
    path::Path,
    rc::Rc,
    str::{
        self,
        FromStr
    },
    time::Duration,
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

    fn select(&self) {
        for (_page, button) in self.gtk.iter() {
            button.get_style_context().add_class("selected");
        }
    }

    fn deselect(&self) {
        for (_page, button) in self.gtk.iter() {
            button.get_style_context().remove_class("selected");
        }
    }

    fn refresh(&self) {
        for (page, button) in self.gtk.iter() {
            button.set_label(match page.as_str() {
                "Keycaps" => &self.physical_name,
                "Layer 0" => &self.scancodes[0].1,
                "Layer 1" => &self.scancodes[1].1,
                "Logical" => &self.logical_name,
                "Electrical" => &self.electrical_name,
                _ => "",
            });
        }
    }
}

pub struct Keyboard {
    ec_opt: RefCell<Option<Ec<AccessDriver>>>,
    keymap: Vec<(String, u16)>,
    keys: RefCell<Vec<Key>>,
    selected: RefCell<Option<usize>>,
}

impl Keyboard {
    fn new<P: AsRef<Path>>(dir: P, ec_opt: Option<Ec<AccessDriver>>) -> Rc<Self> {
        let dir = dir.as_ref();

        let keymap_csv = fs::read_to_string(dir.join("keymap.csv"))
            .expect("failed to load keymap.csv");
        let layout_csv = fs::read_to_string(dir.join("layout.csv"))
            .expect("failed to load layout.csv");
        let physical_json = fs::read_to_string(dir.join("physical.json"))
            .expect("failed to load physical.json");
        Self::new_data(&keymap_csv, &layout_csv, &physical_json, ec_opt)
    }

    fn new_board(board: &str, ec_opt: Option<Ec<AccessDriver>>) -> Option<Rc<Self>> {
        macro_rules! keyboard {
            ($board:expr) => (if board == $board {
                let keymap_csv = include_str!(concat!("../layouts/", $board, "/keymap.csv"));
                let layout_csv = include_str!(concat!("../layouts/", $board, "/layout.csv"));
                let physical_json = include_str!(concat!("../layouts/", $board, "/physical.json"));
                return Some(Keyboard::new_data(keymap_csv, layout_csv, physical_json, ec_opt));
            });
        }

        keyboard!("system76/addw2");
        keyboard!("system76/bonw14");
        keyboard!("system76/darp5");
        keyboard!("system76/darp6");
        keyboard!("system76/gaze15");
        keyboard!("system76/launch_1");
        keyboard!("system76/lemp9");
        keyboard!("system76/oryp5");
        keyboard!("system76/oryp6");
        None
    }

    fn new_data(keymap_csv: &str, layout_csv: &str, physical_json: &str, mut ec_opt: Option<Ec<AccessDriver>>) -> Rc<Self> {
        let mut keymap = Vec::new();
        let mut scancode_names = HashMap::new();
        scancode_names.insert(0, "NONE");
        for line in keymap_csv.lines() {
            let mut parts = line.split(',');
            let scancode_name = parts.next().expect("failed to read scancode name");
            let scancode_str = parts.next().expect("failed to read scancode");
            let scancode_trim = scancode_str.trim_start_matches("0x");
            let scancode = u16::from_str_radix(scancode_trim, 16).expect("failed to parse scancode");
            keymap.push((scancode_name.to_string(), scancode));
            scancode_names.insert(scancode, scancode_name);
        }

        let mut layout = HashMap::new();
        for line in layout_csv.lines() {
            let mut parts = line.split(',');
            let logical_name = parts.next().expect("failed to read logical name");
            let output_str = parts.next().expect("failed to read electrical output");
            let output = output_str.parse().expect("failed to parse electrical output");
            let input_str = parts.next().expect("failed to read electrical input");
            let input = input_str.parse().expect("failed to parse electrical input");
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
                                        .expect("failed to convert row to char");
                                    let col_char = char::from_digit(logical.1 as u32, 36)
                                        .expect("failed to convert col to char");
                                    let logical_name = format!("K{}{}", row_char, col_char).to_uppercase();
                                    println!("  Logical Name: {}", logical_name);

                                    let electrical = layout.get(logical_name.as_str())
                                        //.expect("failed to find electrical mapping");
                                        .unwrap_or(&(0, 0));
                                    println!("  Electrical: {:?}", electrical);

                                    let mut scancodes = Vec::new();
                                    for layer in 0..2 {
                                        println!("  Layer {}", layer);
                                        let scancode = if let Some(ref mut ec) = ec_opt {
                                            let value_res = unsafe {
                                                ec.keymap_get(layer, electrical.0, electrical.1)
                                            };
                                            match value_res {
                                                Ok(value) => value,
                                                Err(err) => {
                                                    eprintln!("failed to read scancode: {:?}", err);
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
            ec_opt: RefCell::new(ec_opt),
            keymap,
            keys: RefCell::new(keys),
            selected: RefCell::new(None),
        })
    }

    fn picker(self: Rc<Self>) -> gtk::Box {
        const DEFAULT_COLS: i32 = 3;
        const PICKER_CSS: &'static str =
r#"
button {
    margin: 0;
    padding: 4px;
}
"#;

        let style_provider = gtk::CssProvider::new();
        style_provider.load_from_data(&PICKER_CSS.as_bytes()).expect("failed to parse css");

        let picker_vbox = gtk::Box::new(gtk::Orientation::Vertical, 32);
        let mut picker_hbox_opt: Option<gtk::Box> = None;
        let mut picker_col = 0;
        let picker_cols = DEFAULT_COLS;

        let mut vbox_opt: Option<gtk::Box> = None;
        let mut hbox_opt: Option<gtk::Box> = None;
        let mut col = 0;
        let mut cols = DEFAULT_COLS;
        let picker_csv = include_str!("../layouts/picker.csv");
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(picker_csv.as_bytes());
        for record_res in reader.records() {
            let record = record_res.expect("failed to parse picker.csv");

            let name = record.get(0).unwrap_or("");
            if name.is_empty() {
                vbox_opt = None;
                hbox_opt = None;
                col = 0;
            } else if let Some(vbox) = &vbox_opt {
                let top = record.get(1).unwrap_or("");
                let bottom = record.get(2).unwrap_or("");

                let button = gtk::Button::new();
                button.set_hexpand(false);
                button.set_size_request(48, 48);
                button.set_label(&if bottom.is_empty() {
                    format!("{}", top)
                } else {
                    format!("{}\n{}", top, bottom)
                });
                {
                    let style_context = button.get_style_context();
                    style_context.add_provider(&style_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);
                }

                let hbox = match hbox_opt.take() {
                    Some(some) => some,
                    None => {
                        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                        vbox.add(&hbox);
                        hbox
                    }
                };

                hbox.add(&button);

                col += 1;
                if col >= cols {
                    col = 0;
                } else {
                    hbox_opt = Some(hbox);
                }
            } else {
                let cols_str = record.get(1).unwrap_or("");
                match cols_str.parse::<i32>() {
                    Ok(ok) => {
                        cols = ok;
                    },
                    Err(err) => {
                        eprintln!("failed to parse column count '{}': {}", cols_str, err);
                        cols = DEFAULT_COLS;
                    }
                }

                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 4);

                let label = gtk::Label::new(Some(name));
                label.set_halign(gtk::Align::Start);
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

                vbox_opt = Some(vbox);
                hbox_opt = None;
                col = 0;
            }
        }

        picker_vbox
    }

    fn gtk(self: Rc<Self>) -> gtk::Box {
    const NONE_CSS: &'static str =
r#"
button {
    background-image: none;
    border-image: none;
    box-shadow: none;
    margin: 0;
    padding: 0;
    text-shadow: none;
    -gtk-icon-effect: none;
    -gtk-icon-shadow: none;
}
"#;

        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);

        let notebook = gtk::Notebook::new();
        hbox.add(&notebook);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
        //hbox.add(&vbox);

        let (selected_label, selected_style_provider) = {
            let label = gtk::Label::new(Some("Selected:"));
            label.set_halign(gtk::Align::Start);
            vbox.add(&label);

            let button = gtk::Button::new();
            button.set_label("None");
            button.set_focus_on_click(false);
            button.set_halign(gtk::Align::Start);
            button.set_sensitive(false);
            button.set_size_request(60, 60);
            vbox.add(&button);

            let style_provider = gtk::CssProvider::new();
            style_provider.load_from_data(&NONE_CSS.as_bytes()).expect("failed to parse css");

            let style_context = button.get_style_context();
            style_context.add_provider(&style_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);

            (Rc::new(button), Rc::new(style_provider))
        };

        let mut layer_boxes = Vec::new();
        for layer in 0..2 {
            {
                let label = gtk::Label::new(Some(&format!("Layer {}: ", layer)));
                label.set_halign(gtk::Align::Start);
                vbox.add(&label);
            }

            let layer_box = gtk::ComboBoxText::new();
            layer_box.set_halign(gtk::Align::Fill);
            layer_box.set_sensitive(false);
            vbox.add(&layer_box);

            layer_box.append(Some("NONE"), "NONE");
            for (scancode_name, _scancode) in self.keymap.iter() {
                layer_box.append(Some(&scancode_name), &scancode_name);
            }
            {
                let kb = self.clone();
                layer_box.connect_changed(move |lb| {
                    if let Some(active_id) = lb.get_active_id() {
                        println!("layer {}: {}", layer, active_id);
                        if let Some(i) = *kb.selected.borrow() {
                            let mut keys = kb.keys.borrow_mut();
                            let k = &mut keys[i];
                            let mut found = false;
                            for (scancode_name, scancode) in kb.keymap.iter() {
                                if active_id.as_str() == scancode_name {
                                    k.scancodes[layer] = (*scancode, scancode_name.clone());
                                    k.refresh();
                                    found = true;
                                    break;
                                }
                            }
                            if ! found {
                                return;
                            }
                            println!("  set {}, {}, {} to {:04X}", layer, k.electrical.0, k.electrical.1, k.scancodes[layer].0);
                            if let Some(ref mut ec) = *kb.ec_opt.borrow_mut() {
                                unsafe {
                                    if let Err(err) = ec.keymap_set(layer as u8, k.electrical.0, k.electrical.1, k.scancodes[layer].0) {
                                        eprintln!("failed to set keymap: {:?}", err);
                                    }
                                }
                            }
                        }
                    }
                });
            }
            layer_boxes.push(layer_box);
        }
        let layer_boxes = Rc::new(layer_boxes);

        {
            let label = gtk::Label::new(Some("Brightness:"));
            label.set_halign(gtk::Align::Start);
            vbox.add(&label);
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
        vbox.add(&brightness_scale);

        {
            let label = gtk::Label::new(Some("Color:"));
            label.set_halign(gtk::Align::Start);
            vbox.add(&label);
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
        vbox.add(&color_button);

        for page in &[
            "Keycaps",
            "Layer 0",
            "Layer 1",
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
                        style_provider.load_from_data(css.as_bytes()).expect("failed to parse css");

                        let style_context = button.get_style_context();
                        style_context.add_provider(&style_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);
                    }
                    fixed.put(&button, x, y);
                    button
                };

                {
                    let kb = self.clone();
                    let selected_label = selected_label.clone();
                    let selected_style_provider = selected_style_provider.clone();
                    let layer_boxes = layer_boxes.clone();
                    button.connect_clicked(move |_| {
                        let keys = kb.keys.borrow();

                        if let Some(selected) = kb.selected.borrow_mut().take() {
                            keys[selected].deselect();
                            // Implements deselect by clicking again
                            if selected == i {
                                selected_label.set_label("None");
                                selected_label.set_sensitive(false);
                                selected_style_provider.load_from_data(&NONE_CSS.as_bytes()).expect("failed to parse css");

                                //TODO: reliable array indexing
                                for layer in 0..2 {
                                    layer_boxes[layer].set_sensitive(false);
                                    layer_boxes[layer].set_active_id(None);
                                }

                                return;
                            }
                        }

                        {
                            let k = &keys[i];
                            println!("{:#?}", k);
                            k.select();

                            selected_label.set_label(&k.physical_name);
                            selected_label.set_sensitive(true);
                            let css = k.css();
                            selected_style_provider.load_from_data(css.as_bytes()).expect("failed to parse css");

                            //TODO: reliable array indexing
                            for layer in 0..2 {
                                let (_scancode, scancode_name) = &k.scancodes[layer];
                                layer_boxes[layer].set_sensitive(true);
                                if layer_boxes[layer].set_active_id(Some(scancode_name)) {
                                    println!("set active item {}", scancode_name);
                                } else {
                                    println!("failed to set active item {}", scancode_name);
                                }
                            }
                        }

                        *kb.selected.borrow_mut() = Some(i);
                    });
                }

                let mut keys = self.keys.borrow_mut();
                let k = &mut keys[i];
                k.gtk.insert(page.to_string(), button);
                k.refresh();
            }
        }

        hbox
    }
}

fn main() {
    /*
    let dir = env::args().nth(1).expect("no directory provided");


    let keyboard = Keyboard::new(dir, ec_opt);
    */

    let mut keyboard_opt = None;
    match AccessDriver::new() {
        Ok(access) => match unsafe { Ec::new(access) } {
            Ok(mut ec) => {
                let mut data = [0; 256 - 2];
                match unsafe { ec.board(&mut data) } {
                    Ok(len) => match str::from_utf8(&data[..len]) {
                        Ok(board) => {
                            eprintln!("detected EC board '{}'", board);
                            keyboard_opt = Keyboard::new_board(board, Some(ec));
                        },
                        Err(err) => {
                            eprintln!("failed to parse EC board: {:?}", err);
                        }
                    },
                    Err(err) => {
                        eprintln!("Failed to run EC board command: {:?}", err);
                    }
                }
            },
            Err(err) => {
                eprintln!("failed to probe EC: {:?}", err);
            }
        },
        Err(err) => {
            eprintln!("failed to access EC: {:?}", err);
        }
    };

    let keyboard = match keyboard_opt {
        Some(some) => some,
        None => {
            eprintln!("failed to locate layout, showing demo");
            Keyboard::new_board("system76/darp6", None).expect("failed to load demo layout")
        }
    };

    //let ansi_104 = Keyboard::new("layouts/ansi-104", None);

    let application =
        gtk::Application::new(Some("com.system76.keyboard-layout"), Default::default())
            .expect("Initialization failed...");

    application.connect_activate(move |_app| {
        // Dialog is used instead of ApplicationWindow to make it float
        let window = gtk::Dialog::new();

        window.set_title("Keyboard Layout");
        window.set_border_width(10);
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(0, 0);
        window.set_modal(true);
        window.set_resizable(false);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 32);
        vbox.add(&keyboard.clone().gtk());
        vbox.add(&keyboard.clone().picker());
        //&ansi_104.clone().gtk(&vbox);
        window.get_content_area().add(&vbox);

        window.set_focus::<gtk::Widget>(None);
        window.show_all();
        window.run();
    });

    application.run(&[]);}
