use cascade::cascade;
use gio::prelude::*;
use gtk::prelude::*;
use std::rc::Rc;

use crate::daemon::{Daemon, DaemonClient, daemon_server};

mod key;
mod keyboard;
mod page;
mod picker;
mod rect;

use keyboard::Keyboard;

//TODO: allow multiple keyboards
fn main_keyboard(app: &gtk::Application, keyboard: Rc<Keyboard>) {
    let vbox = cascade! {
        gtk::Box::new(gtk::Orientation::Vertical, 32);
        ..add(&keyboard.clone().gtk());
        ..add(&keyboard.clone().picker());
    };

    let scrolled_window = cascade! {
        gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
        ..add(&vbox);
    };

    let window = cascade! {
        gtk::ApplicationWindow::new(app);
        ..set_title("Keyboard Layout");
        ..set_border_width(10);
        ..set_position(gtk::WindowPosition::Center);
        ..set_default_size(1024, 768);
        ..add(&scrolled_window);
    };

    window.set_focus::<gtk::Widget>(None);
    window.show_all();

    window.connect_destroy(|_| {
        eprintln!("Window close");
        gtk::main_quit();
    });
}

fn main_app(app: &gtk::Application, daemon: Rc<dyn Daemon>) {
    let boards = daemon.boards().expect("Failed to load boards");
    let i = 0;
    if let Some(board) = boards.get(i) {
        if let Some(keyboard) = Keyboard::new_board(board, Some(daemon), i) {
            main_keyboard(app, keyboard);
            return;
        } else {
            eprintln!("Failed to locate layout for '{}'", board);
        }
    }

    eprintln!("Failed to locate any keyboards, showing demo");
    let keyboard = Keyboard::new_board("system76/launch_alpha_2", None, 0)
        .expect("Failed to load demo layout");
    main_keyboard(app, keyboard);
}

#[cfg(target_os = "linux")]
fn with_daemon<F: Fn(Rc<dyn Daemon>)>(f: F) {
    use std::{
        process::{
            Command,
            Stdio,
        },
    };

    if unsafe { libc::geteuid() == 0 } {
        eprintln!("Already running as root");
        let server = daemon_server().expect("Failed to create server");
        f(Rc::new(server));
        return;
    }

    // Use pkexec to spawn daemon as superuser
    eprintln!("Not running as root, spawning daemon with pkexec");
    let mut command = Command::new("pkexec");

    // Use canonicalized command name
    let command_path = std::env::current_exe().expect("Failed to get executable path");
    command.arg(command_path);
    command.arg("--daemon");

    // Pipe stdin and stdout
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());

    let mut child = command.spawn().expect("Failed to spawn daemon");

    let stdin = child.stdin.take().expect("Failed to get stdin of daemon");
    let stdout = child.stdout.take().expect("Failed to get stdout of daemon");

    f(Rc::new(DaemonClient::new(stdout, stdin)));

    let status = child.wait().expect("Failed to wait for daemon");
    if ! status.success() {
        panic!("Failed to run daemon with exit status {:?}", status);
    }
}

#[cfg(not(target_os = "linux"))]
fn with_daemon<F: Fn(Rc<dyn Daemon>)>(f: F) {
    let server = daemon_server().expect("Failed to create server");
    f(Rc::new(server));
}

#[cfg(target_os = "macos")]
fn macos_init() {
    use std::{env, process};
    // This command returns Dark if we should use the dark theme
    // defaults read -g AppleInterfaceStyle
    if let Ok(output) = process::Command::new("defaults")
        .arg("read")
        .arg("-g")
        .arg("AppleInterfaceStyle")
        .output()
    {
        if output.stdout.starts_with(b"Dark") {
            let _ = env::set_var("GTK_THEME", "Adwaita:dark");
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_init() {
    use std::env;
    // This is a dword with a value of 0 if we should use the dark theme:
    // HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme
    use winreg::RegKey;
    let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    if let Ok(subkey) = hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize") {
        if let Ok(dword) = subkey.get_value::<u32, _>("AppsUseLightTheme") {
            if dword == 0 {
                let _ = env::set_var("GTK_THEME", "Adwaita:dark");
            }
        }
    }
}

pub fn run(args: Vec<String>) -> i32 {
    #[cfg(target_os = "macos")]
    macos_init();

    #[cfg(target_os = "windows")]
    windows_init();

    let application =
        gtk::Application::new(Some("com.system76.keyboard-layout"), Default::default())
            .expect("Failed to create gtk::Application");

    application.connect_activate(move |app| {
        if let Some(window) = app.get_active_window() {
            //TODO
            eprintln!("Focusing current window");
            window.present();
        } else {
            with_daemon(|daemon| {
                main_app(app, daemon);
                //TODO: is this the best way to keep the daemon running?
                gtk::main();
            });
        }
    });

    application.run(&args)
}
