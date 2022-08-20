use std::{
    cell::{Cell, RefCell, RefMut},
    collections::HashMap,
    str,
    thread::sleep,
    time::Duration,
};
use uuid::Uuid;

use super::device_enumerator::{DeviceEnumerator, EcDevice, HidInfo};
use super::{err_str, BoardId, Daemon};
use crate::{Benchmark, Matrix, Nelson, NelsonKind};

static LAUNCH_IDS: &[(u16, u16, i32)] = &[
    // System76 launch_1
    (0x3384, 0x0001, 1),
    // System76 launch_lite_1
    (0x3384, 0x0005, 1),
    // System76 launch_2
    (0x3384, 0x0006, 1),
];
// System76 launch-nelson
static NELSON_ID: (u16, u16, i32) = (0x3384, 0x0002, 0);

pub struct DaemonServer {
    enumerator: RefCell<DeviceEnumerator>,
    boards: RefCell<HashMap<BoardId, (EcDevice, Option<HidInfo>)>>,
    board_ids: RefCell<Vec<BoardId>>,
    nelson: RefCell<Option<EcDevice>>,
}

impl DaemonServer {
    pub fn new() -> Self {
        let mut boards = HashMap::new();
        let mut board_ids = Vec::new();

        let mut enumerator = DeviceEnumerator::new();

        if let Some(ec_device) = enumerator.open_lpc() {
            info!("Adding LPC EC");
            let id = BoardId(Uuid::new_v4().as_u128());
            boards.insert(id, (ec_device, None));
            board_ids.push(id);
        }

        Self {
            enumerator: RefCell::new(enumerator),
            boards: RefCell::new(boards),
            board_ids: RefCell::new(board_ids),
            nelson: RefCell::new(None),
        }
    }

    fn have_device(&self, hid_info: &HidInfo) -> bool {
        self.boards
            .borrow()
            .values()
            .any(|(_, i)| i.as_ref() == Some(hid_info))
    }

    fn board(&self, board: BoardId) -> Result<RefMut<EcDevice>, String> {
        let mut boards = self.boards.borrow_mut();
        if boards.get_mut(&board).is_some() {
            Ok(RefMut::map(boards, |x| &mut x.get_mut(&board).unwrap().0))
        } else {
            Err("failed to find board".to_string())
        }
    }
}

impl Daemon for DaemonServer {
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

    fn version(&self, board: BoardId) -> Result<String, String> {
        let mut ec = self.board(board)?;
        let data_size = unsafe { ec.access().data_size() };
        let mut data = vec![0; data_size];
        let len = unsafe { ec.version(&mut data).map_err(err_str)? };
        let version = str::from_utf8(&data[..len]).map_err(err_str)?;
        Ok(version.to_string())
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

    fn benchmark(&self, _board: BoardId) -> Result<Benchmark, String> {
        Benchmark::new().map_err(err_str)
    }

    fn nelson(&self, board: BoardId, kind: NelsonKind) -> Result<Nelson, String> {
        if let Some(nelson) = &mut *self.nelson.borrow_mut() {
            let delay_ms = 300;
            info!("Nelson delay is {} ms", delay_ms);
            let delay = Duration::from_millis(delay_ms);

            // Check if Nelson is already closed
            if unsafe { nelson.led_get_value(0).map_err(err_str)?.0 > 0 } {
                info!("Open Nelson");
                unsafe { nelson.led_set_value(0, 0).map_err(err_str)? };

                info!("Sleep");
                sleep(delay);
            }

            info!("Close Nelson");
            unsafe { nelson.led_set_value(0, 1).map_err(err_str)? };

            info!("Sleep");
            sleep(delay);

            // Get pressed keys while nelson is closed
            let matrix = self.matrix_get(board)?;

            // Either missing or bouncing is set depending on test
            let (mut missing, bouncing) = match kind {
                NelsonKind::Normal => (matrix.clone(), Matrix::default()),
                NelsonKind::Bouncing => (Matrix::default(), matrix.clone()),
            };

            // Missing must be inverted, since missing keys are not pressed
            for row in 0..missing.rows() {
                for col in 0..missing.cols() {
                    let value = missing.get(row, col).unwrap_or(false);
                    missing.set(row, col, !value);
                }
            }

            info!("Open Nelson");
            unsafe { nelson.led_set_value(0, 0).map_err(err_str)? };

            info!("Sleep");
            sleep(delay);

            // Anything still pressed after nelson is opened is sticking
            let sticking = self.matrix_get(board)?;

            Ok(Nelson {
                missing,
                bouncing,
                sticking,
            })
        } else {
            Err(format!("failed to find Nelson"))
        }
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
        let index = if ec.is_hid() { 0xf0 } else { 0xff };
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
        let mut enumerator = &mut *self.enumerator.borrow_mut();

        // Remove USB boards that are no longer attached
        {
            let mut boards = self.boards.borrow_mut();
            let mut board_ids = self.board_ids.borrow_mut();

            boards.retain(|_, (ec, _)| !(ec.is_hid() && unsafe { ec.probe().is_err() }));
            board_ids.retain(|i| boards.contains_key(i));
        }

        for hid_info in enumerator.enumerate_hid().into_iter() {
            if LAUNCH_IDS.iter().any(|ids| hid_info.matches_ids(*ids)) {
                // Skip if device already open
                if self.have_device(&hid_info) {
                    continue;
                }

                match hid_info.open_device(&mut enumerator) {
                    Some(device) => {
                        info!("Adding USB HID EC at {:?}", hid_info.path());
                        let id = BoardId(Uuid::new_v4().as_u128());
                        self.boards
                            .borrow_mut()
                            .insert(id, (device, Some(hid_info)));
                        self.board_ids.borrow_mut().push(id);
                    }
                    None => {
                        // TODO errors
                        // "Failed to probe USB HID EC at {:?}: {:?}",
                        // "Failed to access USB HID EC at {:?}: {:?}",
                        // "Failed to open USB HID EC at {:?}: {:?}"
                        // error!("Failed to open USB HID EC at {:?}", info.path())
                    }
                }
            } else if hid_info.matches_ids(NELSON_ID) {
                if self.nelson.borrow().is_some() {
                    continue;
                }

                match hid_info.open_device(&mut enumerator) {
                    Some(device) => {
                        info!("Adding Nelson at {:?}", hid_info.path());
                        *self.nelson.borrow_mut() = Some(device);
                    }
                    None => {
                        // TODO errors
                        // "Failed to probe Nelson at {:?}: {:?}",
                        // "Failed to access Nelson at {:?}: {:?}",
                        // "Failed to open Nelson at {:?}: {:?}"
                        // error!("Failed to open Nelson at {:?}", info.path())
                    }
                }
            }
        }

        Ok(())
    }

    fn set_no_input(&self, board: BoardId, no_input: bool) -> Result<(), String> {
        let mut ec = self.board(board)?;
        unsafe { ec.set_no_input(no_input) }.map_err(err_str)
    }
}
