use std::{env::current_exe, path::Path};

const SCHEMA: &str = "com.system76.keyboard-configurator";

fn settings_from_directory(directory: &Path) -> gio::Settings {
    let source = gio::SettingsSchemaSource::from_directory(directory, None, false).unwrap();
    let schema = source.lookup(SCHEMA, false).unwrap();
    gio::Settings::new_full::<gio::SettingsBackend>(&schema, None, None)
}

pub fn settings() -> gio::Settings {
    let mut exepath = current_exe().unwrap();

    if let Some(parent) = exepath.parent() {
        if parent.ends_with("target/debug") || parent.ends_with("target/release") {
            exepath.pop();
            exepath.push("../../data");
            return settings_from_directory(&exepath);
        }
    }

    if cfg!(target_os = "windows") {
        exepath.pop();
        settings_from_directory(&exepath)
    } else if cfg!(target_os = "macos") {
        exepath.pop();
        exepath.push("../Resources");
        settings_from_directory(&exepath)
    } else {
        // NOTE: for appimage, `linuxdeploy-plugin-gtk` sets
        // GSETTINGS_SCHEMA_DIR for us.
        gio::Settings::new(SCHEMA)
    }
}
