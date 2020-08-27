#![windows_subsystem = "windows"]

use cascade::cascade;
use gtk::prelude::*;

use pop_keyboard_backlight::{keyboards, Keyboard, KeyboardColorButton};

fn page(keyboard: Keyboard) -> gtk::Widget {
    let button = KeyboardColorButton::new(keyboard.clone()).widget().clone();

    let label = cascade! {
        gtk::Label::new(Some("Color"));
        ..set_justify(gtk::Justification::Left);
    };

    let row_box = cascade! {
        gtk::Box::new(gtk::Orientation::Horizontal, 0);
        ..set_hexpand(true);
        ..set_vexpand(true);
        ..pack_start(&label, false, false, 0);
        ..pack_end(&button, false, false, 0);
    };

    let keyboard_clone = keyboard.clone();
    let max_brightness = keyboard.get_max_brightness().unwrap() as f64;
    let brightness_scale = cascade! {
        gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., max_brightness, 1.);
        ..connect_value_changed(move |scale| {
            keyboard_clone.set_brightness(scale.get_value() as i32);
        });
    };

    let brightness_row = cascade! {
        gtk::ListBoxRow::new();
        ..set_selectable(false);
        ..set_activatable(false);
        ..set_margin_top(12);
        ..set_margin_bottom(12);
        ..set_margin_start(12);
        ..set_margin_end(12);
        ..add(&brightness_scale);
    };

    let row = cascade! {
        gtk::ListBoxRow::new();
        ..set_selectable(false);
        ..set_activatable(false);
        ..set_margin_top(12);
        ..set_margin_bottom(12);
        ..set_margin_start(12);
        ..set_margin_end(12);
        ..add(&row_box);
    };

    let listbox = cascade! {
        gtk::ListBox::new();
        ..add(&brightness_row);
        ..add(&row);
    };

    listbox.upcast()
}

fn main() {
    gtk::init().unwrap();

    let notebook = gtk::Notebook::new();

    for i in keyboards() {
        let title = format!("{}", i);
        let label = gtk::Label::new(Some(&title));
        notebook.append_page(&page(i), Some(&label));
    }

    let _window = cascade! {
        gtk::Window::new(gtk::WindowType::Toplevel);
        ..set_default_size(500, 500);
        ..add(&notebook);
        ..show_all();
    };

    gtk::main();
}
