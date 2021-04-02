#[macro_use]
extern crate log;

pub mod application;

mod choose_color;
mod color_circle;
mod color_wheel;
mod keyboard_backlight_widget;
mod keyboard_color;
mod selected_keys;

use crate::{
    choose_color::*, color_circle::*, color_wheel::*, keyboard_color::*, selected_keys::*,
};
pub use backend;
use backend::DerefCell;
pub use keyboard_backlight_widget::keyboard_backlight_widget;
