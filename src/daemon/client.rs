use serde::de::DeserializeOwned;
use std::{
    io::{
        BufRead,
        BufReader,
        Read,
        Write,
    },
};

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
}

impl<R: Read, W: Write> Drop for DaemonClient<R, W> {
    fn drop(&mut self) {
        let _ = self.command::<()>(DaemonCommand::Exit);
    }
}
