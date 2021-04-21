#[macro_use]
extern crate log;

mod choose_color;
mod color_circle;
mod color_wheel;
mod keyboard_color;
mod selected_keys;

pub use crate::{
    choose_color::*, color_circle::*, color_wheel::*, keyboard_color::*, selected_keys::*,
};
pub use backend;
use backend::DerefCell;

#[cfg(target_os = "linux")]
mod keyboard_backlight_widget;
#[cfg(target_os = "linux")]
pub use keyboard_backlight_widget::keyboard_backlight_widget;
