use std::{env, io};
use system76_keyboard_configurator::backend::{Daemon, DaemonClient, DaemonServer};

fn daemon_server() -> Result<DaemonServer<io::Stdin, io::Stdout>, String> {
    DaemonServer::new(io::stdin(), io::stdout())
}

#[cfg(target_os = "linux")]
fn with_daemon<F: Fn(Box<dyn Daemon>)>(f: F) {
    if unsafe { libc::geteuid() == 0 } {
        eprintln!("Already running as root");
        let server = daemon_server().expect("Failed to create server");
        f(Box::new(server));
        return;
    }

    f(Box::new(DaemonClient::new_pkexec()));
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

    with_daemon(|daemon| {
        println!("boards: {:?}", daemon.boards());
    });
}
