use serde::de::DeserializeOwned;
use std::{
    io::{
        BufRead,
        BufReader,
        Read,
        Write,
    },
};

use crate::color::Rgb;
use super::{
    err_str,
    Daemon,
    DaemonCommand,
    DaemonResult,
};

pub struct DaemonClient<R: Read, W: Write> {
    read: BufReader<R>,
    write: W,
}

impl<R: Read, W: Write> DaemonClient<R, W> {
    pub fn new(read: R, write: W) -> Self {
        Self {
            read: BufReader::new(read),
            write,
        }
    }

    fn command<T: DeserializeOwned>(&mut self, command: DaemonCommand) -> Result<T, String> {
        let mut command_json = serde_json::to_string(&command).map_err(err_str)?;
        command_json.push('\n');
        self.write.write_all(command_json.as_bytes()).map_err(err_str)?;

        let mut result_json = String::new();
        self.read.read_line(&mut result_json).map_err(err_str)?;
        let result = serde_json::from_str::<DaemonResult>(&result_json).map_err(err_str)?;
        match result {
            DaemonResult::Ok { ok } => {
                serde_json::from_reader(ok.as_bytes()).map_err(err_str)
            },
            DaemonResult::Err { err } => Err(err),
        }
    }
}

impl<R: Read, W: Write> Daemon for DaemonClient<R, W> {
    fn boards(&mut self) -> Result<Vec<String>, String> {
        self.command(DaemonCommand::Boards)
    }

    fn keymap_get(&mut self, board: usize, layer: u8, output: u8, input: u8) -> Result<u16, String> {
        self.command(DaemonCommand::KeymapGet { board, layer, output, input })
    }

    fn keymap_set(&mut self, board: usize, layer: u8, output: u8, input: u8, value: u16) -> Result<(), String> {
        self.command(DaemonCommand::KeymapSet { board, layer, output, input, value })
    }

    fn color(&mut self) -> Result<Rgb, String> {
        self.command(DaemonCommand::Color)
    }

    fn set_color(&mut self, color: Rgb) -> Result<(), String> {
        self.command(DaemonCommand::SetColor { color })
    }

    fn max_brightness(&mut self) -> Result<i32, String> {
        self.command(DaemonCommand::MaxBrightness)
    }

    fn brightness(&mut self) -> Result<i32, String> {
        self.command(DaemonCommand::Brightness)
    }

    fn set_brightness(&mut self, brightness: i32) -> Result<(), String> {
        self.command(DaemonCommand::SetBrightness { brightness })
    }
}

impl<R: Read, W: Write> Drop for DaemonClient<R, W> {
    fn drop(&mut self) {
        let _ = self.command::<()>(DaemonCommand::Exit);
    }
}
