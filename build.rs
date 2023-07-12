fn main() {
    glib_build_tools::compile_resources(
        &["data"],
        "data/com.system76.keyboardconfigurator.gresource.xml",
        "compiled.gresource",
    );
}
