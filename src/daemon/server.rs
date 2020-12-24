use ectool::{Access, AccessHid, Ec};
#[cfg(target_os = "linux")]
use ectool::AccessLpcLinux;
use hidapi::HidApi;
use std::{
    cell::{
        Cell,
        RefCell,
    },
    fs,
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

use crate::color::Rgb;
use super::{
    err_str,
    Daemon,
    DaemonCommand,
};

pub struct DaemonServer<R: Read, W: Write> {
    running: Cell<bool>,
    read: BufReader<R>,
    write: W,
    hid: RefCell<Vec<Ec<AccessHid>>>,
    #[cfg(target_os = "linux")]
    lpc: RefCell<Vec<Ec<AccessLpcLinux>>>,
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
                    // System76 launch_1
                    (0x3384, 0x0001) => match info.interface_number() {
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
            running: Cell::new(true),
            read: BufReader::new(read),
            write,
            hid: RefCell::new(hid),
            #[cfg(target_os = "linux")]
            lpc: RefCell::new(lpc),
        })
    }

    pub fn run(mut self) -> io::Result<()> {
        while self.running.get() {
            let mut command_json = String::new();
            self.read.read_line(&mut command_json)?;

            let command = serde_json::from_str::<DaemonCommand>(&command_json).expect("failed to deserialize command");
            let response = self.dispatch_command_to_method(command);

            //TODO: what to do if we fail to serialize result?
            let mut result_json = serde_json::to_string(&response).expect("failed to serialize result");
            result_json.push('\n');
            self.write.write_all(result_json.as_bytes())?;
        }

        Ok(())
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

    fn keymap_get(&self, mut board: usize, layer: u8, output: u8, input: u8) -> Result<u16, String> {
        #[cfg(target_os = "linux")]
        {
            let mut lpc = self.lpc.borrow_mut();
            if board < lpc.len() {
                return unsafe {
                    lpc[board].keymap_get(layer, output, input).map_err(err_str)
                };
            }
            board -= lpc.len();
        }

        if let Some(ref mut ec) = self.hid.borrow_mut().get_mut(board) {
            unsafe {
                ec.keymap_get(layer, output, input).map_err(err_str)
            }
        } else {
            Err("failed to find board".to_string())
        }
    }

    fn keymap_set(&self, mut board: usize, layer: u8, output: u8, input: u8, value: u16) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            let mut lpc = self.lpc.borrow_mut();
            if board < lpc.len() {
                return unsafe {
                    lpc[board].keymap_set(layer, output, input, value).map_err(err_str)
                };
            }
            board -= lpc.len();
        }

        if let Some(ref mut ec) = self.hid.borrow_mut().get_mut(board) {
            unsafe {
                ec.keymap_set(layer, output, input, value).map_err(err_str)
            }
        } else {
            Err("failed to find board".to_string())
        }
    }

    fn color(&self, board: usize) -> Result<Rgb, String> {
        let path = "/sys/class/leds/system76_acpi::kbd_backlight/color";
        match fs::read_to_string(&path) {
            Ok(string) => {
                let trimmed = string.trim();
                match Rgb::parse(trimmed) {
                    Some(color) => Ok(color),
                    None => Err(format!("Failed to parse keyboard color '{}'", trimmed))
                }
            },
            Err(err) => Err(format!("Failed to read keyboard color: {}", err))
        }
    }

    fn set_color(&self, board: usize, color: Rgb) -> Result<(), String> {
        let path = "/sys/class/leds/system76_acpi::kbd_backlight/color";
        match fs::write(path, &color.to_string()) {
            Ok(()) => Ok(()),
            Err(err) => Err(format!("Failed to write keyboard color: {}", err))
        }

    }

    fn max_brightness(&self, board: usize) -> Result<i32, String> {
        let path = "/sys/class/leds/system76_acpi::kbd_backlight/max_brightness";
        match fs::read_to_string(&path) {
            Ok(string) => {
                let trimmed = string.trim();
                match trimmed.parse::<i32>() {
                    Ok(value) => Ok(value),
                    Err(err) => {
                        Err(format!("Failed to parse keyboard max brightness '{}': {}", trimmed, err))
                    }
                }
            },
            Err(err) => {
                Err(format!("Failed to read keyboard max brightness: {}", err))
            }
        }
    }

    fn brightness(&self, board: usize) -> Result<i32, String> {
        let path = "/sys/class/leds/system76_acpi::kbd_backlight/brightness";
        match fs::read_to_string(&path) {
            Ok(string) => {
                let trimmed = string.trim();
                match trimmed.parse::<i32>() {
                    Ok(value) => Ok(value),
                    Err(err) => {
                        Err(format!("Failed to parse keyboard brightness '{}': {}", trimmed, err))
                    }
                }
            },
            Err(err) => {
                Err(format!("Failed to read keyboard brightness: {}", err))
            }
        }
    }

    fn set_brightness(&self, board: usize, brightness: i32) -> Result<(), String> {
        let path = "/sys/class/leds/system76_acpi::kbd_backlight/brightness";
        match fs::write(path, &format!("{}", brightness)) {
            Ok(()) => Ok(()),
            Err(err) => Err(format!("Failed to write keyboard brightness: {}", err))
        }
    }

    fn exit(&self) -> Result<(), String> {
        self.running.set(false);
        Ok(())
    }
}
