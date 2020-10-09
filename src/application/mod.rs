use cascade::cascade;
use gio::prelude::*;
use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use glib::translate::{FromGlibPtrFull, ToGlib, ToGlibPtr};
use once_cell::unsync::OnceCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::rc::Rc;

use crate::daemon::{Daemon, DaemonClient, DaemonDummy, daemon_server};

mod error_dialog;
mod key;
mod keyboard;
pub(crate) mod layout;
mod page;
mod picker;
mod rect;

use keyboard::Keyboard;
use picker::Picker;

pub struct ConfiguratorAppInner {
    board_dropdown: gtk::ComboBoxText,
    count: AtomicUsize,
    picker: Picker,
    phony_board_names: OnceCell<Vec<String>>,
    scrolled_window: gtk::ScrolledWindow,
    stack: gtk::Stack,
    window: OnceCell<gtk::ApplicationWindow>,
}

impl ObjectSubclass for ConfiguratorAppInner {
    const NAME: &'static str = "S76ConfiguratorApp";

    type ParentType = gtk::Application;

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn new() -> Self {
        let board_dropdown = cascade! {
            gtk::ComboBoxText::new();
        };

        let stack = cascade! {
            gtk::Stack::new();
            ..set_transition_duration(0);
        };

        let picker = Picker::new();

        board_dropdown.connect_changed(clone!(@weak stack, @weak picker => @default-panic, move |combobox| {
            if let Some(id) = combobox.get_active_id() {
                stack.set_visible_child_name(&id);
                let keyboard = stack.get_child_by_name(&id).unwrap().downcast().unwrap();
                picker.set_keyboard(Some(keyboard));
            }
        }));

        let vbox = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 32);
            ..set_property_margin(10);
            ..set_halign(gtk::Align::Center);
            ..add(&board_dropdown);
            ..add(&stack);
            ..add(&picker);
        };

        let scrolled_window = cascade! {
            gtk::ScrolledWindow::new::<gtk::Adjustment, gtk::Adjustment>(None, None);
            ..add(&vbox);
        };

        Self {
            board_dropdown,
            count: AtomicUsize::new(0),
            picker,
            phony_board_names: OnceCell::new(),
            scrolled_window,
            stack,
            window: OnceCell::new(),
        }
    }
}

impl ObjectImpl for ConfiguratorAppInner {
    glib_object_impl!();

    fn constructed(&self, obj: &glib::Object) {
        self.parent_constructed(obj);

        let app: &ConfiguratorApp = obj.downcast_ref().unwrap();
        app.set_application_id(Some("com.system76.keyboard-layout"));
        app.add_main_option("fake-keyboard", glib::Char::new('k').unwrap(), glib::OptionFlags::NONE, glib::OptionArg::String, "", None);
    }
}

impl ApplicationImpl for ConfiguratorAppInner {
    fn handle_local_options(&self, _app: &gio::Application, opts: &glib::VariantDict) -> i32 {
        let board_names = if let Some(opt) = opts.lookup_value("fake-keyboard", None) {
            let value: String = opt.get().unwrap();

            if &value == "all" {
                layout::layouts().iter().map(|s| s.to_string()).collect()
            } else {
                value.split(',').map(str::to_string).collect()
            }
        } else {
            vec![]
        };

        let _ = self.phony_board_names.set(board_names);
        -1
    }

    fn activate(&self, app: &gio::Application) {
        let app: &ConfiguratorApp = app.downcast_ref().unwrap();

        if let Some(window) = app.get_active_window() {
            //TODO
            eprintln!("Focusing current window");
            window.present();
        } else {
            let window = cascade! {
                gtk::ApplicationWindow::new(app);
                ..set_title("Keyboard Layout");
                ..set_position(gtk::WindowPosition::Center);
                ..set_default_size(1024, 768);
                ..add(&self.scrolled_window);
            };

            window.set_focus::<gtk::Widget>(None);
            window.show_all();

            window.connect_destroy(|_| {
                eprintln!("Window close");
            });

            let _ = self.window.set(window);

            let daemon = daemon();
            let boards = daemon.boards().expect("Failed to load boards");

            for (i, board) in boards.iter().enumerate() {
                app.add_keyboard(daemon.clone(), board, i);
            }

            let phony_board_names = self.phony_board_names.get().unwrap();
            if !phony_board_names.is_empty() {
                let daemon = Rc::new(DaemonDummy::new(phony_board_names.clone()));
                let boards = daemon.boards().unwrap();

                for (i, board) in boards.iter().enumerate() {
                    app.add_keyboard(daemon.clone(), board, i);
                }
            } else if self.count.load(Ordering::Relaxed) == 0 {
                eprintln!("Failed to locate any keyboards, showing demo");

                let daemon = Rc::new(DaemonDummy::new(vec!["system76/launch_alpha_2".to_string()]));
                app.add_keyboard(daemon, "system76/launch_alpha_2", 0);
            }
        }
    }
}

impl GtkApplicationImpl for ConfiguratorAppInner {}

glib_wrapper! {
    pub struct ConfiguratorApp(
        Object<subclass::simple::InstanceStruct<ConfiguratorAppInner>,
        subclass::simple::ClassStruct<ConfiguratorAppInner>, ConfiguratorAppClass>)
        @extends gtk::Application, gio::Application;

    match fn {
        get_type => || ConfiguratorAppInner::get_type().to_glib(),
    }
}

impl ConfiguratorApp {
    fn new() -> Self {
        glib::Object::new(Self::static_type(), &[])
            .unwrap()
            .downcast()
            .unwrap()
    }

    fn inner(&self) -> &ConfiguratorAppInner {
        ConfiguratorAppInner::from_instance(self)
    }

    fn add_keyboard(&self, daemon: Rc<dyn Daemon>, board: &str, i: usize) {
        if let Some(keyboard) = Keyboard::new_board(board, daemon.clone(), i) {
            keyboard.show_all();

            // Generate unique ID for board, even with multiple of same model
            let mut num = 1;
            let mut board_id = format!("{}1", board);
            while self.inner().stack.get_child_by_name(&board_id).is_some() {
                num += 1;
                board_id = format!("{}{}", board, num);
            }

            self.inner().board_dropdown.append(Some(&board_id), &board);
            self.inner().stack.add_named(&keyboard, &board_id);

            if self.inner().count.fetch_add(1, Ordering::Relaxed) == 0 {
                self.inner().board_dropdown.set_active_id(Some(&board_id));
                self.inner().picker.set_keyboard(Some(keyboard.clone()));
            }
        } else {
            eprintln!("Failed to locate layout for '{}'", board);
        }
    }
}

#[cfg(target_os = "linux")]
fn daemon() -> Rc<dyn Daemon> {
    use std::{
        env,
        path::PathBuf,
        process::{
            Command,
            Stdio,
        },
    };

    if unsafe { libc::geteuid() == 0 } {
        eprintln!("Already running as root");
        let server = daemon_server().expect("Failed to create server");
        return Rc::new(server);
    }

    // Use pkexec to spawn daemon as superuser
    eprintln!("Not running as root, spawning daemon with pkexec");
    let mut command = Command::new("pkexec");

    // Use canonicalized command name
    let command_path = if cfg!(feature = "appimage") {
        PathBuf::from(env::var("APPIMAGE").expect("Failed to get executable path"))
    } else {
        env::current_exe().expect("Failed to get executable path")
    };

    command.arg(command_path);
    command.arg("--daemon");

    // Pipe stdin and stdout
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());

    let mut child = command.spawn().expect("Failed to spawn daemon");

    let stdin = child.stdin.take().expect("Failed to get stdin of daemon");
    let stdout = child.stdout.take().expect("Failed to get stdout of daemon");

    Rc::new(DaemonClient::new(child, stdout, stdin))
}

#[cfg(not(target_os = "linux"))]
fn daemon() -> Rc<dyn Daemon> {
    let server = daemon_server().expect("Failed to create server");
    Rc::new(server)
}

#[cfg(target_os = "macos")]
fn macos_init() {
    use gtk::SettingsExt;
    use std::{env, process};
    let mut prefer_dark = false;
    // This command returns Dark if we should use the dark theme
    // defaults read -g AppleInterfaceStyle
    if let Ok(output) = process::Command::new("defaults")
        .arg("read")
        .arg("-g")
        .arg("AppleInterfaceStyle")
        .output()
    {
        prefer_dark = output.stdout.starts_with(b"Dark");
    }

    if let Some(settings) = gtk::Settings::get_default() {
        settings.set_property_gtk_application_prefer_dark_theme(prefer_dark);
    }
}

#[cfg(target_os = "windows")]
fn windows_init() {
    // This is a dword with a value of 0 if we should use the dark theme:
    // HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme
    use gtk::SettingsExt;
    use winreg::RegKey;
    let mut prefer_dark = false;
    let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    if let Ok(subkey) = hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize") {
        if let Ok(dword) = subkey.get_value::<u32, _>("AppsUseLightTheme") {
            prefer_dark = (dword == 0);
        }
    }

    if let Some(settings) = gtk::Settings::get_default() {
        settings.set_property_gtk_application_prefer_dark_theme(prefer_dark);
    }
}

pub fn run(args: Vec<String>) -> i32 {
    gtk::init().unwrap();

    #[cfg(target_os = "macos")]
    macos_init();

    #[cfg(target_os = "windows")]
    windows_init();

    let application = ConfiguratorApp::new();
    application.run(&args)
}
