use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct Devices {
    pub Devices: Vec<Device>,
}

impl Devices {
    pub fn get_launch(self) -> Result<Device, Error> {
        self.Devices
            .into_iter()
            .find(|device| {
                device.name().unwrap_or(&"".to_string()).contains("Launch")
                    && device.vendor().unwrap_or(&"".to_string()) == "System76"
            })
            .ok_or(Error::NoLaunch)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct Device {
    Name: Option<String>,
    Vendor: Option<String>,
    Created: usize,
    Releases: Option<Vec<Release>>,
}

impl Device {
    pub fn name(&self) -> Option<&String> {
        self.Name.as_ref()
    }

    pub fn vendor(&self) -> Option<&String> {
        self.Vendor.as_ref()
    }

    pub fn created(&self) -> usize {
        self.Created
    }

    pub fn is_newer_release(self) -> Result<bool, Error> {
        Ok(self
            .Releases
            .as_ref()
            .ok_or(Error::NoRelease)?
            .into_iter()
            .any(|release| release.created() > self.created()))
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct Release {
    Created: usize,
}

impl Release {
    pub fn created(&self) -> usize {
        self.Created
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Error {
    Fwupdmgr,
    Json,
    NoLaunch,
    NoRelease,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Fwupdmgr => write!(f, "Failed running the fwupdmgr command"),
            Error::Json => write!(f, "Return data from fwupd was not parsed correctly"),
            Error::NoLaunch => write!(
                f,
                "Unable to find a device with appstream id `com.system76.launch*` via fwupdmgr"
            ),
            Error::NoRelease => write!(
                f,
                "Unable to find a current release for the connected launch."
            ),
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
        Error::Json
    }
}

impl From<serde_json::Error> for Error {
    fn from(_e: serde_json::Error) -> Self {
        Error::Json
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(_e: std::num::ParseIntError) -> Self {
        Error::Json
    }
}

pub fn is_launch_updated() -> Result<bool, Error> {
    // Get all fwupd devices
    let stdout = Command::new("fwupdmgr")
        .args(["get-devices", "--json"])
        .output()?
        .stdout;
    info!("before json");
    let json = std::str::from_utf8(&stdout)?;
    info!("before devices");
    let devices: Devices = serde_json::from_str(json).unwrap();

    devices.get_launch()?.is_newer_release()
}
