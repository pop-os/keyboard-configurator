use cascade::cascade;
use gtk::prelude::*;

pub fn about_dialog() {
    let dialog = cascade! {
        gtk::AboutDialog::new();
        ..set_title("About Keyboard Configurator");
        ..set_program_name("System76 Keyboard Configurator");
        ..set_version(Some(env!("CARGO_PKG_VERSION")));
        ..set_license_type(gtk::License::Gpl30);
    };

    dialog.run();
    dialog.close();
}
