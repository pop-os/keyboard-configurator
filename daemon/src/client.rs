use std::{
    cell::RefCell,
    env,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use crate::{err_str, Daemon, DaemonClientTrait, DaemonCommand, DaemonResponse};

pub struct DaemonClient {
    child: Child,
    read: RefCell<BufReader<ChildStdout>>,
    write: RefCell<ChildStdin>,
}

impl DaemonClient {
    pub fn new_pkexec() -> Self {
        // Use canonicalized command name
        let command_path = if cfg!(feature = "appimage") {
            PathBuf::from(env::var("APPIMAGE").expect("Failed to get executable path"))
        } else {
            env::current_exe().expect("Failed to get executable path")
        };

        let mut child = Command::new("pkexec")
            .arg(command_path)
            .arg("--daemon")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn daemon");

        let stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());

        // Check if daemon has started
        let mut line = String::new();
        if let Ok(count) = stdout.read_line(&mut line) {
            // pkexec terminated returning EOF
            if count == 0 {
                panic!("Failed to start daemon with pkexec");
            }
        }

        Self {
            child,
            read: RefCell::new(stdout),
            write: RefCell::new(stdin),
        }
    }
}

impl DaemonClientTrait for DaemonClient {
    fn send_command(&self, command: DaemonCommand) -> Result<DaemonResponse, String> {
        let mut command_json = serde_json::to_string(&command).map_err(err_str)?;
        command_json.push('\n');
        self.write
            .borrow_mut()
            .write_all(command_json.as_bytes())
            .map_err(err_str)?;

        let mut response_json = String::new();
        self.read
            .borrow_mut()
            .read_line(&mut response_json)
            .map_err(err_str)?;
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
