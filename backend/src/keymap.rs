use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::str::FromStr;

use crate::Hs;

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
        map: &HashMap<String, Option<Hs>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let map = map
            .iter()
            .map(|(k, hs)| (k, hs.map(|hs| hs.to_ints())))
            .collect::<HashMap<_, _>>();
        map.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<String, Option<Hs>>, D::Error> {
        let map = <HashMap<String, Option<(u8, u8)>>>::deserialize(deserializer)?;
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
    pub map: HashMap<String, Vec<String>>,
    #[serde(with = "hs_map_serde")]
    pub key_leds: HashMap<String, Option<Hs>>,
    pub layers: Vec<KeyMapLayer>,
}

impl KeyMap {
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

impl FromStr for KeyMap {
    type Err = serde_json::Error;

    /// Parse layout from json string
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}
