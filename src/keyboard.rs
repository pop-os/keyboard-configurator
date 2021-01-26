use anyhow::{Error, Result};
use gio::prelude::*;
use glib::clone;
use glib::clone::{Downgrade, Upgrade};
use glib::variant::Variant;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::iter::Iterator;
use std::rc::{Rc, Weak};

use crate::color::Rgb;
use crate::daemon::Daemon;

const DBUS_NAME: &'static str = "com.system76.PowerDaemon";
const DBUS_KEYBOARD_IFACE: &'static str = "com.system76.PowerDaemon.Keyboard";

enum KeyboardImplementation {
    #[cfg(target_os = "linux")]
    S76Power {
        proxy: gio::DBusProxy,
        properties_proxy: gio::DBusProxy,
    },
    Dummy {
        properties: HashMap<&'static str, RefCell<Variant>>,
    },
    Daemon {
        daemon: Rc<dyn Daemon>,
        board: usize,
    }
}

struct KeyboardInner {
    implementation: KeyboardImplementation,
    brightness_changed_handlers: RefCell<Vec<Box<dyn Fn(&Keyboard, i32) + 'static>>>,
    color_changed_handlers: RefCell<Vec<Box<dyn Fn(&Keyboard, Rgb) + 'static>>>,
    brightness_set_cancellable: Cell<gio::Cancellable>,
    color_set_cancellable: Cell<gio::Cancellable>,
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

fn set_property(
    properties_proxy: &gio::DBusProxy,
    property: &str,
    value: glib::Variant,
    cancellable: &gio::Cancellable,
) -> Result<()> {
    let args = (
        DBUS_KEYBOARD_IFACE,
        property,
        value,
    );
    properties_proxy.call(
        "Set",
        Some(&args.to_variant()),
        gio::DBusCallFlags::NONE,
        60000,
        Some(cancellable),
        |_| {},
    );
    Ok(())
}

impl Keyboard {
    fn new(implementation: KeyboardImplementation) -> Self {
        Self(Rc::new(KeyboardInner {
            implementation,
            brightness_changed_handlers: RefCell::new(Vec::new()),
            color_changed_handlers: RefCell::new(Vec::new()),
            brightness_set_cancellable: Cell::new(gio::Cancellable::new()),
            color_set_cancellable: Cell::new(gio::Cancellable::new()),
        }))
    }

    #[cfg(target_os = "linux")]
    fn new_s76_power(path: &str) -> Self {
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

    pub(crate) fn new_daemon(daemon: Rc<dyn Daemon>, board: usize) -> Self {
        Self::new(KeyboardImplementation::Daemon {
            daemon,
            board,
        })
    }

    pub(crate) fn new_dummy() -> Self {
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
                            if let Some(color) = changed.lookup_value("color", None) {
                                if let Some(color) = Rgb::parse(&color.get::<String>().unwrap()) {
                                    self_.color_changed(color);
                                }
                            }
                            None
                        }),
                    )
                    .unwrap();
            }
            KeyboardImplementation::Dummy { .. } => {}
            KeyboardImplementation::Daemon { .. } => {}
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
            KeyboardImplementation::Daemon { ref daemon, board } => {
                match name {
                    "max-brightness" => daemon.max_brightness(*board).map(|b| Some(b.to_variant())).map_err(Error::msg),
                    "brightness" => daemon.brightness(*board).map(|b| Some(b.to_variant())).map_err(Error::msg),
                    "color" => daemon.color(*board).map(|b| Some(b.to_string().to_variant())).map_err(Error::msg),
                    "name" => Ok(Some("".to_variant())),
                    _ => unreachable!(),
                }
            }
        }
    }

    fn set_prop(&self, name: &'static str, value: Variant, cancellable: &gio::Cancellable) -> Result<()> {
        match &self.0.implementation {
            #[cfg(target_os = "linux")]
            KeyboardImplementation::S76Power {
                properties_proxy, ..
            } => {
                set_property(properties_proxy, name, value, cancellable)?;
            }
            KeyboardImplementation::Dummy { properties } => {
                *properties.get(name).unwrap().borrow_mut() = value;
            }
            KeyboardImplementation::Daemon { ref daemon, board } => {
                match name {
                    "brightness" => daemon.set_brightness(*board, value.get().unwrap()).map_err(Error::msg)?,
                    "color" => daemon.set_color(*board, Rgb::parse(&value.get::<String>().unwrap()).unwrap()).map_err(Error::msg)?,
                    _ => unreachable!(),
                };
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
        let cancellable = gio::Cancellable::new();
        self.set_prop("color", color.to_string().to_variant(), &cancellable)?;
        self.0.color_set_cancellable.replace(cancellable).cancel();
        Ok(())
    }

    fn color_changed(&self, color: Rgb) {
        for handler in self.0.color_changed_handlers.borrow().iter() {
            handler(self, color);
        }
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
        let cancellable = gio::Cancellable::new();
        self.set_prop("brightness", brightness.to_variant(), &cancellable)?;
        self.0.brightness_set_cancellable.replace(cancellable).cancel();
        Ok(())
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

    pub fn connect_color_changed<F: Fn(&Self, Rgb) + 'static>(&self, f: F) {
        self.0
            .color_changed_handlers
            .borrow_mut()
            .push(std::boxed::Box::new(f) as Box<dyn Fn(&Self, Rgb)>);
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

impl Default for Keyboard {
    fn default() -> Self {
        Self::new_dummy()
    }
}

#[cfg(target_os = "linux")]
fn add_s76power_keyboards(keyboards: &mut Vec<Keyboard>) -> Result<()> {
    let proxy = gio::DBusProxy::new_for_bus_sync::<gio::Cancellable>(
        gio::BusType::System,
        gio::DBusProxyFlags::NONE,
        None,
        DBUS_NAME,
        "/com/system76/PowerDaemon",
        "org.freedesktop.DBus.ObjectManager",
        None,
    )?;
    let ret = proxy
        .call_sync::<gio::Cancellable>(
            "GetManagedObjects",
            None,
            gio::DBusCallFlags::NONE,
            60000,
            None,
        )?;

    for i in ret.get_child_value(0).iter() {
        let path = i.get_child_value(0)
            .get::<String>()
            .unwrap();
        if path.starts_with("/com/system76/PowerDaemon/keyboard") {
            keyboards.push(Keyboard::new_s76_power(&path));
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn keyboards() -> impl Iterator<Item = Keyboard> {
    let mut keyboards = Vec::new();

    add_s76power_keyboards(&mut keyboards);
    keyboards.push(Keyboard::new_dummy());

    keyboards.into_iter()
}

#[cfg(any(windows, target_os = "macos"))]
pub fn keyboards() -> impl Iterator<Item = Keyboard> {
    vec![Keyboard::new_dummy()].into_iter()
}
