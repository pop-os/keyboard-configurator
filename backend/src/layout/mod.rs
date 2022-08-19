use std::{collections::HashMap, fs, path::Path};

mod meta;
mod physical_layout;
pub use self::meta::Meta;
pub(crate) use physical_layout::{PhysicalLayout, PhysicalLayoutKey};

use crate::{KeyMap, Keycode, Mods};

const QK_MOD_TAP: u16 = 0x6000;
const QK_MOD_TAP_MAX: u16 = 0x7FFF;
const QK_LAYER_TAP: u16 = 0x4000;
const QK_LAYER_TAP_MAX: u16 = 0x4FFF;
const QK_MODS: u16 = 0x0100;
const QK_MODS_MAX: u16 = 0x1FFF;

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
    ($( ($board:expr, $keyboard:expr) ),* $(,)?) => {
        fn layout_data(board: &str) -> Option<(&'static str, &'static str, &'static str, &'static str, &'static str, &'static str)> {
            match board {
                $(
                $board => {
                    let meta_json =
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/", $board, "/meta.json"));
                    let default_json =
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/", $board, "/default.json"));
                    let keymap_json =
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/keyboards/", $keyboard, "/keymap.json"));
                    let layout_json =
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/keyboards/", $keyboard, "/layout.json"));
                    let leds_json =
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/keyboards/", $keyboard, "/leds.json"));
                    let physical_json =
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/keyboards/", $keyboard, "/physical.json"));
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

// Calls the `keyboards!` macro
include!(concat!(env!("OUT_DIR"), "/keyboards.rs"));

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
    pub fn scancode_to_name(&self, scancode: u16) -> Option<Keycode> {
        // XXX only on QMK?
        if scancode >= QK_MOD_TAP && scancode <= QK_MOD_TAP_MAX {
            let mods = Mods::from_bits((scancode >> 8) & 0x1f)?;
            let kc = scancode & 0xff;
            let kc_name = self.scancode_names.get(&kc)?;
            Some(Keycode::MT(mods, kc_name.clone()))
        } else if scancode >= QK_LAYER_TAP && scancode <= QK_LAYER_TAP_MAX {
            let layer = ((scancode >> 8) & 0xf) as u8;
            let kc = scancode & 0xff;
            let kc_name = self.scancode_names.get(&kc)?;
            Some(Keycode::LT(layer, kc_name.clone()))
        } else if scancode >= QK_MODS && scancode <= QK_MODS_MAX {
            let mods = Mods::from_bits((scancode >> 8) & 0x1f)?;
            let kc = scancode & 0xff;
            let kc_name = self.scancode_names.get(&kc)?;
            Some(Keycode::Basic(mods, kc_name.clone()))
        } else {
            let kc_name = self.scancode_names.get(&scancode)?;
            if let Some(mods) = Mods::from_mod_str(kc_name) {
                Some(Keycode::Basic(mods, "NONE".to_string()))
            } else {
                Some(Keycode::Basic(Mods::empty(), kc_name.clone()))
            }
        }
    }

    /// Get the name corresponding to a scancode number
    pub fn scancode_from_name(&self, name: &Keycode) -> Option<u16> {
        match name {
            Keycode::MT(mods, keycode_name) => {
                let kc = *self.keymap.get(keycode_name)?;
                Some(QK_MOD_TAP | (mods.bits() << 8) | (kc & 0xff))
            }
            Keycode::LT(layer, keycode_name) => {
                let kc = *self.keymap.get(keycode_name)?;
                if *layer < 8 {
                    Some(QK_LAYER_TAP | (u16::from(*layer) << 8) | (kc & 0xFF))
                } else {
                    None
                }
            }
            Keycode::Basic(mods, keycode_name) => {
                if mods.is_empty() {
                    self.keymap.get(keycode_name).copied()
                } else if let Some(mod_name) = mods.as_mod_str() {
                    self.keymap.get(mod_name).copied()
                } else {
                    let kc = *self.keymap.get(keycode_name)?;
                    Some((mods.bits() << 8) | (kc & 0xff))
                }
            }
        }
    }

    pub fn f_keys(&self) -> impl Iterator<Item = &str> {
        self.default.map.iter().filter_map(|(k, v)| {
            if let Some(num) = v[0].strip_prefix('F') {
                if num.parse::<u8>().is_ok() {
                    return Some(k.as_str());
                }
            }
            None
        })
    }

    pub fn layout(&self) -> &HashMap<String, (u8, u8)> {
        &self.layout
    }
}

fn parse_keymap_json(keymap_json: &str) -> (HashMap<String, u16>, HashMap<u16, String>) {
    let mut scancode_names = HashMap::new();
    let keymap: HashMap<String, u16> = serde_json::from_str(keymap_json).unwrap();
    for (scancode_name, scancode) in &keymap {
        scancode_names.insert(*scancode, scancode_name.clone());
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

    #[test]
    fn physical_layout_leds_logical() {
        for i in layouts() {
            let layout = Layout::from_board(i).unwrap();
            let logical_in_physical = layout
                .physical
                .keys
                .iter()
                .map(|i| i.logical_name())
                .collect::<HashSet<_>>();
            let logical_in_layout = layout.layout.keys().cloned().collect::<HashSet<_>>();
            let logical_in_leds = layout.layout.keys().cloned().collect::<HashSet<_>>();
            assert_eq!(
                &logical_in_physical - &logical_in_layout,
                HashSet::new(),
                "{}",
                i
            );
            assert_eq!(
                &logical_in_layout - &logical_in_physical,
                HashSet::new(),
                "{}",
                i
            );
            assert_eq!(
                &logical_in_physical - &logical_in_leds,
                HashSet::new(),
                "{}",
                i
            );
            assert_eq!(
                &logical_in_leds - &logical_in_physical,
                HashSet::new(),
                "{}",
                i
            );
        }
    }

    #[test]
    fn layout_has_f_keys() {
        for i in layouts() {
            if *i == "system76/launch_lite_1" {
                continue;
            }

            let layout = Layout::from_board(i).unwrap();
            assert_eq!(layout.f_keys().count(), 12);
        }
    }
}
