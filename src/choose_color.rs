use cascade::cascade;
use gtk::prelude::*;

use crate::color::Rgb;
use crate::color_wheel::ColorWheel;

pub fn choose_color<W: IsA<gtk::Widget>>(w: &W, title: &'static str) -> Option<Rgb> {
    let color_wheel = ColorWheel::new();

    let vbox = cascade! {
        gtk::Box::new(gtk::Orientation::Vertical, 0);
        ..add(color_wheel.widget());
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
