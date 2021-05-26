use cascade::cascade;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::Cell;

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
    fn constructed(&self, app: &ConfiguratorApp) {
        app.set_application_id(Some("com.system76.keyboardconfigurator"));

        self.parent_constructed(app);

        app.add_main_option(
            "fake-keyboard",
            glib::Char::new('k').unwrap(),
            glib::OptionFlags::NONE,
            glib::OptionArg::String,
            "",
            None,
        );
        app.add_main_option(
            "debug-layers",
            glib::Char::new('\0').unwrap(),
            glib::OptionFlags::NONE,
            glib::OptionArg::None,
            "",
            None,
        );
        app.add_main_option(
            "launch-test",
            glib::Char::new('\0').unwrap(),
            glib::OptionFlags::NONE,
            glib::OptionArg::None,
            "",
            None,
        );
    }
}

impl ApplicationImpl for ConfiguratorAppInner {
    fn handle_local_options(&self, _app: &ConfiguratorApp, opts: &glib::VariantDict) -> i32 {
        let board_names = if let Some(opt) = opts.lookup_value("fake-keyboard", None) {
            let value: String = opt.get().unwrap();

            if &value == "all" {
                backend::layouts().iter().map(|s| s.to_string()).collect()
            } else {
                value.split(',').map(str::to_string).collect()
            }
        } else {
            vec![]
        };

        self.phony_board_names.set(board_names);
        self.debug_layers.set(opts.contains("debug-layers"));
        self.launch_test.set(opts.contains("launch-test"));
        -1
    }

    fn startup(&self, app: &ConfiguratorApp) {
        self.parent_startup(app);

        let about_action = cascade! {
            gio::SimpleAction::new("about", None);
            ..connect_activate(|_, _| about_dialog::show_about_dialog());
        };

        app.add_action(&about_action);
        app.set_accels_for_action("kbd.import", &["<Primary>o"]);
        app.set_accels_for_action("kbd.export", &["<Primary>e"]);
        for (i, _) in Page::iter_all().enumerate() {
            app.set_accels_for_action(&format!("kbd.page{}", i), &[&format!("<Primary>{}", i + 1)]);
        }
    }

    fn activate(&self, app: &ConfiguratorApp) {
        self.parent_activate(app);

        if let Some(window) = app.get_active_window() {
            info!("Focusing current window");
            window.present();
        } else {
            MainWindow::new(app);
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
        glib::Object::new(&[]).unwrap()
    }

    fn inner(&self) -> &ConfiguratorAppInner {
        ConfiguratorAppInner::from_instance(self)
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
        settings.set_property_gtk_decoration_layout(Some("close,minimize,maximize:menu"));
        settings.set_property_gtk_application_prefer_dark_theme(prefer_dark);
        settings.set_property_gtk_enable_animations(false);
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
        &gdk::Screen::get_default().unwrap(),
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

#[cfg(target_os = "windows")]
fn windows_init() {
    // This is a dword with a value of 0 if we should use the dark theme:
    // HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme
    use gtk::SettingsExt;
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

    if let Some(settings) = gtk::Settings::get_default() {
        settings.set_property_gtk_application_prefer_dark_theme(prefer_dark);
    }
}

pub fn run() -> i32 {
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
