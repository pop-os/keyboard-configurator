use std::cell::RefCell;
use std::char;
use std::collections::HashMap;

mod physical_layout;
pub(super) use physical_layout::PhysicalLayout;

use super::key::Key;
use super::rect::Rect;
use physical_layout::{PhysicalKeyEnum, PhysicalLayoutEntry};

pub(super) struct Layout<'a> {
    pub keymap: HashMap<String, u16>,
    pub scancode_names: HashMap<u16, &'a str>,
    physical: PhysicalLayout,
    layout: HashMap<&'a str, (u8, u8)>,
}

macro_rules! keyboards {
    ($( $board:expr ),* $(,)?) => {
        fn layout_data(board: &str) -> Option<(&'static str, &'static str, &'static str)> {
            match board {
                $(
                $board => {
                    let keymap_csv =
                        include_str!(concat!("../../../layouts/", $board, "/keymap.csv"));
                    let layout_csv =
                        include_str!(concat!("../../../layouts/", $board, "/layout.csv"));
                    let physical_json =
                        include_str!(concat!("../../../layouts/", $board, "/physical.json"));
                    Some((keymap_csv, layout_csv, physical_json))
                }
                )*
                _ => None
            }
        }

        pub(super) fn layouts() -> &'static [&'static str] {
            &[$( $board ),*]
        }
    };
}

keyboards![
    "system76/addw1",
    "system76/addw2",
    "system76/bonw14",
    "system76/darp5",
    "system76/darp6",
    "system76/gaze15",
    "system76/launch_alpha_1",
    "system76/launch_alpha_2",
    "system76/launch_beta_1",
    "system76/lemp9",
    "system76/oryp5",
    "system76/oryp6",
];

impl<'a> Layout<'a> {
    pub fn from_data(keymap_csv: &'a str, layout_csv: &'a str, physical_json: &'a str) -> Self {
        let (keymap, scancode_names) = parse_keymap_csv(keymap_csv);
        let layout = parse_layout_csv(layout_csv);
        let physical = parse_physical_json(&physical_json);
        Self {
            keymap,
            scancode_names,
            physical,
            layout,
        }
    }

    pub fn from_board(board: &'a str) -> Option<Self> {
        layout_data(board).map(|(keymap_csv, layout_csv, physical_json)| {
            Self::from_data(keymap_csv, layout_csv, physical_json)
        })
    }

    pub fn keys(&self) -> Vec<Key> {
        let mut keys = Vec::new();

        let mut row_i = 0;
        let mut col_i = 0;
        let mut x = 0.0;
        let mut y = 0.0;
        let mut w = 1.0;
        let mut h = 1.0;
        let mut background_color = "#cccccc".to_string();
        let mut foreground_color = "#000000".to_string();

        for entry in &self.physical.0 {
            if let PhysicalLayoutEntry::Row(row) = entry {
                for i in &row.0 {
                    match i {
                        PhysicalKeyEnum::Meta(meta) => {
                            println!("Key metadata {:?}", meta);
                            x += meta.x;
                            y -= meta.y;
                            w = meta.w.unwrap_or(w);
                            h = meta.h.unwrap_or(h);
                            background_color = meta.c.clone().unwrap_or(background_color);
                            if let Some(t) = &meta.t {
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

                            let electrical = self
                                .layout
                                .get(logical_name.as_str())
                                //.expect("Failed to find electrical mapping");
                                .unwrap_or(&(0, 0));
                            println!("  Electrical: {:?}", electrical);

                            keys.push(Key {
                                logical,
                                logical_name,
                                physical: Rect::new(x, y, w, h),
                                physical_name: name.clone(),
                                electrical: electrical.clone(),
                                electrical_name: format!("{}, {}", electrical.0, electrical.1),
                                scancodes: RefCell::new(Vec::new()),
                                background_color: background_color.clone(),
                                foreground_color: foreground_color.clone(),
                                gtk: RefCell::new(HashMap::new()),
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

        keys
    }
}

fn parse_keymap_csv(keymap_csv: &str) -> (HashMap<String, u16>, HashMap<u16, &str>) {
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
    (keymap, scancode_names)
}

fn parse_layout_csv(layout_csv: &str) -> HashMap<&str, (u8, u8)> {
    let mut layout = HashMap::new();
    for line in layout_csv.lines() {
        let mut parts = line.split(',');
        let logical_name = parts.next().expect("Failed to read logical name");
        let output_str = parts.next().expect("Failed to read electrical output");
        let output = output_str
            .parse()
            .expect("Failed to parse electrical output");
        let input_str = parts.next().expect("Failed to read electrical input");
        let input = input_str.parse().expect("Failed to parse electrical input");
        layout.insert(logical_name, (output, input));
    }
    layout
}

fn parse_physical_json(physical_json: &str) -> PhysicalLayout {
    serde_json::from_str(physical_json).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_from_board() {
        for i in layouts() {
            Layout::from_board(i).unwrap();
        }
    }
}
