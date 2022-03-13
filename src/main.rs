#![windows_subsystem = "windows"]

#[macro_use]
extern crate log;

use i18n_embed::DesktopLanguageRequester;
use std::env;
use std::process;

mod about_dialog;
mod backlight;
mod cli;
mod configurator_app;
mod error_dialog;
mod keyboard;
mod keyboard_layer;
mod localize;
mod main_window;
mod page;
mod picker;
mod shortcuts_window;
mod testing;

pub use self::configurator_app::run;
use self::{
    backlight::*, configurator_app::*, error_dialog::*, keyboard::*, keyboard_layer::*,
    main_window::*, page::*, picker::*, shortcuts_window::*, testing::*,
};

fn main() {
    translate();
    env_logger::Builder::from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    )
    .format_timestamp(None)
    .format_module_path(false)
    .init();

    let args = env::args().collect::<Vec<_>>();
    for arg in args.iter().skip(1) {
        if arg.as_str() == "--daemon" {
            backend::run_daemon();
        }
    }

    process::exit(crate::run());
}

fn translate() {
    let requested_languages = DesktopLanguageRequester::requested_languages();

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
