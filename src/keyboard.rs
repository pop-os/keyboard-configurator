use anyhow::{Error, Result};
use std::cell::Cell;
use std::fmt;
use std::iter::Iterator;

use crate::color::Rgb;

enum KeyboardPattern {
    Solid,
    Breathe,
    Wave,
    Snake,
    Random
}

#[derive(Clone)]
pub enum Keyboard {
    #[cfg(target_os = "linux")]
    S76Power,
    Dummy(Cell<Rgb>, Cell<i32>),
}

impl Keyboard {
    #[cfg(target_os = "linux")]
    fn new_s76Power() -> Self {
        Self::S76Power
    }

    fn new_dummy() -> Self {
        Self::Dummy(Cell::new(Rgb::new(0, 0, 0)), Cell::new(0))
    }

    /// Returns `true` if the keyboard has a backlight capable of setting color
    pub fn has_color(&self) -> Result<bool> {
        Ok(true)
    }

    /// Gets backlight color
    pub fn color(&self) -> Result<Rgb> {
        match self {
            #[cfg(target_os = "linux")]
            Self::S76Power => {
                use system76_power::{client::PowerClient, Power};
                let mut client = PowerClient::new().map_err(Error::msg)?;
                let color_str = client.get_keyboard_color().map_err(Error::msg)?;
                Rgb::parse(&color_str).ok_or(Error::msg("Invalid color string"))
            }
            Self::Dummy(ref c, _) => Ok(c.get()),
        }
    }

    /// Sets backlight color
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
            Self::Dummy(ref c, _) => c.set(color),
        }
        Ok(())
    }

    /// Returns `true` if the keyboard has a backlight capable of setting brightness
    pub fn has_brightness(&self) -> Result<bool> {
        Ok(true)
    }

    /// Gets backlight brightness
    pub fn brightness(&self, brightness: i32) -> Result<i32> {
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
                    "GetBrightness",
                    (),
                )?;
                Ok(brightness)
            }
            Self::Dummy(_, b) => Ok(b.get()),
        }
    }

    /// Sets backlight brightness
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
            Self::Dummy(_, b) => b.set(brightness),
        }
        Ok(())
    }

    /// Gets maximum brightness that can be set
    pub fn max_brightness(&self) -> Result<i32> {
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
            Self::Dummy(_, _) => Ok(100),
        }
    }

    /// Returns `true` if the keyboard has a backlight capable of patterns
    pub fn has_pattern(&self) -> Result<bool> {
        Ok(false)
    }

    /// Gets backlight pattern
    pub fn pattern(&self) -> Result<KeyboardPattern> {
        // XXX
        Ok(KeyboardPattern::Solid)
    }

    /// Sets backlight pattern
    pub fn set_pattern(&self, _pattern: KeyboardPattern) -> Result<()> {
        // XXX
        Ok(())
    }
}

impl fmt::Display for Keyboard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(target_os = "linux")]
            Self::S76Power => write!(f, "system76-power Keyboard"),
            Self::Dummy(_, _) => write!(f, "Dummy Keyboard"),
        }
    }
}

#[cfg(target_os = "linux")]
pub fn keyboards() -> impl Iterator<Item = Keyboard> {
    vec![Keyboard::new_s76Power(), Keyboard::new_dummy()].into_iter()
}

#[cfg(any(windows, target_os = "macos"))]
pub fn keyboards() -> impl Iterator<Item = Keyboard> {
    vec![Keyboard::new_dummy()].into_iter()
}
