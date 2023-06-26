use cascade::cascade;
use regex::Regex;
use std::{collections::HashMap, fs, path::Path};

mod meta;
use once_cell::sync::Lazy;
mod physical_layout;
pub use self::meta::Meta;
pub(crate) use physical_layout::{PhysicalLayout, PhysicalLayoutKey};

use crate::KeyMap;

const QK_MOD_TAP_LEGACY: u16 = 0x6000;
const QK_MOD_TAP_MAX_LEGACY: u16 = 0x7FFF;
const QK_MOD_TAP: u16 = 0x2000;
const QK_MOD_TAP_MAX: u16 = 0x3FFF;

pub static MOD_TAP_MODS: Lazy<HashMap<&str, u16>> = Lazy::new(|| {
    cascade! {
        HashMap::new();
        ..insert("LEFT_CTRL", 0x01);
        ..insert("LEFT_SHIFT", 0x02);
        ..insert("LEFT_ALT", 0x04);
        ..insert("LEFT_SUPER", 0x08);
        ..insert("RIGHT_CTRL", 0x11);
        ..insert("RIGHT_SHIFT", 0x12);
        ..insert("RIGHT_ALT", 0x14);
        ..insert("RIGHT_SUPER", 0x18);
    }
});

#[derive(Debug)]
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
    use_legacy_scancodes: bool,
}

macro_rules! keyboards {
    ($( ($board:expr, $keyboard:expr) ),* $(,)?) => {
        fn layout_data(board: &str, use_legacy_scancodes: bool) -> Option<(&'static str, &'static str, &'static str, &'static str, &'static str, &'static str)> {
            match board {
                $(
                $board => {
                    let meta_json =
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/", $board, "/meta.json"));
                    let default_json =
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/", $board, "/default.json"));
                    let keymap_json = if use_legacy_scancodes {
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/keyboards/", $keyboard, "/keymap.json"))
                    } else {
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/keyboards/overrides/0.19.12/", $keyboard, "/keymap.json"))
                    };
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
        use_legacy_scancodes: bool,
    ) -> Self {
        let meta = serde_json::from_str(meta_json).unwrap();
        let default = default_json.into();
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
            use_legacy_scancodes,
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
            false,
        )
    }

    pub fn from_board(board: &str, version: &str) -> Option<Self> {
        let use_legacy_scancodes = version.contains("0.7.103")
            || version.contains("0.7.104")
            || version.contains("0.12.20");
        layout_data(board, use_legacy_scancodes).map(
            |(meta_json, default_json, keymap_json, layout_json, leds_json, physical_json)| {
                Self::from_data(
                    meta_json,
                    default_json,
                    keymap_json,
                    layout_json,
                    leds_json,
                    physical_json,
                    use_legacy_scancodes,
                )
            },
        )
    }

    /// Get the scancode number corresponding to a name
    pub fn scancode_to_name(&self, scancode: u16) -> Option<String> {
        let (qk_mod_tap, qk_mod_tap_max) = if self.use_legacy_scancodes {
            (QK_MOD_TAP_LEGACY, QK_MOD_TAP_MAX_LEGACY)
        } else {
            (QK_MOD_TAP, QK_MOD_TAP_MAX)
        };
        if scancode >= qk_mod_tap && scancode < qk_mod_tap_max {
            let mod_ = (scancode >> 8) & 0x1f;
            let kc = scancode & 0xff;
            let mod_name = MOD_TAP_MODS.iter().find(|(_, v)| **v == mod_)?.0;
            let kc_name = self.scancode_names.get(&kc)?;
            Some(format!("MT({}, {})", mod_name, kc_name))
        } else {
            self.scancode_names.get(&scancode).cloned()
        }
    }

    /// Get the name corresponding to a scancode number
    pub fn scancode_from_name(&self, name: &str) -> Option<u16> {
        // Check if mod-tap
        let mt_re = Regex::new("MT\\(([^()]+), ([^()]+)\\)").unwrap();
        if let Some(captures) = mt_re.captures(name) {
            let qk_mod_tap = if self.use_legacy_scancodes {
                QK_MOD_TAP_LEGACY
            } else {
                QK_MOD_TAP
            };
            let mod_ = *MOD_TAP_MODS.get(&captures.get(1).unwrap().as_str())?;
            let kc = *self.keymap.get(captures.get(2).unwrap().as_str())?;
            Some(qk_mod_tap | ((mod_ & 0x1f) << 8) | (kc & 0xff))
        } else {
            self.keymap.get(name).copied()
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

    const VERSIONS: [&str; 3] = ["0.7.103", "0.7.104", "0.19.12"];

    #[test]
    fn layout_from_board() {
        for i in layouts() {
            for version in VERSIONS {
                Layout::from_board(i, version).unwrap();
            }
        }
    }

    #[test]
    fn default_keys_exist() {
        for i in layouts() {
            for version in VERSIONS {
                let mut missing = HashSet::new();
                let layout = Layout::from_board(i, version).unwrap();
                for j in layout.default.map.values().flatten() {
                    if !layout.keymap.keys().any(|x| x == j) {
                        missing.insert(j.to_owned());
                    }
                }
                assert_eq!(missing, HashSet::new(), "Mssing in keymap for {}", i);
            }
        }
    }

    #[test]
    fn qmk_has_ec_keycodes() {
        for version in VERSIONS {
            let layout_ec = Layout::from_board("system76/darp6", version).unwrap();
            let layout_qmk = Layout::from_board("system76/launch_1", version).unwrap();
            for k in layout_ec.keymap.keys() {
                if k == "KBD_COLOR"
                    || k == "KBD_BKL"
                    || k == "TOUCHPAD"
                    || k == "DISPLAY_TOGGLE"
                    || k == "DISPLAY_MODE"
                    || k == "FAN_TOGGLE"
                    || k == "CAMERA_TOGGLE"
                    || k == "AIRPLANE_MODE"
                    || k == "MIC_MUTE"
                {
                    continue;
                }
                assert_eq!(layout_qmk.keymap.keys().find(|x| x == &k), Some(k));
            }
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
            for version in VERSIONS {
                let layout = Layout::from_board(i, version).unwrap();
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
    }

    #[test]
    fn layout_has_f_keys() {
        for i in layouts() {
            if *i == "system76/launch_lite_1" {
                continue;
            }

            for version in VERSIONS {
                let layout = Layout::from_board(i, version).unwrap();
                assert_eq!(layout.f_keys().count(), 12);
            }
        }
    }
}
