use crate::fl;
use gtk::prelude::*;

pub fn shortcuts_window() -> gtk::ShortcutsWindow {
    // GtkShortcutWindow docs specifically say it should be used from
    // GtkBuilder, and lacks things like a new function.

    let xml = include_str!("shortcuts_window.ui");
    let builder = gtk::Builder::from_string(xml);

    let import: gtk::ShortcutsShortcut = builder.object("import-layout").unwrap();
    import.set_title(Some(&fl!("layout-import")));

    let export: gtk::ShortcutsShortcut = builder.object("export-layout").unwrap();
    export.set_title(Some(&fl!("layout-export")));

    builder.object("shortcuts-window").unwrap()
}
