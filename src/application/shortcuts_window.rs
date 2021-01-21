use gtk::prelude::*;

pub fn shortcuts_window() -> gtk::ShortcutsWindow {
    // GtkShortcutWindow docs specifically say it should be used from
    // GtkBuilder, and lacks things like a new function.

    let xml = include_str!("shortcuts_window.ui");
    let builder = gtk::Builder::from_string(xml);
    builder.get_object("shortcuts-window").unwrap()
}
