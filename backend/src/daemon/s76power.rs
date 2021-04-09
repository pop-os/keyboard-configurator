// Note: Linux only
// Need to watch properties of each object?
// TODO: Hotplug detection support

use std::iter::Iterator;
use zbus::{dbus_proxy, fdo::ObjectManagerProxy, Connection};

use super::{err_str, BoardId, Daemon, Matrix};
use crate::Rgb;

const DBUS_NAME: &str = "com.system76.PowerDaemon";

#[dbus_proxy(interface = "com.system76.PowerDaemon.Keyboard")]
trait Keyboard {
    #[dbus_proxy(property)]
    fn brightness(&self) -> zbus::Result<i32>;
    #[dbus_proxy(property)]
    fn set_brightness(&self, value: i32) -> zbus::Result<()>;

    #[dbus_proxy(property)]
    fn color(&self) -> zbus::Result<String>;
    #[dbus_proxy(property)]
    fn set_color(&self, value: &str) -> zbus::Result<()>;

    #[dbus_proxy(property)]
    fn max_brightness(&self) -> zbus::Result<i32>;

    #[dbus_proxy(property)]
    fn name(&self) -> zbus::Result<String>;
}

struct Keyboard {
    proxy: KeyboardProxy<'static>,
}

impl Keyboard {
    fn new(path: &str) -> Result<Self, String> {
        let connection = Connection::new_system().map_err(err_str)?;
        let proxy =
            KeyboardProxy::new_for_owned(connection, DBUS_NAME.to_string(), path.to_string())
                .map_err(err_str)?;
        Ok(Self { proxy })
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

        let connection = Connection::new_system().map_err(err_str)?;
        let proxy =
            ObjectManagerProxy::new_for(&connection, DBUS_NAME, "/com/system76/PowerDaemon")
                .map_err(err_str)?;
        let objects = proxy.get_managed_objects().map_err(err_str)?;

        for path in objects.keys() {
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

    fn model(&self, board: BoardId) -> Result<String, String> {
        Ok(self
            .board(board)?
            .proxy
            .name()
            .unwrap_or_else(|_| "".to_string()))
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

    fn matrix_get(&self, _board: BoardId) -> Result<Matrix, String> {
        Err("Unimplemented".to_string())
    }

    fn color(&self, board: BoardId, index: u8) -> Result<(u8, u8, u8), String> {
        if index != 0xFF {
            return Err(format!("Can't set color index {}", index));
        }
        let color = self.board(board)?.proxy.color().map_err(err_str)?;
        Ok(Rgb::parse(&color).map_or((0, 0, 0), |rgb| (rgb.r, rgb.g, rgb.b)))
    }

    fn set_color(&self, board: BoardId, index: u8, color: (u8, u8, u8)) -> Result<(), String> {
        if index != 0xFF {
            return Err(format!("Can't set color index {}", index));
        }
        self.board(board)?
            .proxy
            .set_color(&Rgb::new(color.0, color.1, color.2).to_string())
            .map_err(err_str)
    }

    fn max_brightness(&self, board: BoardId) -> Result<i32, String> {
        Ok(self.board(board)?.proxy.max_brightness().map_err(err_str)?)
    }

    fn brightness(&self, board: BoardId, index: u8) -> Result<i32, String> {
        if index != 0xFF {
            return Err(format!("Can't set brightness index {}", index));
        }
        Ok(self.board(board)?.proxy.brightness().map_err(err_str)?)
    }

    fn set_brightness(&self, board: BoardId, index: u8, brightness: i32) -> Result<(), String> {
        if index != 0xFF {
            return Err(format!("Can't set brightness index {}", index));
        }
        self.board(board)?
            .proxy
            .set_brightness(brightness)
            .map_err(err_str)
    }

    fn mode(&self, _board: BoardId, _layer: u8) -> Result<(u8, u8), String> {
        Err("Unimplemented".to_string())
    }

    fn set_mode(&self, _board: BoardId, _layer: u8, _mode: u8, _speed: u8) -> Result<(), String> {
        Err("Unimplemented".to_string())
    }

    fn led_save(&self, _board: BoardId) -> Result<(), String> {
        Err("Unimplemented".to_string())
    }

    fn refresh(&self) -> Result<(), String> {
        Ok(())
    }

    fn exit(&self) -> Result<(), String> {
        Ok(())
    }
}
