// Intended for use in Gnome Control Center's Keyboard panel

use cascade::cascade;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;

use crate::keyboard::{keyboards, Keyboard};
use crate::keyboard_color_button::KeyboardColorButton;

pub fn keyboard_backlight_widget() -> gtk::Widget {
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
        ..add(&stack_switcher);
        ..add(&stack);
    };

    for i in keyboards() {
        let title = format!("{}", i);
        stack.add_titled(&page(i), &title, &title);
    }

    vbox.upcast()
}

fn page(keyboard: Keyboard) -> gtk::Widget {
    let max_brightness = keyboard.max_brightness().unwrap() as f64;
    let brightness = keyboard.brightness().unwrap() as f64;
    let brightness_scale = cascade! {
        gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., max_brightness, 1.);
        ..set_hexpand(true);
        ..set_draw_value(false);
        ..set_value(brightness);
        ..connect_change_value(clone!(@weak keyboard => @default-panic, move |_scale, _, value| {
            keyboard.set_brightness(value as i32);
            Inhibit(false)
        }));
    };

    keyboard.connect_brightness_changed(
        clone!(@weak brightness_scale => @default-panic, move |_, brightness| {
            brightness_scale.set_value(brightness as f64);
        }),
    );

    let button = KeyboardColorButton::new(keyboard);

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
