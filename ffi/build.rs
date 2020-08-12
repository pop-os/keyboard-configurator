use std::{env, fs::File, io::Write, path::PathBuf};

fn main() {
    let target_dir = PathBuf::from("../target");

    let pkg_config = format!(
        include_str!("pop_keyboard_backlight.pc.in"),
        name = "pop_keyboard_backlight",
        description = env::var("CARGO_PKG_DESCRIPTION").unwrap(),
        version = env::var("CARGO_PKG_VERSION").unwrap()
    );

    File::create(target_dir.join("pop_keyboard_backlight.pc.stub"))
        .expect("failed to create pc.stub")
        .write_all(&pkg_config.as_bytes())
        .expect("failed to write pc.stub");
}
