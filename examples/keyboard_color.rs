#![windows_subsystem = "windows"]

use cascade::cascade;
use gio::prelude::*;
use gtk::prelude::*;
use std::env;
use std::process;

use pop_keyboard_backlight::{keyboards, Keyboard, KeyboardColorButton};

fn row<W: IsA<gtk::Widget>>(text: &str, widget: &W, expand: bool) -> gtk::ListBoxRow {
    let label = cascade! {
        gtk::Label::new(Some(text));
        ..set_justify(gtk::Justification::Left);
    };

    let hbox = cascade! {
        gtk::Box::new(gtk::Orientation::Horizontal, 24);
        ..set_hexpand(true);
        ..set_vexpand(true);
        ..pack_start(&label, false, false, 0);
        ..pack_end(widget, expand, expand, 0);
    };

    let list_box_row = cascade! {
        gtk::ListBoxRow::new();
        ..set_selectable(false);
        ..set_activatable(false);
        ..set_margin_top(12);
        ..set_margin_bottom(12);
        ..set_margin_start(12);
        ..set_margin_end(12);
        ..add(&hbox);
    };

    list_box_row
}

fn page(keyboard: Keyboard) -> gtk::Widget {
    let keyboard_clone = keyboard.clone();
    let max_brightness = keyboard.get_max_brightness().unwrap() as f64;
    let brightness_scale = cascade! {
        gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., max_brightness, 1.);
        ..set_hexpand(true);
        ..set_draw_value(false);
        ..connect_value_changed(move |scale| {
            keyboard_clone.set_brightness(scale.get_value() as i32);
        });
    };

    let button = KeyboardColorButton::new(keyboard.clone()).widget().clone();

    let listbox = cascade! {
        gtk::ListBox::new();
        ..set_header_func(Some(Box::new(|row, before| {
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            row.set_header(before.and(Some(&separator)));
        })));
        ..add(&row("Brightness", &brightness_scale, true));
        ..add(&row("Color", &button, false));
    };

    listbox.upcast()
}

fn main() {
    gtk::init().unwrap();

    let stack = cascade! {
        gtk::Stack::new();
        ..get_style_context().add_class("frame");
        ..set_transition_type(gtk::StackTransitionType::SlideLeftRight);
    };

    let stack_switcher = cascade! {
        gtk::StackSwitcher::new();
        ..set_stack(Some(&stack));
    };

    let vbox = cascade! {
        gtk::Box::new(gtk::Orientation::Vertical, 12);
        ..set_margin_top(12);
        ..set_margin_bottom(12);
        ..set_margin_start(12);
        ..set_margin_end(12);
        ..add(&stack_switcher);
        ..add(&stack);
    };

    for i in keyboards() {
        let title = format!("{}", i);
        stack.add_titled(&page(i), &title, &title);
    }

    let application = cascade! {
        gtk::Application::new(None, gio::ApplicationFlags::FLAGS_NONE).unwrap();

    };

    application.connect_activate(move |app| {
        cascade! {
            gtk::ApplicationWindow::new(app);
            ..set_default_size(500, 500);
            ..add(&vbox);
            ..show_all();
        }
    });

    process::exit(application.run(&env::args().collect::<Vec<_>>()));
}
