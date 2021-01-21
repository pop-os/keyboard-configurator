use cascade::cascade;
use glib::clone;
use gtk::prelude::*;

use crate::color::Rgb;
use crate::color_wheel::ColorWheel;
use crate::keyboard::Keyboard;

pub fn choose_color<W: IsA<gtk::Widget>>(
    keyboard: Keyboard,
    w: &W,
    title: &'static str,
    color: Option<Rgb>,
) -> Option<Rgb> {
    let color_wheel = cascade! {
        ColorWheel::new();
        ..set_size_request(300, 300);
    };

    if let Some(color) = color {
        color_wheel.set_hs(color.to_hs_lossy());
    }

    let preview = cascade! {
        gtk::DrawingArea::new();
        ..set_halign(gtk::Align::Center);
        ..set_size_request(300, 25);
        ..connect_draw(clone!(@weak color_wheel => @default-panic, move |_w, cr| {
            let (r, g, b) = color_wheel.hs().to_rgb().to_floats();
            cr.set_source_rgb(r, g, b);
            cr.paint();
            Inhibit(false)
        }));
    };

    color_wheel.connect_hs_changed(clone!(@weak preview => @default-panic, move |wheel| {
        keyboard.set_color(wheel.hs().to_rgb());
        preview.queue_draw();
    }));

    let vbox = cascade! {
        gtk::Box::new(gtk::Orientation::Vertical, 12);
        ..set_margin_start(24);
        ..set_margin_end(24);
        ..set_margin_top(24);
        ..set_margin_bottom(24);
        ..add(&color_wheel);
        ..add(&preview);
        ..show_all();
    };

    let window = w
        .get_toplevel()
        .and_then(|x| x.downcast::<gtk::Window>().ok());

    let dialog = gtk::DialogBuilder::new()
        .title(title)
        .use_header_bar(1)
        .modal(true)
        .build();

    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button("Save", gtk::ResponseType::Ok);
    dialog.get_content_area().add(&vbox);
    dialog.set_transient_for(window.as_ref());

    let response = dialog.run();
    let rgb = color_wheel.hs().to_rgb();
    dialog.close();

    if response == gtk::ResponseType::Ok {
        Some(rgb)
    } else {
        None
    }
}
