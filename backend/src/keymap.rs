use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::BTreeMap,
    convert::TryFrom,
    io::{Read, Write},
};

use crate::{Hs, Layout, Mode};

mod hs_serde {
    use super::*;

    pub fn serialize<S: Serializer>(color: &Hs, serializer: S) -> Result<S::Ok, S::Error> {
        color.to_ints().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Hs, D::Error> {
        let (h, s) = <(u8, u8)>::deserialize(deserializer)?;
        Ok(Hs::from_ints(h, s))
    }
}

mod hs_map_serde {
    use super::*;

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<String, Option<Hs>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let map = map
            .iter()
            .map(|(k, hs)| (k, hs.map(|hs| hs.to_ints())))
            .collect::<BTreeMap<_, _>>();
        map.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<String, Option<Hs>>, D::Error> {
        let map = <BTreeMap<String, Option<(u8, u8)>>>::deserialize(deserializer)?;
        Ok(map
            .into_iter()
            .map(|(k, v)| (k, v.map(|(h, s)| Hs::from_ints(h, s))))
            .collect())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyMapLayer {
    pub mode: Option<(u8, u8)>,
    pub brightness: i32,
    #[serde(with = "hs_serde")]
    pub color: Hs,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyMap {
    pub model: String,
    pub version: u8,
    pub map: BTreeMap<String, Vec<String>>,
    #[serde(with = "hs_map_serde")]
    pub key_leds: BTreeMap<String, Option<Hs>>,
    pub layers: Vec<KeyMapLayer>,
}

impl KeyMap {
    /// Validate an exported profile completely before any device writes are queued.
    /// Partial keymaps remain supported by the regular import path, but a quick
    /// switch must describe every key and lighting layer to be reproducible.
    pub fn validate_for(&self, layout: &Layout, max_brightness: i32) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!("Unsupported keymap version: {}", self.version));
        }
        if self.model != layout.default.model {
            return Err(format!("Keymap is for a different board: {}", self.model));
        }
        if self.map.len() != layout.default.map.len() {
            return Err("Profile must contain every key on the keyboard".into());
        }
        for name in layout.default.map.keys() {
            let codes = self
                .map
                .get(name)
                .ok_or_else(|| format!("Missing key: {name}"))?;
            if codes.len() != usize::from(layout.meta.num_layers) {
                return Err(format!("Incorrect number of layers for key: {name}"));
            }
            for code in codes {
                if layout.scancode_from_name(code).is_none() {
                    return Err(format!("Unknown scancode for {name}: {code}"));
                }
            }
        }
        // Older bundled defaults may omit LEDs that are present in an actual
        // export. The physical layout, not that historical file, is authoritative.
        let led_keys: std::collections::BTreeSet<_> = layout
            .leds
            .iter()
            .filter(|(_, leds)| !leds.is_empty())
            .map(|(name, _)| name)
            .collect();
        if self.key_leds.keys().ne(led_keys) {
            return Err("Profile LEDs do not match the keyboard".into());
        }
        let layer_count = if layout.meta.has_per_layer {
            layout.meta.num_layers
        } else {
            1
        };
        if self.layers.len() != usize::from(layer_count) {
            return Err("Incorrect number of lighting layers".into());
        }
        for layer in &self.layers {
            if layout.meta.has_mode != layer.mode.is_some() {
                return Err("Profile lighting modes do not match the keyboard".into());
            }
            if let Some((mode, _)) = layer.mode {
                if Mode::from_index(mode).is_none() {
                    return Err(format!("Unknown lighting mode: {mode}"));
                }
            }
            if !(0..=max_brightness).contains(&layer.brightness) {
                return Err(format!("Brightness must be between 0 and {max_brightness}"));
            }
        }
        Ok(())
    }

    /// Parse layout from json file
    pub fn from_reader<R: Read>(rdr: R) -> serde_json::Result<Self> {
        serde_json::from_reader(rdr)
    }

    /// Write layout to json file, pretty printed
    pub fn to_writer_pretty<W: Write>(&self, wtr: W) -> serde_json::Result<()> {
        serde_json::to_writer_pretty(wtr, self)
    }

    /// Write layout to json string, pretty printed
    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

impl TryFrom<&str> for KeyMap {
    type Error = serde_json::Error;
    fn try_from(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout::from_board("system76/launch_heavy_1", "0.20.0").unwrap()
    }

    #[test]
    fn valid_complete_profile() {
        let layout = layout();
        assert!(layout.default.validate_for(&layout, 255).is_ok());
    }

    #[test]
    fn rejects_unsupported_version_and_wrong_model() {
        let layout = layout();
        let mut profile = layout.default.clone();
        profile.version = 2;
        assert!(profile.validate_for(&layout, 255).is_err());
        profile.version = 1;
        profile.model = "other".into();
        assert!(profile.validate_for(&layout, 255).is_err());
    }

    #[test]
    fn rejects_missing_unknown_keys_and_invalid_scancodes() {
        let layout = layout();
        let mut profile = layout.default.clone();
        let name = profile.map.keys().next().unwrap().clone();
        let codes = profile.map.remove(&name).unwrap();
        assert!(profile.validate_for(&layout, 255).is_err());
        profile.map.insert("unknown".into(), codes.clone());
        assert!(profile.validate_for(&layout, 255).is_err());
        profile.map.remove("unknown");
        profile.map.insert(name.clone(), codes);
        profile.map.get_mut(&name).unwrap().push("A".into());
        assert!(profile.validate_for(&layout, 255).is_err());
        profile.map.get_mut(&name).unwrap().pop();
        profile.map.get_mut(&name).unwrap()[0] = "not a key".into();
        assert!(profile.validate_for(&layout, 255).is_err());
    }

    #[test]
    fn rejects_invalid_lighting() {
        let layout = layout();
        let mut profile = layout.default.clone();
        profile.layers.push(profile.layers[0].clone());
        assert!(profile.validate_for(&layout, 255).is_err());
        profile.layers.pop();
        profile.layers[0].mode = Some((255, 0));
        assert!(profile.validate_for(&layout, 255).is_err());
        profile.layers[0].mode = None;
        assert!(profile.validate_for(&layout, 255).is_err());
        profile.layers[0] = layout.default.layers[0].clone();
        profile.layers[0].brightness = -1;
        assert!(profile.validate_for(&layout, 255).is_err());
        profile.layers[0].brightness = 256;
        assert!(profile.validate_for(&layout, 255).is_err());
        profile.layers[0].brightness = 100;
        profile.key_leds.clear();
        assert!(profile.validate_for(&layout, 255).is_err());
    }
}
