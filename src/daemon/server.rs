#[cfg(target_os = "linux")]
use ectool::AccessLpcLinux;
use ectool::{Access, AccessHid, Ec};
use hidapi::HidApi;
use std::{
    cell::{Cell, RefCell, RefMut},
    io::{self, BufRead, BufReader, Read, Write},
    str,
    time::Duration,
};

use super::{err_str, Daemon, DaemonCommand};
use crate::color::Rgb;

pub struct DaemonServer<R: Read, W: Write> {
    running: Cell<bool>,
    read: BufReader<R>,
    write: W,
    hid: RefCell<Vec<Ec<Box<dyn Access>>>>,
    #[cfg(target_os = "linux")]
    lpc: RefCell<Vec<Ec<Box<dyn Access>>>>,
}

impl DaemonServer<io::Stdin, io::Stdout> {
    pub fn new_stdio() -> Result<Self, String> {
        Self::new(io::stdin(), io::stdout())
    }
}

impl<R: Read, W: Write> DaemonServer<R, W> {
    pub fn new(read: R, write: W) -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        let mut lpc = Vec::new();
        #[cfg(target_os = "linux")]
        match unsafe { AccessLpcLinux::new(Duration::new(1, 0)) } {
            Ok(access) => match unsafe { Ec::new(access) } {
                Ok(ec) => {
                    info!("Adding LPC EC");
                    lpc.push(ec.into_dyn());
                }
                Err(err) => {
                    error!("Failed to probe LPC EC: {:?}", err);
                }
            },
            Err(err) => {
                error!("Failed to access LPC EC: {:?}", err);
            }
        }

        let mut hid = Vec::new();
        //TODO: should we continue through HID errors?
        match HidApi::new() {
            Ok(api) => {
                for info in api.device_list() {
                    match (info.vendor_id(), info.product_id()) {
                        // System76 launch_1
                        (0x3384, 0x0001) => match info.interface_number() {
                            //TODO: better way to determine this
                            1 => match info.open_device(&api) {
                                Ok(device) => match AccessHid::new(device, 10, 100) {
                                    Ok(access) => match unsafe { Ec::new(access) } {
                                        Ok(ec) => {
                                            info!("Adding USB HID EC at {:?}", info.path());
                                            hid.push(ec.into_dyn());
                                        }
                                        Err(err) => {
                                            error!(
                                                "Failed to probe USB HID EC at {:?}: {:?}",
                                                info.path(),
                                                err
                                            );
                                        }
                                    },
                                    Err(err) => {
                                        error!(
                                            "Failed to access USB HID EC at {:?}: {:?}",
                                            info.path(),
                                            err
                                        );
                                    }
                                },
                                Err(err) => {
                                    error!(
                                        "Failed to open USB HID EC at {:?}: {:?}",
                                        info.path(),
                                        err
                                    );
                                }
                            },
                            _ => (),
                        },
                        _ => (),
                    }
                }
            }
            Err(err) => {
                error!("Failed to list USB HID ECs: {:?}", err);
            }
        }

        Ok(Self {
            running: Cell::new(true),
            read: BufReader::new(read),
            write,
            hid: RefCell::new(hid),
            #[cfg(target_os = "linux")]
            lpc: RefCell::new(lpc),
        })
    }

    pub fn run(mut self) -> io::Result<()> {
        println!("Daemon started");

        while self.running.get() {
            let mut command_json = String::new();
            self.read.read_line(&mut command_json)?;

            let command = serde_json::from_str::<DaemonCommand>(&command_json)
                .expect("failed to deserialize command");
            let response = self.dispatch_command_to_method(command);

            //TODO: what to do if we fail to serialize result?
            let mut result_json =
                serde_json::to_string(&response).expect("failed to serialize result");
            result_json.push('\n');
            self.write.write_all(result_json.as_bytes())?;
        }

        Ok(())
    }

    fn board(&self, mut board: usize) -> Result<RefMut<Ec<Box<dyn Access>>>, String> {
        #[cfg(target_os = "linux")]
        {
            let mut lpc = self.lpc.borrow_mut();
            if lpc.get_mut(board).is_some() {
                return Ok(RefMut::map(lpc, |x| x.get_mut(board).unwrap()));
            }
            board -= lpc.len();
        }

        let mut hid = self.hid.borrow_mut();
        if hid.get_mut(board).is_some() {
            return Ok(RefMut::map(hid, |x| x.get_mut(board).unwrap()));
        }

        Err("failed to find board".to_string())
    }
}

impl<R: Read, W: Write> Daemon for DaemonServer<R, W> {
    fn boards(&self) -> Result<Vec<String>, String> {
        let mut boards = Vec::new();

        #[cfg(target_os = "linux")]
        for ec in self.lpc.borrow_mut().iter_mut() {
            let data_size = unsafe { ec.access().data_size() };
            let mut data = vec![0; data_size];
            let len = unsafe { ec.board(&mut data).map_err(err_str)? };
            let board = str::from_utf8(&data[..len]).map_err(err_str)?;
            boards.push(board.to_string());
        }

        for ec in self.hid.borrow_mut().iter_mut() {
            let data_size = unsafe { ec.access().data_size() };
            let mut data = vec![0; data_size];
            let len = unsafe { ec.board(&mut data).map_err(err_str)? };
            let board = str::from_utf8(&data[..len]).map_err(err_str)?;
            boards.push(board.to_string());
        }

        Ok(boards)
    }

    fn keymap_get(&self, board: usize, layer: u8, output: u8, input: u8) -> Result<u16, String> {
        let mut ec = self.board(board)?;
        unsafe { ec.keymap_get(layer, output, input).map_err(err_str) }
    }

    fn keymap_set(
        &self,
        board: usize,
        layer: u8,
        output: u8,
        input: u8,
        value: u16,
    ) -> Result<(), String> {
        let mut ec = self.board(board)?;
        unsafe { ec.keymap_set(layer, output, input, value).map_err(err_str) }
    }

    fn color(&self, board: usize) -> Result<Rgb, String> {
        let mut ec = self.board(board)?;
        unsafe {
            ec.led_get_color(0xFF)
                .map(|x| Rgb::new(x.0, x.1, x.2))
                .map_err(err_str)
        }
    }

    fn set_color(&self, board: usize, color: Rgb) -> Result<(), String> {
        let mut ec = self.board(board)?;
        unsafe {
            ec.led_set_color(0xFF, color.r, color.g, color.b)
                .map_err(err_str)
        }
    }

    fn max_brightness(&self, board: usize) -> Result<i32, String> {
        let mut ec = self.board(board)?;
        unsafe { ec.led_get_value(0xFF).map(|x| x.1 as i32).map_err(err_str) }
    }

    fn brightness(&self, board: usize) -> Result<i32, String> {
        let mut ec = self.board(board)?;
        unsafe { ec.led_get_value(0xFF).map(|x| x.0 as i32).map_err(err_str) }
    }

    fn set_brightness(&self, board: usize, brightness: i32) -> Result<(), String> {
        let mut ec = self.board(board)?;
        unsafe { ec.led_set_value(0xFF, brightness as u8).map_err(err_str) }
    }

    fn exit(&self) -> Result<(), String> {
        self.running.set(false);
        Ok(())
    }
}
