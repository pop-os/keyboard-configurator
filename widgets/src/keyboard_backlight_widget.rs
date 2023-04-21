// Intended for use in Gnome Control Center's Keyboard panel

use crate::fl;
use cascade::cascade;
use gtk::{
    glib::{self, clone},
    prelude::*,
};

use crate::{KeyboardColor, KeyboardColorIndex};
use backend::{Backend, Board};

pub fn keyboard_backlight_widget() -> gtk::Widget {
    let stack = cascade! {
        gtk::Stack::new();
        ..style_context().add_class("frame");
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

    if let Err(err) = add_boards(&stack) {
        eprintln!("Failed to get keyboards: {}", err);
    }

    vbox.upcast()
}

fn add_boards(stack: &gtk::Stack) -> Result<(), String> {
    let backend = Backend::new_s76power()?;
    backend.connect_board_added(clone!(@weak stack => move |board| {
        let name = board.model().to_owned();
        stack.add_titled(&page(board), &name, &name);
    }));
    backend.refresh();

    Ok(())
}

fn page(board: Board) -> gtk::Widget {
    let max_brightness = board.max_brightness() as f64;
    let brightness = board.layers()[0].brightness() as f64;
    let brightness_scale = cascade! {
        gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., max_brightness, 1.);
        ..set_hexpand(true);
        ..set_draw_value(false);
        ..set_value(brightness);
        ..connect_change_value(clone!(@strong board => move |_scale, _, value| {
            glib::MainContext::default().spawn_local(clone!(@strong board => async move {
                if let Err(err) = board.layers()[0].set_brightness(value as i32).await {
                    eprintln!("{}: {}", fl!("error-set-brightness"), err);
                }
            }));
            Inhibit(false)
        }));
    };

    // TODO detect when brightness changed in daemon

    let button = KeyboardColor::new(Some(board), KeyboardColorIndex::Layer(0));

    let listbox = cascade! {
        gtk::ListBox::new();
        ..set_header_func(Some(Box::new(|row, before| {
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            row.set_header(before.and(Some(&separator)));
        })));
        ..add(&row(&fl!("scale-brightness"), &brightness_scale, true));
        ..add(&row(&fl!("button-color"), &button, false));
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
        ..set_margin(12);
        ..add(&hbox);
    };

    list_box_row
}
