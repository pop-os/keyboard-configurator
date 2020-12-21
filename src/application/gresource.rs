pub fn init() -> Result<(), glib::Error> {
    const GRESOURCE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/compiled.gresource"));

    gio::resources_register(&gio::Resource::from_data(&glib::Bytes::from_static(GRESOURCE))?);

    Ok(())
}
