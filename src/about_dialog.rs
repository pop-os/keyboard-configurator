use crate::fl;
use cascade::cascade;
use gtk::prelude::*;

pub fn show_about_dialog() {
    cascade! {
        gtk::AboutDialog::new();
        ..set_titlebar(Some(&cascade! {
            gtk::HeaderBar::new();
            ..set_show_close_button(true);
            ..show();
        }));
        ..set_title(&fl!("app-about"));
        ..set_program_name(&fl!("app-title"));
        ..set_version(Some(concat!(env!("CARGO_PKG_VERSION"), "-beta1")));
        ..set_license_type(gtk::License::Gpl30);
        ..set_logo_icon_name(Some("com.system76.keyboardconfigurator"));
        ..connect_response(|dialog, _| dialog.close());
        ..show();
    };
}
