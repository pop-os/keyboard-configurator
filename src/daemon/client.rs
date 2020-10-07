use std::{
    cell::RefCell,
    io::{
        BufRead,
        BufReader,
        Read,
        Write,
    },
};

use super::{
    err_str,
    DaemonClientTrait,
    DaemonCommand,
    DaemonResponse,
};

pub struct DaemonClient<R: Read, W: Write> {
    read: RefCell<BufReader<R>>,
    write: RefCell<W>,
}

impl<R: Read, W: Write> DaemonClient<R, W> {
    pub fn new(read: R, write: W) -> Self {
        Self {
            read: RefCell::new(BufReader::new(read)),
            write: RefCell::new(write),
        }
    }
}

impl<R: std::io::Read, W: std::io::Write> DaemonClientTrait for DaemonClient<R, W> {
    fn send_command(&self, command: DaemonCommand) -> Result<DaemonResponse, String> {
        let mut command_json = serde_json::to_string(&command).map_err(err_str)?;
        command_json.push('\n');
        self.write.borrow_mut().write_all(command_json.as_bytes()).map_err(err_str)?;

        let mut response_json = String::new();
        self.read.borrow_mut().read_line(&mut response_json).map_err(err_str)?;
        serde_json::from_str(&response_json).map_err(err_str)?
    }
}

impl<R: Read, W: Write> Drop for DaemonClient<R, W> {
    fn drop(&mut self) {
        let _ = self.send_command(DaemonCommand::exit{});
    }
}
