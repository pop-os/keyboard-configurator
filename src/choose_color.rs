use cascade::cascade;
use gtk::prelude::*;

use crate::color::Rgb;
use crate::color_wheel::ColorWheel;
use crate::set_keyboard_color;

pub fn choose_color<W: IsA<gtk::Widget>>(w: &W, title: &'static str) -> Option<Rgb> {
    let color_wheel = ColorWheel::new();

    let color_wheel_clone = color_wheel.clone();
    let preview = cascade! {
        gtk::DrawingArea::new();
        ..set_halign(gtk::Align::Center);
        ..set_size_request(300, 25);
        ..connect_draw(move |_w, cr| {
            let (r, g, b) = color_wheel_clone.hs().to_rgb().to_floats();
            cr.set_source_rgb(r, g, b);
            cr.paint();
            Inhibit(false)
        });
    };

    let preview_clone = preview.clone();
    color_wheel.connect_hs_changed(move |wheel| {
        set_keyboard_color(wheel.hs().to_rgb());
        preview_clone.queue_draw();
    });

    let vbox = cascade! {
        gtk::Box::new(gtk::Orientation::Vertical, 12);
        ..set_margin_start(12);
        ..set_margin_end(12);
        ..set_margin_top(12);
        ..set_margin_bottom(12);
        ..add(color_wheel.widget());
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
