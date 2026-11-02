#![windows_subsystem = "windows"]

#[macro_use]
extern crate log;

use i18n_embed::DesktopLanguageRequester;
use std::env;

mod about_dialog;
mod backlight;
mod configurator_app;
mod error_dialog;
mod keyboard;
mod keyboard_layer;
mod keycap;
mod localize;
mod main_window;
mod page;
mod picker;
mod quick_switch;
mod shortcuts_window;
mod smoke;
mod testing;

use gtk::glib;

pub use self::configurator_app::run;
use self::{
    backlight::*, configurator_app::*, error_dialog::*, keyboard::*, keyboard_layer::*,
    main_window::*, page::*, picker::*, shortcuts_window::*, testing::*,
};

fn main() -> glib::ExitCode {
    translate();
    env_logger::Builder::from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    )
    .format_timestamp(None)
    .format_module_path(false)
    .init();

    let args = env::args().collect::<Vec<_>>();
    // The same mutex name is used by the companion. Hold it for the entire app
    // lifetime, not just an individual write. Isolated dummy runs never use HID.
    #[cfg(windows)]
    let _device_lock = if env::var_os("LAUNCH_HEAVY_FAKE_ONLY").is_none() {
        match profile_engine::DeviceLock::acquire() {
            Ok(lock) => Some(lock),
            Err(message) => {
                if gtk::init().is_ok() {
                    use gtk::prelude::*;
                    let dialog = gtk::MessageDialog::new(
                        None::<&gtk::Window>,
                        gtk::DialogFlags::MODAL,
                        gtk::MessageType::Warning,
                        gtk::ButtonsType::Close,
                        &fl!("quick-switch-device-busy", message = message),
                    );
                    dialog.run();
                    dialog.close();
                }
                return glib::ExitCode::FAILURE;
            }
        }
    } else {
        None
    };
    for arg in args.iter().skip(1) {
        if arg.as_str() == "--daemon" {
            backend::run_daemon();
        }
    }

    crate::run()
}

fn translate() {
    let requested_languages = if env::var_os("LAUNCH_HEAVY_FAKE_ONLY").is_some() {
        env::var("CONFIGURATOR_SMOKE_LANGUAGE")
            .ok()
            .and_then(|value| value.parse().ok())
            .map(|language| vec![language])
            .unwrap_or_else(DesktopLanguageRequester::requested_languages)
    } else {
        DesktopLanguageRequester::requested_languages()
    };

    let localizers = vec![
        ("keyboard-configurator", crate::localize::localizer()),
        ("backend", backend::localizer()),
        ("widgets", widgets::localizer()),
    ];

    for (crate_name, localizer) in localizers {
        if let Err(error) = localizer.select(&requested_languages) {
            eprintln!("Error while loading languages for {} {}", crate_name, error);
        }
    }
}
