use serde::{Deserialize, Serialize};
use std::io;

use crate::color::Rgb;

pub use self::client::DaemonClient;
mod client;

pub use self::server::DaemonServer;
mod server;

pub trait Daemon {
    fn boards(&mut self) -> Result<Vec<String>, String>;
    fn keymap_get(&mut self, board: usize, layer: u8, output: u8, input: u8) -> Result<u16, String>;
    fn keymap_set(&mut self, board: usize, layer: u8, output: u8, input: u8, value: u16) -> Result<(), String>;
    fn color(&mut self) -> Result<Rgb, String>;
    fn set_color(&mut self, color: Rgb) -> Result<(), String>;
    fn max_brightness(&mut self) -> Result<i32, String>;
    fn brightness(&mut self) -> Result<i32, String>;
    fn set_brightness(&mut self, brightness: i32) -> Result<(), String>;
}

fn err_str<E: std::fmt::Debug>(err: E) -> String {
    format!("{:?}", err)
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind")]
enum DaemonCommand {
    Boards,
    KeymapGet { board: usize, layer: u8, output: u8, input: u8 },
    KeymapSet { board: usize, layer: u8, output: u8, input: u8, value: u16 },
    MaxBrightness,
    Brightness,
    Color,
    SetBrightness { brightness: i32 },
    SetColor { color: Rgb },
    Exit,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
enum DaemonResult {
    Ok { ok: String },
    Err { err: String },
}

pub fn daemon_server() -> Result<DaemonServer<io::Stdin, io::Stdout>, String> {
    DaemonServer::new(io::stdin(), io::stdout())
}