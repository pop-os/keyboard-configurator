use std::env;
use system76_keyboard_configurator_backend::{run_daemon, Backend, Events};

#[cfg(target_os = "linux")]
fn with_daemon<F: Fn(Backend, Events)>(f: F) {
    let (backend, events) = if unsafe { libc::geteuid() == 0 } {
        eprintln!("Already running as root");
        Backend::new().expect("Failed to create server")
    } else {
        Backend::new_pkexec().unwrap()
    };

    f(backend, events);
}

#[cfg(not(target_os = "linux"))]
fn with_daemon<F: Fn(Backend, Events)>(f: F) {
    let (backend, events) = Backend::new(false).expect("Failed to create server");
    f(backend, events);
}

fn main() {
    for arg in env::args().skip(1) {
        if arg.as_str() == "--daemon" {
            run_daemon()
        }
    }

    with_daemon(|_backend, _events| {
        // println!("boards: {:?}", daemon.boards());
    });
}
