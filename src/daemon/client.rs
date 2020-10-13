use std::{
    cell::RefCell,
    io::{
        BufRead,
        BufReader,
        Write,
    },
    process::{
        Child,
        ChildStdin,
        ChildStdout,
    },
};

use super::{
    err_str,
    Daemon,
    DaemonClientTrait,
    DaemonCommand,
    DaemonResponse,
};

pub struct DaemonClient {
    child: Child,
    read: RefCell<BufReader<ChildStdout>>,
    write: RefCell<ChildStdin>,
}

impl DaemonClient {
    pub fn new(child: Child, stdout: ChildStdout, stdin: ChildStdin) -> Self {
        Self {
            child,
            read: RefCell::new(BufReader::new(stdout)),
            write: RefCell::new(stdin),
        }
    }
}

impl DaemonClientTrait for DaemonClient {
    fn send_command(&self, command: DaemonCommand) -> Result<DaemonResponse, String> {
        let mut command_json = serde_json::to_string(&command).map_err(err_str)?;
        command_json.push('\n');
        self.write.borrow_mut().write_all(command_json.as_bytes()).map_err(err_str)?;

        let mut response_json = String::new();
        self.read.borrow_mut().read_line(&mut response_json).map_err(err_str)?;
        serde_json::from_str(&response_json).map_err(err_str)?
    }
}

impl Drop for DaemonClient {
    fn drop(&mut self) {
        let _ = self.exit();

        let status = self.child.wait().expect("Failed to wait for daemon");
        if !status.success() {
            panic!("Failed to run daemon with exit status {:?}", status);
        }
    }
}
