// Note: Linux only
// Need to watch properties of each object?
// TODO: Hotplug detection support

use gio::prelude::*;
use glib::variant::{FromVariant, ToVariant};
use std::{cell::Cell, iter::Iterator};

use super::{err_str, BoardId, Daemon};
use crate::color::Rgb;

const DBUS_NAME: &str = "com.system76.PowerDaemon";
const DBUS_KEYBOARD_IFACE: &str = "com.system76.PowerDaemon.Keyboard";

struct Keyboard {
    proxy: gio::DBusProxy,
    properties_proxy: gio::DBusProxy,
    brightness_set_cancellable: Cell<gio::Cancellable>,
    color_set_cancellable: Cell<gio::Cancellable>,
}

impl Keyboard {
    fn new(path: &str) -> Result<Self, String> {
        let proxy = gio::DBusProxy::new_for_bus_sync::<gio::Cancellable>(
            gio::BusType::System,
            gio::DBusProxyFlags::NONE,
            None,
            DBUS_NAME,
            path,
            DBUS_KEYBOARD_IFACE,
            None,
        )
        .map_err(err_str)?;
        let properties_proxy = gio::DBusProxy::new_for_bus_sync::<gio::Cancellable>(
            gio::BusType::System,
            gio::DBusProxyFlags::NONE,
            None,
            DBUS_NAME,
            path,
            "org.freedesktop.DBus.Properties",
            None,
        )
        .map_err(err_str)?;
        Ok(Self {
            proxy,
            properties_proxy,
            brightness_set_cancellable: Cell::new(gio::Cancellable::new()),
            color_set_cancellable: Cell::new(gio::Cancellable::new()),
        })
    }

    fn prop<T: FromVariant>(&self, name: &'static str) -> Result<Option<T>, String> {
        Ok(self.proxy.get_cached_property(name).and_then(|v| v.get()))
    }

    fn set_prop<T: ToVariant>(
        &self,
        name: &'static str,
        value: T,
        cancellable: &Cell<gio::Cancellable>,
    ) -> Result<(), String> {
        let new_cancellable = gio::Cancellable::new();
        let args = (DBUS_KEYBOARD_IFACE, name, value.to_variant());
        self.properties_proxy.call(
            "Set",
            Some(&args.to_variant()),
            gio::DBusCallFlags::NONE,
            60000,
            Some(&new_cancellable),
            |_| {},
        );
        cancellable.replace(new_cancellable).cancel();
        Ok(())
    }
}

pub struct DaemonS76Power {
    boards: Vec<Keyboard>,
}

impl DaemonS76Power {
    fn board(&self, board: BoardId) -> Result<&Keyboard, String> {
        self.boards
            .get(board.0 as usize)
            .ok_or_else(|| "No board".to_string())
    }
}

impl DaemonS76Power {
    pub fn new() -> Result<Self, String> {
        let mut boards = Vec::new();

        let proxy = gio::DBusProxy::new_for_bus_sync::<gio::Cancellable>(
            gio::BusType::System,
            gio::DBusProxyFlags::NONE,
            None,
            DBUS_NAME,
            "/com/system76/PowerDaemon",
            "org.freedesktop.DBus.ObjectManager",
            None,
        )
        .map_err(err_str)?;
        let ret = proxy
            .call_sync::<gio::Cancellable>(
                "GetManagedObjects",
                None,
                gio::DBusCallFlags::NONE,
                60000,
                None,
            )
            .map_err(err_str)?;

        for i in ret.get_child_value(0).iter() {
            let path = i.get_child_value(0).get::<String>().unwrap();
            if path.starts_with("/com/system76/PowerDaemon/keyboard") {
                boards.push(Keyboard::new(&path)?);
            }
        }

        Ok(Self { boards })
    }
}

impl Daemon for DaemonS76Power {
    fn boards(&self) -> Result<Vec<BoardId>, String> {
        Ok((0..self.boards.len() as u128).map(BoardId).collect())
    }

    fn refresh(&self) -> Result<(), String> {
        // TODO
        Ok(())
    }

    fn model(&self, board: BoardId) -> Result<String, String> {
        Ok(self
            .board(board)?
            .prop::<String>("name")?
            .unwrap_or_else(|| "".to_string()))
    }

    fn keymap_get(
        &self,
        _board: BoardId,
        _layer: u8,
        _output: u8,
        _input: u8,
    ) -> Result<u16, String> {
        Err("Unimplemented".to_string())
    }

    fn keymap_set(
        &self,
        _board: BoardId,
        _layer: u8,
        _output: u8,
        _input: u8,
        _value: u16,
    ) -> Result<(), String> {
        Err("Unimplemented".to_string())
    }

    fn color(&self, board: BoardId) -> Result<Rgb, String> {
        let color = self.board(board)?.prop::<String>("color")?;
        Ok(color
            .and_then(|c| Rgb::parse(&c))
            .unwrap_or_else(|| Rgb::new(0, 0, 0)))
    }

    fn set_color(&self, board: BoardId, color: Rgb) -> Result<(), String> {
        let board = self.board(board)?;
        board.set_prop("color", color.to_string(), &board.color_set_cancellable)?;
        Ok(())
    }

    fn max_brightness(&self, board: BoardId) -> Result<i32, String> {
        Ok(self.board(board)?.prop("max_brightness")?.unwrap_or(100))
    }

    fn brightness(&self, board: BoardId) -> Result<i32, String> {
        Ok(self.board(board)?.prop("brightness")?.unwrap_or(0))
    }

    fn set_brightness(&self, board: BoardId, brightness: i32) -> Result<(), String> {
        let board = self.board(board)?;
        board.set_prop("brightness", brightness, &board.brightness_set_cancellable)?;
        Ok(())
    }

    fn exit(&self) -> Result<(), String> {
        Ok(())
    }
}
