use std::cell::RefCell;
use std::char;
use std::collections::HashMap;

mod physical_layout;
pub(super) use physical_layout::PhysicalLayout;

use super::{Key, Rect};
use crate::{KeyMap, Rgb};
use physical_layout::{PhysicalKeyEnum, PhysicalLayoutEntry};

pub(super) struct Layout {
    pub default: KeyMap,
    pub keymap: HashMap<String, u16>,
    pub scancode_names: HashMap<u16, String>,
    physical: PhysicalLayout,
    layout: HashMap<String, (u8, u8)>,
    leds: HashMap<String, Vec<u8>>,
}

macro_rules! keyboards {
    ($( $board:expr ),* $(,)?) => {
        fn layout_data(board: &str) -> Option<(&'static str, &'static str, &'static str, &'static str, &'static str)> {
            match board {
                $(
                $board => {
                    let default_json =
                        include_str!(concat!("../../../layouts/", $board, "/default.json"));
                    let keymap_json =
                        include_str!(concat!("../../../layouts/", $board, "/keymap.json"));
                    let layout_json =
                        include_str!(concat!("../../../layouts/", $board, "/layout.json"));
                    let leds_json =
                        include_str!(concat!("../../../layouts/", $board, "/leds.json"));
                    let physical_json =
                        include_str!(concat!("../../../layouts/", $board, "/physical.json"));
                    Some((default_json, keymap_json, layout_json, leds_json, physical_json))
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
    "system76/launch_1",
    "system76/lemp9",
    "system76/oryp5",
    "system76/oryp6",
    "system76/oryp7",
];

impl Layout {
    pub fn from_data(
        default_json: &str,
        keymap_json: &str,
        layout_json: &str,
        leds_json: &str,
        physical_json: &str,
    ) -> Self {
        let default = KeyMap::from_str(default_json).unwrap();
        let (keymap, scancode_names) = parse_keymap_json(keymap_json);
        let layout = parse_layout_json(layout_json);
        let leds = parse_leds_json(leds_json);
        let physical = parse_physical_json(&physical_json);
        Self {
            default,
            keymap,
            scancode_names,
            physical,
            layout,
            leds,
        }
    }

    pub fn from_board(board: &str) -> Option<Self> {
        layout_data(board).map(
            |(default_json, keymap_json, layout_json, leds_json, physical_json)| {
                Self::from_data(
                    default_json,
                    keymap_json,
                    layout_json,
                    leds_json,
                    physical_json,
                )
            },
        )
    }

    pub fn keys(&self) -> Vec<Key> {
        let mut keys = Vec::new();

        let mut row_i = 0;
        let mut col_i = 0;
        let mut x = 0.0;
        let mut y = 0.0;
        let mut w = 1.0;
        let mut h = 1.0;
        let mut background_color = Rgb::new(0xcc, 0xcc, 0xcc);
        let mut foreground_color = Rgb::new(0x00, 0x00, 0x00);

        for entry in &self.physical.0 {
            if let PhysicalLayoutEntry::Row(row) = entry {
                for i in &row.0 {
                    match i {
                        PhysicalKeyEnum::Meta(meta) => {
                            debug!("Key metadata {:?}", meta);
                            x += meta.x;
                            y -= meta.y;
                            w = meta.w.unwrap_or(w);
                            h = meta.h.unwrap_or(h);
                            background_color = meta
                                .c
                                .as_ref()
                                .map(|c| {
                                    let err = format!("Failed to parse color {}", c);
                                    Rgb::parse(&c[1..]).expect(&err)
                                })
                                .unwrap_or(background_color);
                            if let Some(t) = &meta.t {
                                //TODO: support using different color per line?
                                //Is this even possible in GTK?
                                if let Some(t_l) = t.lines().next() {
                                    let err = format!("Failed to parse color {}", t_l);
                                    foreground_color = Rgb::parse(&t_l[1..]).expect(&err);
                                }
                            }
                        }
                        PhysicalKeyEnum::Name(name) => {
                            debug!("Key {}, {} = {:?}", x, y, name);

                            let logical = (row_i as u8, col_i as u8);
                            debug!("  Logical: {:?}", logical);

                            let row_char = char::from_digit(logical.0 as u32, 36)
                                .expect("Failed to convert row to char");
                            let col_char = char::from_digit(logical.1 as u32, 36)
                                .expect("Failed to convert col to char");
                            let logical_name = format!("K{}{}", row_char, col_char).to_uppercase();
                            debug!("  Logical Name: {}", logical_name);

                            let electrical = self
                                .layout
                                .get(logical_name.as_str())
                                //.expect("Failed to find electrical mapping");
                                .unwrap_or(&(0, 0));
                            debug!("  Electrical: {:?}", electrical);

                            let leds = self
                                .leds
                                .get(logical_name.as_str())
                                .map_or(Vec::new(), |x| x.clone());
                            let mut led_name = String::new();
                            for led in leds.iter() {
                                if !led_name.is_empty() {
                                    led_name.push_str(", ");
                                }
                                led_name.push_str(&led.to_string());
                            }
                            debug!("  LEDs: {:?}", leds);

                            keys.push(Key {
                                logical,
                                logical_name,
                                physical: Rect::new(x, y, w, h),
                                physical_name: name.clone(),
                                electrical: *electrical,
                                electrical_name: format!("{}, {}", electrical.0, electrical.1),
                                leds,
                                led_name,
                                scancodes: RefCell::new(Vec::new()),
                                background_color,
                                foreground_color,
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

fn parse_keymap_json(keymap_json: &str) -> (HashMap<String, u16>, HashMap<u16, String>) {
    let mut keymap = HashMap::new();
    let mut scancode_names = HashMap::new();
    let l: Vec<(String, u16)> = serde_json::from_str(keymap_json).unwrap();
    for (scancode_name, scancode) in l {
        keymap.insert(scancode_name.clone(), scancode);
        scancode_names.insert(scancode, scancode_name);
    }
    (keymap, scancode_names)
}

fn parse_layout_json(layout_json: &str) -> HashMap<String, (u8, u8)> {
    serde_json::from_str(layout_json).unwrap()
}

fn parse_leds_json(leds_json: &str) -> HashMap<String, Vec<u8>> {
    serde_json::from_str(leds_json).unwrap()
}

fn parse_physical_json(physical_json: &str) -> PhysicalLayout {
    serde_json::from_str(physical_json).unwrap()
}

#[cfg(test)]
mod tests {
    use super::super::picker::SCANCODE_LABELS;
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn layout_from_board() {
        for i in layouts() {
            Layout::from_board(i).unwrap();
        }
    }

    #[test]
    fn default_keys_exist() {
        let mut missing = HashSet::new();
        for i in layouts() {
            let layout = Layout::from_board(i).unwrap();
            for j in layout.default.map.values().flatten() {
                if layout.keymap.keys().find(|x| x == &j).is_none() {
                    missing.insert(j.to_owned());
                }
            }
        }
        assert_eq!(missing, HashSet::new());
    }

    #[test]
    fn picker_has_keys() {
        let mut missing = HashSet::new();
        for i in layouts() {
            let layout = Layout::from_board(i).unwrap();
            for j in layout.default.map.values().flatten() {
                if SCANCODE_LABELS.keys().find(|x| x == &j).is_none() {
                    missing.insert(j.to_owned());
                }
            }
        }
        assert_eq!(missing, HashSet::new());
    }
}
