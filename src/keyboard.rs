use anyhow::{Error, Result};
use gio::prelude::*;
use glib::translate::{from_glib_none, ToGlibPtr};
use std::cell::Cell;
use std::fmt;
use std::iter::Iterator;

use crate::color::Rgb;

enum KeyboardPattern {
    Solid,
    Breathe,
    Wave,
    Snake,
    Random,
}

#[derive(Clone)]
pub enum Keyboard {
    #[cfg(target_os = "linux")]
    S76Power(gio::DBusProxy),
    Dummy(Cell<Rgb>, Cell<i32>),
}

impl Keyboard {
    #[cfg(target_os = "linux")]
    fn new_s76Power() -> Self {
        // XXX unwrap
        let proxy = gio::DBusProxy::new_for_bus_sync::<gio::Cancellable>(
            gio::BusType::System,
            gio::DBusProxyFlags::NONE,
            None,
            "org.freedesktop.UPower",
            "/org/freedesktop/UPower/KbdBacklight",
            "org.freedesktop.UPower.KbdBacklight",
            None,
        )
        .unwrap();
        Self::S76Power(proxy)
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
            Self::S76Power(_) => {
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
            Self::S76Power(_) => {
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
            Self::S76Power(ref proxy) => {
                let ret = proxy.call_sync::<gio::Cancellable>(
                    "GetBrightness",
                    None,
                    gio::DBusCallFlags::NONE,
                    60000,
                    None,
                )?;
                let brightness: glib::Variant = unsafe {
                    from_glib_none(glib_sys::g_variant_get_child_value(ret.to_glib_none().0, 0))
                };
                Ok(brightness.get().unwrap())
            }
            Self::Dummy(_, b) => Ok(b.get()),
        }
    }

    /// Sets backlight brightness
    pub fn set_brightness(&self, brightness: i32) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Self::S76Power(ref proxy) => {
                let args: glib::Variant = unsafe {
                    from_glib_none(glib_sys::g_variant_new_tuple(
                        vec![brightness.to_variant()].to_glib_none().0,
                        1,
                    ))
                };
                let brightness = proxy.call_sync::<gio::Cancellable>(
                    "SetBrightness",
                    Some(&args),
                    gio::DBusCallFlags::NONE,
                    60000,
                    None,
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
            Self::S76Power(ref proxy) => {
                let ret = proxy.call_sync::<gio::Cancellable>(
                    "GetMaxBrightness",
                    None,
                    gio::DBusCallFlags::NONE,
                    60000,
                    None,
                )?;
                let brightness: glib::Variant = unsafe {
                    from_glib_none(glib_sys::g_variant_get_child_value(ret.to_glib_none().0, 0))
                };
                Ok(brightness.get().unwrap())
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
            Self::S76Power(_) => write!(f, "system76-power Keyboard"),
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
