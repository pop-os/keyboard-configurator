use cascade::cascade;
use gtk::prelude::*;
use std::fmt::Display;

pub fn show_error_dialog<W: IsA<gtk::Window>, E: Display>(parent: &W, title: &str, err: E) {
    let label = cascade! {
        gtk::Label::new(Some(&format!("<b>{}</b>:\n{}", title, err)));
        ..set_use_markup(true);
        ..show();
    };

    let dialog = cascade! {
        gtk::Dialog::with_buttons(Some(title), Some(parent), gtk::DialogFlags::MODAL | gtk::DialogFlags::USE_HEADER_BAR, &[("Ok", gtk::ResponseType::Ok)]);
        ..connect_response(|dialog, _| dialog.close());
    };

    let header = dialog.header_bar().unwrap();
    header.set_show_close_button(false);

    let content = dialog.content_area();
    content.add(&label);
    content.set_margin(24);

    dialog.show();
}
