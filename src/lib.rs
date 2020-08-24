mod choose_color;
mod color;
mod color_circle;
mod color_wheel;
mod keyboard_color_button;

#[cfg(target_os = "linux")]
use system76_power::{client::PowerClient, Power};

pub use keyboard_color_button::KeyboardColorButton;

#[cfg(target_os = "linux")]
fn set_keyboard_color(rgb: color::Rgb) {
    let mut client = PowerClient::new().unwrap();
    client.set_keyboard_color(&rgb.to_string()).unwrap();
}

#[cfg(windows)]
fn set_keyboard_color(_rgb: color::Rgb) {
    eprintln!("Color setting not implemented on Windows yet");
}