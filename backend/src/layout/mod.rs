use std::{collections::HashMap, fs, path::Path};

mod meta;
mod physical_layout;
pub use self::meta::Meta;
pub(crate) use physical_layout::{PhysicalLayout, PhysicalLayoutKey};

use crate::{KeyMap, Rgb};

pub struct Layout {
    /// Metadata for keyboard
    pub meta: Meta,
    /// Default keymap for this keyboard
    pub default: KeyMap,
    keymap: HashMap<String, u16>,
    scancode_names: HashMap<u16, String>,
    pub(crate) physical: PhysicalLayout,
    pub(crate) layout: HashMap<String, (u8, u8)>,
    pub(crate) leds: HashMap<String, Vec<u8>>,
}

macro_rules! keyboards {
    ($( $board:expr ),* $(,)?) => {
        fn layout_data(board: &str) -> Option<(&'static str, &'static str, &'static str, &'static str, &'static str, &'static str)> {
            match board {
                $(
                $board => {
                    let meta_json =
                        include_str!(concat!("../../../layouts/", $board, "/meta.json"));
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
                    Some((meta_json, default_json, keymap_json, layout_json, leds_json, physical_json))
                }
                )*
                _ => None
            }
        }

        /// Names of board layouts that can be opened with `Layout::from_board`
        pub fn layouts() -> &'static [&'static str] {
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
    "system76/darp7",
    "system76/galp3-c",
    "system76/galp4",
    "system76/galp5",
    "system76/gaze15",
    "system76/launch_alpha_1",
    "system76/launch_alpha_2",
    "system76/launch_1",
    "system76/lemp9",
    "system76/lemp10",
    "system76/oryp5",
    "system76/oryp6",
    "system76/oryp7",
];

impl Layout {
    pub fn from_data(
        meta_json: &str,
        default_json: &str,
        keymap_json: &str,
        layout_json: &str,
        leds_json: &str,
        physical_json: &str,
    ) -> Self {
        let meta = serde_json::from_str(meta_json).unwrap();
        let default = KeyMap::from_str(default_json).unwrap();
        let (keymap, scancode_names) = parse_keymap_json(keymap_json);
        let layout = serde_json::from_str(layout_json).unwrap();
        let leds = serde_json::from_str(leds_json).unwrap();
        let physical = PhysicalLayout::from_str(physical_json);
        Self {
            meta,
            default,
            keymap,
            scancode_names,
            physical,
            layout,
            leds,
        }
    }

    #[allow(dead_code)]
    pub fn from_dir<P: AsRef<Path>>(dir: P) -> Self {
        let dir = dir.as_ref();

        let meta_json =
            fs::read_to_string(dir.join("meta.json")).expect("Failed to load meta.json");
        let default_json =
            fs::read_to_string(dir.join("default.json")).expect("Failed to load default.json");
        let keymap_json =
            fs::read_to_string(dir.join("keymap.json")).expect("Failed to load keymap.json");
        let layout_json =
            fs::read_to_string(dir.join("layout.json")).expect("Failed to load layout.json");
        let leds_json =
            fs::read_to_string(dir.join("leds.json")).expect("Failed to load leds.json");
        let physical_json =
            fs::read_to_string(dir.join("physical.json")).expect("Failed to load physical.json");

        Self::from_data(
            &meta_json,
            &default_json,
            &keymap_json,
            &layout_json,
            &leds_json,
            &physical_json,
        )
    }

    pub fn from_board(board: &str) -> Option<Self> {
        layout_data(board).map(
            |(meta_json, default_json, keymap_json, layout_json, leds_json, physical_json)| {
                Self::from_data(
                    meta_json,
                    default_json,
                    keymap_json,
                    layout_json,
                    leds_json,
                    physical_json,
                )
            },
        )
    }

    /// Get the scancode number corresponding to a name
    pub fn scancode_to_name(&self, scancode: u16) -> Option<&str> {
        self.scancode_names.get(&scancode).map(String::as_str)
    }

    /// Get the name corresponding to a scancode number
    pub fn scancode_from_name(&self, name: &str) -> Option<u16> {
        self.keymap.get(name).copied()
    }

    pub fn pressed_color(&self) -> Rgb {
        self.physical.meta.pressed_color
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, fs, io};

    #[test]
    fn layout_from_board() {
        for i in layouts() {
            Layout::from_board(i).unwrap();
        }
    }

    #[test]
    fn default_keys_exist() {
        for i in layouts() {
            let mut missing = HashSet::new();
            let layout = Layout::from_board(i).unwrap();
            for j in layout.default.map.values().flatten() {
                if layout.keymap.keys().find(|x| x == &j).is_none() {
                    missing.insert(j.to_owned());
                }
            }
            assert_eq!(missing, HashSet::new(), "Mssing in keymap for {}", i);
        }
    }

    #[test]
    fn qmk_has_ec_keycodes() {
        let layout_ec = Layout::from_board("system76/darp6").unwrap();
        let layout_qmk = Layout::from_board("system76/launch_1").unwrap();
        for k in layout_ec.keymap.keys() {
            if k == "KBD_COLOR"
                || k == "KBD_BKL"
                || k == "TOUCHPAD"
                || k == "DISPLAY_TOGGLE"
                || k == "DISPLAY_MODE"
                || k == "FAN_TOGGLE"
                || k == "CAMERA_TOGGLE"
                || k == "AIRPLANE_MODE"
            {
                continue;
            }
            assert_eq!(layout_qmk.keymap.keys().find(|x| x == &k), Some(k));
        }
    }

    #[test]
    fn has_all_layouts_in_dir() -> io::Result<()> {
        let layouts = layouts();
        for i in fs::read_dir("../layouts/system76")? {
            let i = i?;
            if i.file_type()?.is_dir() {
                let name = format!("system76/{}", i.file_name().into_string().unwrap());
                assert!(
                    layouts.contains(&name.as_str()),
                    "{} not listed in {}",
                    name,
                    file!()
                );
            }
        }
        Ok(())
    }
}
