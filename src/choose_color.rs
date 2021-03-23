use cascade::cascade;
use glib::clone;
use gtk::prelude::*;

use crate::color::Hs;
use crate::color_wheel::ColorWheel;
use crate::daemon::DaemonBoard;

pub fn choose_color<W: IsA<gtk::Widget>, F: Fn(Option<Hs>) + 'static>(
    board: DaemonBoard,
    index: u8,
    w: &W,
    title: &'static str,
    color: Option<Hs>,
    cb: F,
) {
    let color_wheel = cascade! {
        ColorWheel::new();
        ..set_size_request(300, 300);
    };

    if let Some(color) = color {
        color_wheel.set_hs(color);
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
        if let Err(err) = board.set_color(index, wheel.hs()) {
            error!("Failed to set keyboard color: {}", err);
        }
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
    };

    let window = w
        .get_toplevel()
        .and_then(|x| x.downcast::<gtk::Window>().ok());

    cascade! {
        gtk::DialogBuilder::new()
            .title(title)
            .use_header_bar(1)
            .modal(true)
            .build();
        ..add_button("Cancel", gtk::ResponseType::Cancel);
        ..add_button("Save", gtk::ResponseType::Ok);
        ..get_content_area().add(&vbox);
        ..set_transient_for(window.as_ref());
        ..connect_response(move |dialog, response| {
            let hs = color_wheel.hs();
            dialog.close();

            cb(if response == gtk::ResponseType::Ok {
                Some(hs)
            } else {
                None
            })
        });
        ..show_all();
    };
}
