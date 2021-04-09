use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyMap {
    pub board: String,
    pub map: HashMap<String, Vec<String>>,
}

impl KeyMap {
    pub fn from_reader<R: Read>(rdr: R) -> serde_json::Result<Self> {
        serde_json::from_reader(rdr)
    }

    pub fn from_str(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }

    pub fn to_writer_pretty<W: Write>(&self, wtr: W) -> serde_json::Result<()> {
        serde_json::to_writer_pretty(wtr, self)
    }

    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}
