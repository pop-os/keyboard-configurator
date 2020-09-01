use anyhow::{Error, Result};
use gio::prelude::*;
use glib::clone;
use glib::clone::{Downgrade, Upgrade};
use glib::translate::{from_glib_none, ToGlibPtr};
use glib::variant::Variant;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::iter::Iterator;
use std::rc::{Rc, Weak};

use crate::color::Rgb;

const DBUS_NAME: &'static str = "com.system76.PowerDaemon";
const DBUS_KEYBOARD_IFACE: &'static str = "com.system76.PowerDaemon.Keyboard";

pub enum KeyboardPattern {
    Solid,
    Breathe,
    Wave,
    Snake,
    Random,
}

enum KeyboardImplementation {
    #[cfg(target_os = "linux")]
    S76Power {
        proxy: gio::DBusProxy,
        properties_proxy: gio::DBusProxy,
    },
    Dummy {
        properties: HashMap<&'static str, RefCell<Variant>>,
    },
}

struct KeyboardInner {
    implementation: KeyboardImplementation,
    brightness_changed_handlers: RefCell<Vec<Box<dyn Fn(&Keyboard, i32) + 'static>>>,
}

#[derive(Clone)]
pub struct Keyboard(Rc<KeyboardInner>);

pub struct KeyboardWeak(Weak<KeyboardInner>);

impl Downgrade for Keyboard {
    type Weak = KeyboardWeak;

    fn downgrade(&self) -> Self::Weak {
        KeyboardWeak(self.0.downgrade())
    }
}

impl Upgrade for KeyboardWeak {
    type Strong = Keyboard;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(Keyboard)
    }
}

// https://github.com/gtk-rs/glib/pull/651
unsafe fn variant_get_child_value(variant: &glib::Variant, index: usize) -> glib::Variant {
    from_glib_none(glib_sys::g_variant_get_child_value(
        variant.to_glib_none().0,
        index,
    ))
}

fn set_property(
    properties_proxy: &gio::DBusProxy,
    property: &str,
    value: glib::Variant,
) -> Result<()> {
    let variant: glib::Variant =
        unsafe { from_glib_none(glib_sys::g_variant_new_variant(value.to_glib_none().0)) };
    let args: glib::Variant = unsafe {
        from_glib_none(glib_sys::g_variant_new_tuple(
            vec![
                DBUS_KEYBOARD_IFACE.to_variant(),
                property.to_variant(),
                variant,
            ]
            .to_glib_none()
            .0,
            3,
        ))
    };
    properties_proxy.call_sync::<gio::Cancellable>(
        "Set",
        Some(&args),
        gio::DBusCallFlags::NONE,
        60000,
        None,
    )?;
    Ok(())
}

impl Keyboard {
    fn new(implementation: KeyboardImplementation) -> Self {
        Self(Rc::new(KeyboardInner {
            implementation,
            brightness_changed_handlers: RefCell::new(Vec::new()),
        }))
    }

    #[cfg(target_os = "linux")]
    fn new_s76Power(path: &str) -> Self {
        // XXX unwrap
        let proxy = gio::DBusProxy::new_for_bus_sync::<gio::Cancellable>(
            gio::BusType::System,
            gio::DBusProxyFlags::NONE,
            None,
            DBUS_NAME,
            path,
            DBUS_KEYBOARD_IFACE,
            None,
        )
        .unwrap();
        let properties_proxy = gio::DBusProxy::new_for_bus_sync::<gio::Cancellable>(
            gio::BusType::System,
            gio::DBusProxyFlags::NONE,
            None,
            DBUS_NAME,
            path,
            "org.freedesktop.DBus.Properties",
            None,
        )
        .unwrap();
        let keyboard = Self::new(KeyboardImplementation::S76Power {
            proxy,
            properties_proxy,
        });
        keyboard.connect_signals();
        keyboard
    }

    fn new_dummy() -> Self {
        let mut properties = HashMap::new();
        properties.insert("max_brightness", RefCell::new(100i32.to_variant()));
        properties.insert("brightness", RefCell::new(0i32.to_variant()));
        properties.insert("color", RefCell::new("ff0000".to_variant()));
        properties.insert("name", RefCell::new("Dummy Keyboard".to_variant()));
        Self::new(KeyboardImplementation::Dummy { properties })
    }

    fn connect_signals(&self) {
        let self_ = self;

        match &self.0.implementation {
            #[cfg(target_os = "linux")]
            KeyboardImplementation::S76Power { proxy, .. } => {
                proxy
                    .connect_local(
                        "g-properties-changed",
                        true,
                        clone!(@weak self_ => @default-panic, move |args| {
                            let changed = args[1].get::<glib::Variant>().unwrap().unwrap().get::<glib::VariantDict>().unwrap();
                            if let Some(brightness) = changed.lookup_value("brightness", None) {
                                self_.brightness_changed(brightness.get::<i32>().unwrap());
                            }
                            None
                        }),
                    )
                    .unwrap();
            }
            KeyboardImplementation::Dummy { .. } => {}
        }
    }

    fn prop(&self, name: &'static str) -> Result<Option<Variant>> {
        match &self.0.implementation {
            #[cfg(target_os = "linux")]
            KeyboardImplementation::S76Power { ref proxy, .. } => {
                Ok(proxy.get_cached_property(name))
            }
            KeyboardImplementation::Dummy { properties } => {
                Ok(Some(properties.get(name).unwrap().borrow().clone()))
            }
        }
    }

    pub fn set_prop(&self, name: &'static str, value: Variant) -> Result<()> {
        match &self.0.implementation {
            #[cfg(target_os = "linux")]
            KeyboardImplementation::S76Power {
                properties_proxy, ..
            } => {
                set_property(properties_proxy, name, value)?;
            }
            KeyboardImplementation::Dummy { properties } => {
                *properties.get(name).unwrap().borrow_mut() = value;
            }
        }
        Ok(())
    }

    /// Returns `true` if the keyboard has a backlight capable of setting color
    pub fn has_color(&self) -> Result<bool> {
        Ok(true)
    }

    /// Gets backlight color
    pub fn color(&self) -> Result<Rgb> {
        let color = match self.prop("color")? {
            Some(value) => value.get().unwrap(),
            None => "".to_string(),
        };
        Ok(Rgb::parse(&color).unwrap_or(Rgb::new(0, 0, 0)))
    }

    /// Sets backlight color
    pub fn set_color(&self, color: Rgb) -> Result<()> {
        self.set_prop("color", color.to_string().to_variant())
    }

    /// Returns `true` if the keyboard has a backlight capable of setting brightness
    pub fn has_brightness(&self) -> Result<bool> {
        Ok(true)
    }

    /// Gets backlight brightness
    pub fn brightness(&self) -> Result<i32> {
        Ok(self
            .prop("brightness")?
            .map(|v| v.get().unwrap())
            .unwrap_or(0))
    }

    /// Sets backlight brightness
    pub fn set_brightness(&self, brightness: i32) -> Result<()> {
        self.set_prop("brightness", brightness.to_variant())
    }

    /// Gets maximum brightness that can be set
    pub fn max_brightness(&self) -> Result<i32> {
        Ok(self
            .prop("max_brightness")?
            .map(|v| v.get().unwrap())
            .unwrap_or(100))
    }

    fn brightness_changed(&self, brightness: i32) {
        for handler in self.0.brightness_changed_handlers.borrow().iter() {
            handler(self, brightness);
        }
    }

    pub fn connect_brightness_changed<F: Fn(&Self, i32) + 'static>(&self, f: F) {
        self.0
            .brightness_changed_handlers
            .borrow_mut()
            .push(std::boxed::Box::new(f) as Box<dyn Fn(&Self, i32)>);
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
        if let Ok(Some(value)) = self.prop("name") {
            write!(f, "{}", value.get::<String>().unwrap())
        } else {
            write!(f, "")
        }
    }
}

#[cfg(target_os = "linux")]
pub fn keyboards() -> impl Iterator<Item = Keyboard> {
    // XXX unwrap
    let proxy = gio::DBusProxy::new_for_bus_sync::<gio::Cancellable>(
        gio::BusType::System,
        gio::DBusProxyFlags::NONE,
        None,
        DBUS_NAME,
        "/com/system76/PowerDaemon",
        "org.freedesktop.DBus.ObjectManager",
        None,
    )
    .unwrap();
    let ret = proxy
        .call_sync::<gio::Cancellable>(
            "GetManagedObjects",
            None,
            gio::DBusCallFlags::NONE,
            60000,
            None,
        )
        .unwrap();

    let mut keyboards = Vec::new();

    let dict = unsafe { variant_get_child_value(&ret, 0) };
    let iter = unsafe { glib_sys::g_variant_iter_new(dict.to_glib_none().0) };
    loop {
        let i = unsafe { glib_sys::g_variant_iter_next_value(iter) };
        if i.is_null() {
            break;
        }
        let i: glib::Variant = unsafe { from_glib_none(i) };
        let path = unsafe { variant_get_child_value(&i, 0) }
            .get::<String>()
            .unwrap();
        if path.starts_with("/com/system76/PowerDaemon/keyboard") {
            keyboards.push(Keyboard::new_s76Power(&path));
        }
    }
    unsafe { glib_sys::g_variant_iter_free(iter) };

    keyboards.push(Keyboard::new_dummy());

    keyboards.into_iter()
}

#[cfg(any(windows, target_os = "macos"))]
pub fn keyboards() -> impl Iterator<Item = Keyboard> {
    vec![Keyboard::new_dummy()].into_iter()
}
