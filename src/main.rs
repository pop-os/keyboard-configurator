#![windows_subsystem = "windows"]

#[macro_use]
extern crate log;

use std::env;
use std::process;

mod about_dialog;
mod backlight;
mod configurator_app;
mod error_dialog;
mod keyboard;
mod keyboard_layer;
mod main_window;
mod page;
mod picker;
mod shortcuts_window;

pub use self::configurator_app::run;
use self::{
    backlight::*, configurator_app::*, error_dialog::*, keyboard::*, keyboard_layer::*,
    main_window::*, page::*, picker::*, shortcuts_window::*,
};

fn main() {
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
