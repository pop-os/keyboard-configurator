#[cfg(target_os = "linux")]
use ectool::AccessLpcLinux;
use ectool::{Access, AccessHid, Ec};
use hidapi::{DeviceInfo, HidApi};
use std::{
    cell::{Cell, RefCell, RefMut},
    collections::HashMap,
    io::{self, BufRead, BufReader, Read, Write},
    str,
    time::Duration,
};
use uuid::Uuid;

use super::{err_str, BoardId, Daemon, DaemonCommand, Nelson};
use crate::Matrix;

pub struct DaemonServer<R: Read + Send + 'static, W: Write + Send + 'static> {
    hidapi: RefCell<Option<HidApi>>,
    running: Cell<bool>,
    read: BufReader<R>,
    write: W,
    boards: RefCell<HashMap<BoardId, (Ec<Box<dyn Access>>, Option<DeviceInfo>)>>,
    board_ids: RefCell<Vec<BoardId>>,
    nelson: RefCell<Option<Ec<AccessHid>>>,
}

impl DaemonServer<io::Stdin, io::Stdout> {
    pub fn new_stdio() -> Result<Self, String> {
        Self::new(io::stdin(), io::stdout())
    }
}

impl<R: Read + Send + 'static, W: Write + Send + 'static> DaemonServer<R, W> {
    pub fn new(read: R, write: W) -> Result<Self, String> {
        let mut boards = HashMap::new();
        let mut board_ids = Vec::new();

        #[cfg(target_os = "linux")]
        match unsafe { AccessLpcLinux::new(Duration::new(1, 0)) } {
            Ok(access) => match unsafe { Ec::new(access) } {
                Ok(ec) => {
                    info!("Adding LPC EC");
                    let id = BoardId(Uuid::new_v4().as_u128());
                    boards.insert(id, (ec.into_dyn(), None));
                    board_ids.push(id);
                }
                Err(err) => {
                    error!("Failed to probe LPC EC: {:?}", err);
                }
            },
            Err(err) => {
                error!("Failed to access LPC EC: {:?}", err);
            }
        }

        //TODO: should we continue through HID errors?
        let hidapi = match HidApi::new() {
            Ok(api) => Some(api),
            Err(err) => {
                error!("Failed to list USB HID ECs: {:?}", err);
                None
            }
        };

        Ok(Self {
            hidapi: RefCell::new(hidapi),
            running: Cell::new(true),
            read: BufReader::new(read),
            write,
            boards: RefCell::new(boards),
            board_ids: RefCell::new(board_ids),
            nelson: RefCell::new(None),
        })
    }

    fn have_device(&self, info: &DeviceInfo) -> bool {
        for (_, i) in self.boards.borrow().values() {
            if let Some(i) = i {
                if (i.vendor_id(), i.product_id(), i.path())
                    == (info.vendor_id(), info.product_id(), info.path())
                {
                    return true;
                }
            }
        }
        false
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

    fn board(&self, board: BoardId) -> Result<RefMut<Ec<Box<dyn Access>>>, String> {
        let mut boards = self.boards.borrow_mut();
        if boards.get_mut(&board).is_some() {
            Ok(RefMut::map(boards, |x| &mut x.get_mut(&board).unwrap().0))
        } else {
            Err("failed to find board".to_string())
        }
    }
}

impl<R: Read + Send + 'static, W: Write + Send + 'static> Daemon for DaemonServer<R, W> {
    fn boards(&self) -> Result<Vec<BoardId>, String> {
        Ok(self.board_ids.borrow().clone())
    }

    fn model(&self, board: BoardId) -> Result<String, String> {
        let mut ec = self.board(board)?;
        let data_size = unsafe { ec.access().data_size() };
        let mut data = vec![0; data_size];
        let len = unsafe { ec.board(&mut data).map_err(err_str)? };
        let board = str::from_utf8(&data[..len]).map_err(err_str)?;
        Ok(board.to_string())
    }

    fn keymap_get(&self, board: BoardId, layer: u8, output: u8, input: u8) -> Result<u16, String> {
        let mut ec = self.board(board)?;
        unsafe { ec.keymap_get(layer, output, input).map_err(err_str) }
    }

    fn keymap_set(
        &self,
        board: BoardId,
        layer: u8,
        output: u8,
        input: u8,
        value: u16,
    ) -> Result<(), String> {
        let mut ec = self.board(board)?;
        unsafe { ec.keymap_set(layer, output, input, value).map_err(err_str) }
    }

    fn matrix_get(&self, board: BoardId) -> Result<Matrix, String> {
        let mut ec = self.board(board)?;

        let data_size = unsafe { ec.access().data_size() };
        let mut data = vec![0; data_size];
        unsafe { ec.matrix_get(&mut data).map_err(err_str)? };

        let rows = data.remove(0) as usize;
        let cols = data.remove(0) as usize;
        Ok(Matrix::new(rows, cols, data.into_boxed_slice()))
    }

    fn nelson(&self, board: BoardId) -> Result<Nelson, String> {
        Err("Unimplemented".to_string())
    }

    fn color(&self, board: BoardId, index: u8) -> Result<(u8, u8, u8), String> {
        let mut ec = self.board(board)?;
        unsafe { ec.led_get_color(index) }.map_err(err_str)
    }

    fn set_color(&self, board: BoardId, index: u8, color: (u8, u8, u8)) -> Result<(), String> {
        let mut ec = self.board(board)?;
        unsafe {
            ec.led_set_color(index, color.0, color.1, color.2)
                .map_err(err_str)
        }
    }

    fn max_brightness(&self, board: BoardId) -> Result<i32, String> {
        let mut ec = self.board(board)?;
        let index = if unsafe { ec.access().is::<AccessHid>() } {
            0xf0
        } else {
            0xff
        };
        unsafe { ec.led_get_value(index) }
            .map(|x| x.1 as i32)
            .map_err(err_str)
    }

    fn brightness(&self, board: BoardId, index: u8) -> Result<i32, String> {
        let mut ec = self.board(board)?;
        unsafe { ec.led_get_value(index).map(|x| x.0 as i32).map_err(err_str) }
    }

    fn set_brightness(&self, board: BoardId, index: u8, brightness: i32) -> Result<(), String> {
        let mut ec = self.board(board)?;
        unsafe { ec.led_set_value(index, brightness as u8).map_err(err_str) }
    }

    fn mode(&self, board: BoardId, layer: u8) -> Result<(u8, u8), String> {
        let mut ec = self.board(board)?;
        unsafe { ec.led_get_mode(layer).map_err(err_str) }
    }

    fn set_mode(&self, board: BoardId, layer: u8, mode: u8, speed: u8) -> Result<(), String> {
        let mut ec = self.board(board)?;
        unsafe { ec.led_set_mode(layer, mode, speed).map_err(err_str) }
    }

    fn led_save(&self, board: BoardId) -> Result<(), String> {
        let mut ec = self.board(board)?;
        unsafe { ec.led_save().map_err(err_str) }
    }

    fn refresh(&self) -> Result<(), String> {
        if let Some(api) = &mut *self.hidapi.borrow_mut() {
            // Remove USB boards that are no longer attached
            {
                let mut boards = self.boards.borrow_mut();
                let mut board_ids = self.board_ids.borrow_mut();

                boards.retain(|_, (ec, _)| unsafe {
                    !(ec.access().is::<AccessHid>() && ec.probe().is_err())
                });
                board_ids.retain(|i| boards.contains_key(i));
            }

            if let Err(err) = api.refresh_devices() {
                error!("Failed to refresh hidapi devices: {}", err);
            }

            for info in api.device_list() {
                match (info.vendor_id(), info.product_id(), info.interface_number()) {
                    // System76 launch_1
                    //TODO: better way to determine this
                    (0x3384, 0x0001, 1) => {
                        // Skip if device already open
                        if self.have_device(&info) {
                            continue;
                        }

                        match info.open_device(&api) {
                            Ok(device) => match AccessHid::new(device, 10, 1000) {
                                Ok(access) => match unsafe { Ec::new(access) } {
                                    Ok(ec) => {
                                        info!("Adding USB HID EC at {:?}", info.path());
                                        let id = BoardId(Uuid::new_v4().as_u128());
                                        self.boards
                                            .borrow_mut()
                                            .insert(id, (ec.into_dyn(), Some(info.clone())));
                                        self.board_ids.borrow_mut().push(id);
                                    }
                                    Err(err) => error!(
                                        "Failed to probe USB HID EC at {:?}: {:?}",
                                        info.path(),
                                        err
                                    ),
                                },
                                Err(err) => error!(
                                    "Failed to access USB HID EC at {:?}: {:?}",
                                    info.path(),
                                    err
                                ),
                            },
                            Err(err) => {
                                error!("Failed to open USB HID EC at {:?}: {:?}", info.path(), err)
                            }
                        }
                    },
                    // System76 launch-nelson
                    (0x3384, 0x0002, 0) => {
                        if self.nelson.borrow().is_some() {
                            continue;
                        }

                        match info.open_device(&api) {
                            Ok(device) => match AccessHid::new(device, 10, 1000) {
                                Ok(access) => match unsafe { Ec::new(access) } {
                                    Ok(ec) => {
                                        info!("Adding Nelson at {:?}", info.path());
                                        *self.nelson.borrow_mut() = Some(ec);
                                    }
                                    Err(err) => error!(
                                        "Failed to probe Nelson at {:?}: {:?}",
                                        info.path(),
                                        err
                                    ),
                                },
                                Err(err) => error!(
                                    "Failed to access Nelson at {:?}: {:?}",
                                    info.path(),
                                    err
                                ),
                            },
                            Err(err) => {
                                error!("Failed to open Nelson at {:?}: {:?}", info.path(), err)
                            }
                        }
                    },
                    _ => (),
                }
            }
        }

        Ok(())
    }

    fn exit(&self) -> Result<(), String> {
        self.running.set(false);
        Ok(())
    }
}
