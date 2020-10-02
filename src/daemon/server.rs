use ectool::{Access, AccessHid, Ec};
#[cfg(target_os = "linux")]
use ectool::AccessLpcLinux;
use hidapi::HidApi;
use serde::Serialize;
use std::{
    io::{
        self,
        BufRead,
        BufReader,
        Read,
        Write,
    },
    str,
    time::Duration,
};

use super::{
    err_str,
    Daemon,
    DaemonCommand,
    DaemonResult,
};

pub struct DaemonServer<R: Read, W: Write> {
    running: bool,
    read: BufReader<R>,
    write: W,
    hid: Vec<Ec<AccessHid>>,
    #[cfg(target_os = "linux")]
    lpc: Vec<Ec<AccessLpcLinux>>,
}

impl<R: Read, W: Write> DaemonServer<R, W> {
    pub fn new(read: R, write: W) -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        let mut lpc = Vec::new();
        #[cfg(target_os = "linux")]
        match unsafe { AccessLpcLinux::new(Duration::new(1, 0)) } {
            Ok(access) => match unsafe { Ec::new(access) } {
                Ok(ec) => {
                    eprintln!("Adding LPC EC");
                    lpc.push(ec);
                },
                Err(err) => {
                    eprintln!("Failed to probe LPC EC: {:?}", err);
                },
            },
            Err(err) => {
                eprintln!("Failed to access LPC EC: {:?}", err);
            },
        }

        let mut hid = Vec::new();
        //TODO: should we continue through HID errors?
        match HidApi::new() {
            Ok(api) => for info in api.device_list() {
                match (info.vendor_id(), info.product_id()) {
                    (0x1776, 0x1776) => match info.interface_number() {
                        //TODO: better way to determine this
                        1 => match info.open_device(&api) {
                            Ok(device) => {
                                match AccessHid::new(device, 10, 100) {
                                    Ok(access) => match unsafe { Ec::new(access) } {
                                        Ok(ec) => {
                                            eprintln!("Adding USB HID EC at {:?}", info.path());
                                            hid.push(ec);
                                        },
                                        Err(err) => {
                                            eprintln!("Failed to probe USB HID EC at {:?}: {:?}", info.path(), err);
                                        }
                                    },
                                    Err(err) => {
                                        eprintln!("Failed to access USB HID EC at {:?}: {:?}", info.path(), err);
                                    },
                                }
                            },
                            Err(err) => {
                                eprintln!("Failed to open USB HID EC at {:?}: {:?}", info.path(), err);
                            },
                        },
                        _ => (),
                    },
                    _ => (),
                }
            },
            Err(err) => {
                eprintln!("Failed to list USB HID ECs: {:?}", err);
            }
        }

        Ok(Self {
            running: true,
            read: BufReader::new(read),
            write,
            hid,
            #[cfg(target_os = "linux")]
            lpc,
        })
    }

    fn command(&mut self, command_json: &str) -> Result<String, String> {
        fn json<T: Serialize>(value: T) -> Result<String, String> {
            serde_json::to_string(&value).map_err(err_str)
        }

        let command = serde_json::from_str::<DaemonCommand>(&command_json).map_err(err_str)?;
        match command {
            DaemonCommand::Boards => {
                json(self.boards()?)
            },
            DaemonCommand::KeymapGet { board, layer, output, input } => {
                json(self.keymap_get(board, layer, output, input)?)
            },
            DaemonCommand::KeymapSet { board, layer, output, input, value } => {
                json(self.keymap_set(board, layer, output, input, value)?)
            },
            DaemonCommand::Exit => {
                self.running = false;
                json(())
            },
        }
    }

    pub fn run(mut self) -> io::Result<()> {
        while self.running {
            let mut command_json = String::new();
            self.read.read_line(&mut command_json)?;

            let result = match self.command(&command_json) {
                Ok(ok) => DaemonResult::Ok { ok },
                Err(err) => DaemonResult::Err { err },
            };

            //TODO: what to do if we fail to serialize result?
            let mut result_json = serde_json::to_string(&result).expect("failed to serialize result");
            result_json.push('\n');
            self.write.write_all(result_json.as_bytes())?;
        }

        Ok(())
    }
}

impl<R: Read, W: Write> Daemon for DaemonServer<R, W> {
    fn boards(&mut self) -> Result<Vec<String>, String> {
        let mut boards = Vec::new();

        #[cfg(target_os = "linux")]
        for ec in self.lpc.iter_mut() {
            let data_size = unsafe { ec.access().data_size() };
            let mut data = vec![0; data_size];
            let len = unsafe { ec.board(&mut data).map_err(err_str)? };
            let board = str::from_utf8(&data[..len]).map_err(err_str)?;
            boards.push(board.to_string());
        }

        for ec in self.hid.iter_mut() {
            let data_size = unsafe { ec.access().data_size() };
            let mut data = vec![0; data_size];
            let len = unsafe { ec.board(&mut data).map_err(err_str)? };
            let board = str::from_utf8(&data[..len]).map_err(err_str)?;
            boards.push(board.to_string());
        }

        Ok(boards)
    }

    fn keymap_get(&mut self, mut board: usize, layer: u8, output: u8, input: u8) -> Result<u16, String> {
        #[cfg(target_os = "linux")]
        {
            if board < self.lpc.len() {
                return unsafe {
                    self.lpc[board].keymap_get(layer, output, input).map_err(err_str)
                };
            }
            board -= self.lpc.len();
        }

        if let Some(ref mut ec) = self.hid.get_mut(board) {
            unsafe {
                ec.keymap_get(layer, output, input).map_err(err_str)
            }
        } else {
            Err("failed to find board".to_string())
        }
    }

    fn keymap_set(&mut self, mut board: usize, layer: u8, output: u8, input: u8, value: u16) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            if board < self.lpc.len() {
                return unsafe {
                    self.lpc[board].keymap_set(layer, output, input, value).map_err(err_str)
                };
            }
            board -= self.lpc.len();
        }

        if let Some(ref mut ec) = self.hid.get_mut(board) {
            unsafe {
                ec.keymap_set(layer, output, input, value).map_err(err_str)
            }
        } else {
            Err("failed to find board".to_string())
        }
    }
}
