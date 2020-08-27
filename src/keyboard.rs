use anyhow::{Error, Result};
use std::iter::Iterator;
use std::fmt;
#[cfg(target_os = "linux")]
use system76_power::{client::PowerClient, Power};

use crate::color::Rgb;

#[derive(Clone)]
pub enum Keyboard {
    #[cfg(target_os = "linux")]
    S76Power,
    Dummy,
}

impl Keyboard {
    pub fn set_color(&self, color: Rgb) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Self::S76Power => {
                let mut client = PowerClient::new().map_err(Error::msg)?;
                client.set_keyboard_color(&color.to_string()).map_err(Error::msg)?;
            }
            Self::Dummy => {}
        }
        Ok(())
    }
}

impl fmt::Display for Keyboard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(target_os = "linux")]
            Self::S76Power => write!(f, "system76-power Keyboard"),
            Self::Dummy => write!(f, "Dummy Keyboard"),
        }
    }
}

#[cfg(target_os = "linux")]
pub fn keyboards() -> impl Iterator<Item=Keyboard> {
    vec![Keyboard::S76Power, Keyboard::Dummy].into_iter()
}

#[cfg(windows)]
pub fn keyboards() -> impl Iterator<Item=Keyboard> {
    vec![Keyboard::Dummy].into_iter()
}