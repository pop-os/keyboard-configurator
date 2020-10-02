use std::{
    env,
    fs,
    io,
};
use system76_keyboard_configurator::{
    daemon::{
        Daemon,
        DaemonClient,
        DaemonServer,
    },
};

fn daemon_server() -> Result<DaemonServer<io::Stdin, io::Stdout>, String> {
    DaemonServer::new(io::stdin(), io::stdout())
}

#[cfg(target_os = "linux")]
fn with_daemon<F: Fn(Box<dyn Daemon>)>(f: F) {
    use std::{
        process::{
            Command,
            Stdio,
        },
    };

    if unsafe { libc::geteuid() == 0 } {
        eprintln!("Already running as root");
        let server = daemon_server().expect("Failed to create server");
        f(Box::new(server));
        return;
    }

    // Use pkexec to spawn daemon as superuser
    eprintln!("Not running as root, spawning daemon with pkexec");
    let mut command = Command::new("pkexec");

    // Use canonicalized command name
    let command_name = env::args().nth(0).expect("Failed to get command name");
    let command_path = fs::canonicalize(command_name).expect("Failed to canonicalize command");
    command.arg(command_path);
    command.arg("--daemon");

    // Pipe stdin and stdout
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());

    let mut child = command.spawn().expect("Failed to spawn daemon");

    let stdin = child.stdin.take().expect("Failed to get stdin of daemon");
    let stdout = child.stdout.take().expect("Failed to get stdout of daemon");

    f(Box::new(DaemonClient::new(stdout, stdin)));

    let status = child.wait().expect("Failed to wait for daemon");
    if ! status.success() {
        panic!("Failed to run daemon with exit status {:?}", status);
    }
}

#[cfg(not(target_os = "linux"))]
fn with_daemon<F: Fn(Box<dyn Daemon>)>(f: F) {
    let server = daemon_server().expect("Failed to create server");
    f(Box::new(server));
}

fn main() {
    for arg in env::args().skip(1) {
        if arg.as_str() == "--daemon" {
            let server = daemon_server().expect("Failed to create server");
            server.run().expect("Failed to run server");
            return;
        }
    }

    with_daemon(|mut daemon| {
        println!("boards: {:?}", daemon.boards());
    });
}
