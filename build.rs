use std::process::Command;

fn main() {
    gio::compile_resources(
        "data",
        "data/com.system76.keyboardconfigurator.gresource.xml",
        "compiled.gresource",
    );

    println!("cargo:rerun-if-changed=data/com.system76.keyboardconfigurator.gschema.xml");
    let status = Command::new("glib-compile-schemas")
        .arg("data")
        .status()
        .unwrap();
    assert!(status.success());
}
