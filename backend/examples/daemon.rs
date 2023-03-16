use std::env;
use system76_keyboard_configurator_backend::{run_daemon, Backend};

#[cfg(target_os = "linux")]
fn with_daemon<F: Fn(Backend)>(f: F) {
    if unsafe { libc::geteuid() == 0 } {
        eprintln!("Already running as root");
        let server = Backend::new(false).expect("Failed to create server");
        f(server);
        return;
    }

    f(Backend::new_pkexec(false).unwrap());
}

#[cfg(not(target_os = "linux"))]
fn with_daemon<F: Fn(Box<dyn Daemon>)>(f: F) {
    let server = Backend::new(false).expect("Failed to create server");
    f(server);
}

fn main() {
    for arg in env::args().skip(1) {
        if arg.as_str() == "--daemon" {
            run_daemon()
        }
    }

    with_daemon(|_daemon| {
        // println!("boards: {:?}", daemon.boards());
    });
}
