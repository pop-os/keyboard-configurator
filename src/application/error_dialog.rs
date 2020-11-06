use cascade::cascade;
use gtk::prelude::*;
use std::fmt::Display;

pub fn error_dialog<W: IsA<gtk::Window>, E: Display>(parent: &W, title: &str, err: E) {
    let label = cascade! {
        gtk::Label::new(Some(&format!("<b>{}</b>:\n{}", title, err)));
        ..set_use_markup(true);
        ..show();
    };

    let dialog = cascade! {
        gtk::Dialog::with_buttons(Some(title), Some(parent), gtk::DialogFlags::MODAL | gtk::DialogFlags::USE_HEADER_BAR, &[("Ok", gtk::ResponseType::Ok)]);
    };

    let header = dialog.get_header_bar().unwrap();
    header.set_show_close_button(false);

    let content = dialog.get_content_area();
    content.add(&label);
    content.set_property_margin(24);

    dialog.run();
    dialog.close();
}
