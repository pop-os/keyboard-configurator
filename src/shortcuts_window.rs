use crate::fl;
use gtk::prelude::*;

pub fn shortcuts_window() -> gtk::ShortcutsWindow {
    // GtkShortcutWindow docs specifically say it should be used from
    // GtkBuilder, and lacks things like a new function.

    let xml = include_str!("shortcuts_window.ui");
    let builder = gtk::Builder::from_string(xml);

    let import: gtk::ShortcutsShortcut = builder.get_object("import-layout").unwrap();
    import.set_property_title(Some(&fl!("layout-import")));

    let export: gtk::ShortcutsShortcut = builder.get_object("export-layout").unwrap();
    export.set_property_title(Some(&fl!("layout-export")));

    builder.get_object("shortcuts-window").unwrap()
}
