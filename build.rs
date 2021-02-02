fn main() {
    gio::compile_resources(
        "data",
        "data/com.system76.keyboard-configurator.gresource.xml",
        "compiled.gresource",
    );
}
