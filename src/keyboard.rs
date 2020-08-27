use anyhow::{Error, Result};
use std::fmt;
use std::iter::Iterator;

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
                use system76_power::{client::PowerClient, Power};
                let mut client = PowerClient::new().map_err(Error::msg)?;
                client
                    .set_keyboard_color(&color.to_string())
                    .map_err(Error::msg)?;
            }
            Self::Dummy => {}
        }
        Ok(())
    }

    pub fn set_brightness(&self, brightness: i32) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Self::S76Power => {
                let conn = dbus::blocking::Connection::new_system()?;
                let proxy = conn.with_proxy(
                    "org.freedesktop.UPower",
                    "/org/freedesktop/UPower/KbdBacklight",
                    std::time::Duration::from_millis(60000),
                );
                proxy.method_call(
                    "org.freedesktop.UPower.KbdBacklight",
                    "SetBrightness",
                    (brightness,),
                )?;
            }
            Self::Dummy => {}
        }
        Ok(())
    }

    pub fn get_max_brightness(&self) -> Result<i32> {
        match self {
            #[cfg(target_os = "linux")]
            Self::S76Power => {
                let conn = dbus::blocking::Connection::new_system()?;
                let proxy = conn.with_proxy(
                    "org.freedesktop.UPower",
                    "/org/freedesktop/UPower/KbdBacklight",
                    std::time::Duration::from_millis(60000),
                );
                let (brightness,) = proxy.method_call(
                    "org.freedesktop.UPower.KbdBacklight",
                    "GetMaxBrightness",
                    (),
                )?;
                Ok(brightness)
            }
            Self::Dummy => Ok(100),
        }
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
pub fn keyboards() -> impl Iterator<Item = Keyboard> {
    vec![Keyboard::S76Power, Keyboard::Dummy].into_iter()
}

#[cfg(windows)]
pub fn keyboards() -> impl Iterator<Item = Keyboard> {
    vec![Keyboard::Dummy].into_iter()
}
