fn main() {
    gio::compile_resources(
        "data",
        "data/com.system76.keyboardconfigurator.gresource.xml",
        "compiled.gresource",
    );
}
