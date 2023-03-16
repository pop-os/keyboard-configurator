use std::process::Command;

use once_cell::sync::Lazy;
use regex::bytes::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Release {
    created: usize,
}

#[derive(Copy, Clone, Debug)]
pub enum Error {
    Fwupdmgr,
    Utf8,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Fwupdmgr => write!(f, "Failed running the fwupdmgr command"),
            Error::Utf8 => write!(f, "Return data from fwupd was not parsed correctly"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(_e: std::io::Error) -> Self {
        Error::Fwupdmgr
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(_e: std::str::Utf8Error) -> Self {
        Error::Utf8
    }
}

pub fn is_launch_updated() -> Result<bool, Error> {
    let stdout = Command::new("fwupdmgr")
        .args(["get-updates", "--json"])
        .output()?
        .stdout;

    static RE: Lazy<Regex> = Lazy::new(|| Regex::new("Launch.* Configurable Keyboard").unwrap());
    Ok(!RE.is_match(&stdout))
}
