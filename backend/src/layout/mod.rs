use cascade::cascade;
use regex::Regex;
use std::{collections::HashMap, convert::TryFrom, fs, path::Path, str::FromStr};

mod meta;
use once_cell::sync::Lazy;
mod physical_layout;
pub use self::meta::Meta;
pub(crate) use physical_layout::{PhysicalLayout, PhysicalLayoutKey};

use crate::KeyMap;

// Merge date of https://github.com/system76/ec/pull/229
// Before this, `PAUSE` will not work.
const EC_PAUSE_DATE: (u16, u16, u16) = (2022, 5, 23);
// https://github.com/system76/ec/pull/263
const EC_FNLOCK_DATE: (u16, u16, u16) = (2023, 8, 1);

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
    ($( ($board:expr, $keyboard:expr, $is_qmk:expr) ),* $(,)?) => {
        fn layout_data(board: &str, use_legacy_scancodes: bool) -> Option<(&'static str, &'static str, &'static str, &'static str, &'static str, &'static str)> {
            match board {
                $(
                $board => {
                    let meta_json =
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/", $board, "/meta.json"));
                    let default_json =
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/", $board, "/default.json"));
                    let keymap_json = if use_legacy_scancodes && $is_qmk {
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/keymap/qmk_legacy.json"))
                    } else if $is_qmk {
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/keymap/qmk.json"))
                    } else {
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../layouts/keymap/ec.json"))
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
    #[allow(clippy::too_many_arguments)]
    pub fn from_data(
        board: &str,
        meta_json: &str,
        default_json: &str,
        keymap_json: &str,
        layout_json: &str,
        leds_json: &str,
        physical_json: &str,
        version: &str,
        use_legacy_scancodes: bool,
    ) -> Self {
        let meta: Meta = serde_json::from_str(meta_json).unwrap();
        let mut default = KeyMap::try_from(default_json).unwrap();

        let has_pause_scancode = if meta.is_qmk {
            true
        } else {
            parse_ec_date(version).map_or(true, |date| date >= EC_PAUSE_DATE)
        };
        if !has_pause_scancode {
            keymap_remove_pause(&mut default);
        }

        let has_fnlock_scancode = if meta.is_qmk {
            false
        } else {
            parse_ec_date(version).map_or(true, |date| date >= EC_FNLOCK_DATE)
        };
        if !has_fnlock_scancode {
            keymap_remove_fnlock(&mut default);
        }

        let (keymap, scancode_names) = parse_keymap_json(
            keymap_json,
            board,
            &meta,
            has_pause_scancode,
            has_fnlock_scancode,
        );
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
    pub fn from_dir<P: AsRef<Path>>(board: &str, dir: P) -> Self {
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
            board,
            &meta_json,
            &default_json,
            &keymap_json,
            &layout_json,
            &leds_json,
            &physical_json,
            "dummy",
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
                    board,
                    meta_json,
                    default_json,
                    keymap_json,
                    layout_json,
                    leds_json,
                    physical_json,
                    version,
                    use_legacy_scancodes,
                )
            },
        )
    }

    /// Get the scancode number corresponding to a name
    pub fn scancode_to_name(&self, scancode: u16) -> Option<String> {
        if self.meta.is_qmk {
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
                return Some(format!("MT({}, {})", mod_name, kc_name));
            }
        }
        self.scancode_names.get(&scancode).cloned()
    }

    /// Get the name corresponding to a scancode number
    pub fn scancode_from_name(&self, name: &str) -> Option<u16> {
        if self.meta.is_qmk {
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
                return Some(qk_mod_tap | ((mod_ & 0x1f) << 8) | (kc & 0xff));
            }
        }
        self.keymap.get(name).copied()
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

fn parse_keymap_json(
    keymap_json: &str,
    board: &str,
    meta: &Meta,
    has_pause_scancode: bool,
    has_fnlock_scancode: bool,
) -> (HashMap<String, u16>, HashMap<u16, String>) {
    let mut keymap: HashMap<String, u16> = serde_json::from_str(keymap_json).unwrap();

    // Filter out keycodes that aren't relevant to this particular model
    // TODO: Support bonw backlight over USB?
    if meta.has_color || board == "system76/bonw14" || board == "system76/bonw15" {
        keymap.remove("KBD_BKL");
    } else if meta.has_brightness {
        keymap.remove("KBD_COLOR");
    } else {
        for i in ["KBD_COLOR", "KBD_DOWN", "KBD_UP", "KBD_BKL", "KBD_TOGGLE"] {
            keymap.remove(i);
        }
    }

    if !has_pause_scancode {
        keymap.remove("PAUSE");
    }
    if !has_fnlock_scancode {
        keymap.remove("FNLOCK");
    }

    // Generate reverse mapping, from scancode to names
    let mut scancode_names = HashMap::new();
    for (scancode_name, scancode) in &keymap {
        scancode_names.insert(*scancode, scancode_name.clone());
    }

    (keymap, scancode_names)
}

fn parse_ec_date(version: &str) -> Option<(u16, u16, u16)> {
    let groups = Regex::new(r"^(\d+)-(\d+)-(\d+)_")
        .unwrap()
        .captures(version)?;
    let mut groups = groups
        .iter()
        .skip(1)
        .map(|g| u16::from_str(g.unwrap().as_str()).unwrap());
    Some((
        groups.next().unwrap(),
        groups.next().unwrap(),
        groups.next().unwrap(),
    ))
}

fn keymap_remove_pause(keymap: &mut KeyMap) {
    for values in keymap.map.values_mut() {
        if values.get(1).map(String::as_str) == Some("PAUSE") {
            // Change `PAUSE` on layer 1 to match layer 0
            values[1] = values[0].clone();
        }
    }
}

fn keymap_remove_fnlock(keymap: &mut KeyMap) {
    for values in keymap.map.values_mut() {
        if values.get(1).map(String::as_str) == Some("FNLOCK") {
            // Change `FNLOCK` on layer 1 to match layer 0
            values[1] = values[0].clone();
        }
    }
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
                    || k == "FNLOCK"
                {
                    continue;
                }
                assert_eq!(layout_qmk.keymap.keys().find(|x| x == &k), Some(k));
            }
        }
    }

    #[test]
    fn color_brightness_keycodes() {
        const VERSION: &str = "0.19.12";

        let layout_no_color = Layout::from_board("system76/lemp10", VERSION).unwrap();
        assert!(
            layout_no_color.keymap.contains_key("KBD_BKL")
                && !layout_no_color.keymap.contains_key("KBD_COLOR")
        );

        let layout_color = Layout::from_board("system76/gaze15", VERSION).unwrap();
        assert!(
            layout_color.keymap.contains_key("KBD_COLOR")
                && !layout_color.keymap.contains_key("KBD_BKL")
        );

        let layout_bonw = Layout::from_board("system76/bonw14", VERSION).unwrap();
        assert!(
            layout_bonw.keymap.contains_key("KBD_COLOR")
                && !layout_bonw.keymap.contains_key("KBD_BKL")
        );
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
