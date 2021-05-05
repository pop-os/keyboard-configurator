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
        ..set_title("About Keyboard Configurator");
        ..set_program_name("System76 Keyboard Configurator");
        ..set_version(Some(env!("CARGO_PKG_VERSION")));
        ..set_license_type(gtk::License::Gpl30);
        ..set_logo_icon_name(Some("com.system76.keyboardconfigurator"));
        ..connect_response(|dialog, _| dialog.close());
        ..show();
    };
}
