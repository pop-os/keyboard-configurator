mod choose_color;
mod color;
mod color_circle;
mod color_wheel;
mod keyboard_color_button;

use system76_power::{client::PowerClient, Power};

pub use keyboard_color_button::KeyboardColorButton;

fn set_keyboard_color(rgb: color::Rgb) {
    let mut client = PowerClient::new().unwrap();
    client.set_keyboard_color(&rgb.to_string()).unwrap();
}