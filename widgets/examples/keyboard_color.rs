#![windows_subsystem = "windows"]

use cascade::cascade;
use gio::prelude::*;
use gtk::prelude::*;
use std::process;

use system76_keyboard_configurator_widgets::keyboard_backlight_widget;

fn main() {
    gtk::init().unwrap();

    let application = cascade! {
        gtk::Application::new(None, gio::ApplicationFlags::FLAGS_NONE).unwrap();
        ..connect_activate(move |app| {
            let backlight_widget = cascade! {
                keyboard_backlight_widget();
                ..set_margin_top(12);
                ..set_margin_bottom(12);
                ..set_margin_start(12);
                ..set_margin_end(12);
            };

            cascade! {
                gtk::ApplicationWindow::new(app);
                ..set_default_size(500, 500);
                ..add(&backlight_widget);
                ..show_all();
            };
        });
    };

    process::exit(application.run());
}
