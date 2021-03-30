#[macro_use]
extern crate log;

pub mod application;

mod choose_color;
mod color_circle;
mod color_wheel;
mod deref_cell;
mod keyboard_backlight_widget;
mod keyboard_color;

use crate::{choose_color::*, color_circle::*, color_wheel::*, deref_cell::*, keyboard_color::*};
pub use daemon;
pub use keyboard_backlight_widget::keyboard_backlight_widget;
