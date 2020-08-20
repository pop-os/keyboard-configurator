use cascade::cascade;
use gtk::prelude::*;

use pop_keyboard_backlight::KeyboardColorButton;

fn keyboard_color_button() -> gtk::Widget {
    let button = KeyboardColorButton::new();
    button.widget().clone()
}

fn main() {
    gtk::init().unwrap();

    let button = keyboard_color_button();

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

    let brightness_scale = cascade! {
        gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., 100., 1.);
        ..connect_value_changed(|_| {});
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

    let _window = cascade! {
        gtk::Window::new(gtk::WindowType::Toplevel);
        ..set_default_size(500, 500);
        ..add(&listbox);
        ..show_all();
    };

    gtk::main();
}
