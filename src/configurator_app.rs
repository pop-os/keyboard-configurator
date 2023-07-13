use cascade::cascade;
use gtk::{gdk, gio, glib, prelude::*, subclass::prelude::*};
use std::cell::Cell;
use std::process::Command;

use crate::{about_dialog, fl, MainWindow, Page};
use backend::DerefCell;

#[derive(Default)]
pub struct ConfiguratorAppInner {
    phony_board_names: DerefCell<Vec<String>>,
    debug_layers: Cell<bool>,
    launch_test: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for ConfiguratorAppInner {
    const NAME: &'static str = "S76ConfiguratorApp";
    type ParentType = gtk::Application;
    type Type = ConfiguratorApp;
}

impl ObjectImpl for ConfiguratorAppInner {
    fn constructed(&self) {
        let app = self.obj();

        app.set_application_id(Some("com.system76.keyboardconfigurator"));

        self.parent_constructed();

        app.add_main_option(
            "fake-keyboard",
            glib::Char::from(b'k'),
            glib::OptionFlags::NONE,
            glib::OptionArg::String,
            "",
            None,
        );
        app.add_main_option(
            "debug-layers",
            glib::Char::from(b'\0'),
            glib::OptionFlags::NONE,
            glib::OptionArg::None,
            "",
            None,
        );
        app.add_main_option(
            "launch-test",
            glib::Char::from(b'\0'),
            glib::OptionFlags::NONE,
            glib::OptionArg::None,
            "",
            None,
        );
    }
}

impl ApplicationImpl for ConfiguratorAppInner {
    fn handle_local_options(&self, opts: &glib::VariantDict) -> glib::ExitCode {
        fn lookup<T: glib::FromVariant>(opts: &glib::VariantDict, key: &str) -> Option<T> {
            opts.lookup_value(key, None)?.get()
        }

        let board_names = match lookup::<String>(opts, "fake-keyboard").as_deref() {
            Some("all") => backend::layouts().iter().map(|s| s.to_string()).collect(),
            Some(value) => value.split(',').map(str::to_string).collect(),
            None => vec![],
        };

        self.phony_board_names.set(board_names);
        self.debug_layers.set(opts.contains("debug-layers"));
        self.launch_test.set(opts.contains("launch-test"));

        self.parent_handle_local_options(opts)
    }

    fn startup(&self) {
        self.parent_startup();

        let about_action = cascade! {
            gio::SimpleAction::new("about", None);
            ..connect_activate(|_, _| about_dialog::show_about_dialog());
        };

        let flash_1 = cascade! {
            gio::SimpleAction::new("flash-to-launch-1", None);
            ..connect_activate(|_, _| {
                let _ = Command::new("dfu-programmer").args(["atmega32u4", "erase"]).status();
                let _ = Command::new("dfu-programmer").args(["atmega32u4", "flash", "/var/lib/system76-keyboard-configurator/system76_launch_1_default.hex", "--suppress-bootloader-mem"]).status();
                let _ = Command::new("dfu-programmer").args(["atmega32u4", "start"]).status();
            });
        };
        let flash_2 = cascade! {
            gio::SimpleAction::new("flash-to-launch-2", None);
            ..connect_activate(|_, _| {
                let _ = Command::new("dfu-programmer").args(["at90usb646", "flash", "/var/lib/system76-keyboard-configurator/system76_launch_2_default.hex", "--suppress-bootloader-mem"]).status();
                let _ = Command::new("dfu-programmer").args(["at90usb646", "reset"]).status();
            });
        };
        let flash_lite_1 = cascade! {
            gio::SimpleAction::new("flash-to-launch-lite-1", None);
            ..connect_activate(|_, _| {
                let _ = Command::new("dfu-programmer").args(["at90usb646", "flash", "/var/lib/system76-keyboard-configurator/system76_launch_lite_1_default.hex", "--suppress-bootloader-mem"]).status();
                let _ = Command::new("dfu-programmer").args(["at90usb646", "reset"]).status();
            });
        };
        let flash_heavy_1 = cascade! {
            gio::SimpleAction::new("flash-to-launch-heavy-1", None);
            ..connect_activate(|_, _| {
                let _ = Command::new("dfu-programmer").args(["at90usb646", "flash", "/var/lib/system76-keyboard-configurator/system76_launch_heavy_1_default.hex", "--suppress-bootloader-mem"]).status();
                let _ = Command::new("dfu-programmer").args(["at90usb646", "reset"]).status();
            });
        };

        let app = self.obj();
        app.add_action(&about_action);
        app.add_action(&flash_heavy_1);
        app.add_action(&flash_2);
        app.add_action(&flash_1);
        app.add_action(&flash_lite_1);
        app.set_accels_for_action("kbd.import", &["<Primary>o"]);
        app.set_accels_for_action("kbd.export", &["<Primary>e"]);
        for (i, _) in Page::iter_all().enumerate() {
            app.set_accels_for_action(&format!("kbd.page{}", i), &[&format!("<Primary>{}", i + 1)]);
        }
    }

    fn activate(&self) {
        self.parent_activate();

        if let Some(window) = self.obj().active_window() {
            info!("Focusing current window");
            window.present();
        } else {
            MainWindow::new(&*self.obj());
        }
    }
}

impl GtkApplicationImpl for ConfiguratorAppInner {}

glib::wrapper! {
    pub struct ConfiguratorApp(ObjectSubclass<ConfiguratorAppInner>)
        @extends gtk::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl ConfiguratorApp {
    fn new() -> Self {
        glib::Object::new()
    }

    fn inner(&self) -> &ConfiguratorAppInner {
        ConfiguratorAppInner::from_obj(self)
    }

    pub fn phony_board_names(&self) -> &[String] {
        &self.inner().phony_board_names
    }

    pub fn debug_layers(&self) -> bool {
        self.inner().debug_layers.get()
    }

    pub fn launch_test(&self) -> bool {
        self.inner().launch_test.get()
    }
}

#[cfg(target_os = "macos")]
fn macos_init() {
    use gtk::traits::SettingsExt;
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

    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_decoration_layout(Some("close,minimize,maximize:menu"));
        settings.set_gtk_application_prefer_dark_theme(prefer_dark);
        settings.set_gtk_enable_animations(false);
    }

    let css_provider = cascade! {
        gtk::CssProvider::new();
        ..load_from_data(b"
            button, button:hover {
                box-shadow: none;
                -gtk-icon-shadow: none;
                text-shadow: none;
            }
        ").unwrap();
    };

    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().unwrap(),
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

#[cfg(target_os = "windows")]
fn windows_init() {
    // This is a dword with a value of 0 if we should use the dark theme:
    // HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme
    use gtk::traits::SettingsExt;
    use winreg::RegKey;
    let mut prefer_dark = false;
    let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    if let Ok(subkey) =
        hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
    {
        if let Ok(dword) = subkey.get_value::<u32, _>("AppsUseLightTheme") {
            prefer_dark = dword == 0;
        }
    }

    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(prefer_dark);
    }
}

pub fn run() -> glib::ExitCode {
    gtk::init().unwrap();

    glib::set_prgname(Some("com.system76.keyboardconfigurator"));
    glib::set_application_name(&fl!("app-title"));
    gdk::set_program_class(&fl!("app-title"));

    #[cfg(target_os = "macos")]
    macos_init();

    #[cfg(target_os = "windows")]
    windows_init();

    gio::resources_register_include!("compiled.gresource").unwrap();
    gtk::Window::set_default_icon_name("com.system76.keyboardconfigurator");

    let application = ConfiguratorApp::new();
    application.run()
}
