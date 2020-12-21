use gio::prelude::*;
use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use glib::translate::{FromGlibPtrFull, ToGlib, ToGlibPtr};
use once_cell::unsync::OnceCell;

use main_window::MainWindow;

mod error_dialog;
mod gresource;
mod key;
mod keyboard;
pub(crate) mod layout;
mod main_window;
mod page;
mod picker;
mod rect;

pub struct ConfiguratorAppInner {
    phony_board_names: OnceCell<Vec<String>>,
}

impl ObjectSubclass for ConfiguratorAppInner {
    const NAME: &'static str = "S76ConfiguratorApp";

    type ParentType = gtk::Application;

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn new() -> Self {
        Self {
            phony_board_names: OnceCell::new(),
        }
    }
}

impl ObjectImpl for ConfiguratorAppInner {
    glib_object_impl!();

    fn constructed(&self, obj: &glib::Object) {
        self.parent_constructed(obj);

        let app: &ConfiguratorApp = obj.downcast_ref().unwrap();
        app.set_application_id(Some("com.system76.keyboard-configurator"));
        app.set_resource_base_path(Some("/com/system76/keyboard-configurator"));
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
            let phony_board_names = self.phony_board_names.get().unwrap();
            let window = MainWindow::new(phony_board_names.clone());
            app.add_window(&window);
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

    gresource::init().expect("failed to init configurator gresource");

    let application = ConfiguratorApp::new();
    application.run(&args)
}
