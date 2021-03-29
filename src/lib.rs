#[macro_use]
extern crate log;
extern crate system76_keyboard_configurator_daemon as daemon;

pub mod application;

mod choose_color;
mod color_circle;
mod color_wheel;
mod deref_cell;
mod keyboard_backlight_widget;
mod keyboard_color;
mod keymap;

pub use daemon::DaemonServer;

use crate::{
    choose_color::*, color_circle::*, color_wheel::*, daemon::*, deref_cell::*, keyboard_color::*,
    keymap::*,
};
pub use keyboard_backlight_widget::keyboard_backlight_widget;
