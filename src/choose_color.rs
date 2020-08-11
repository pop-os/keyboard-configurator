use gtk::prelude::*;

pub fn choose_color<W: IsA<gtk::Widget>>(w: &W) -> Option<(f64, f64, f64)> {
    let window = w.get_toplevel().and_then(|x| x.downcast::<gtk::Window>().ok());
    let color_dialog = gtk::ColorChooserDialog::new(None, window.as_ref());
    let response = color_dialog.run();
    let rgba = color_dialog.get_rgba();
    color_dialog.destroy();

    if response == gtk::ResponseType::Ok {
        Some((rgba.red, rgba.green, rgba.blue))
    } else {
        None
    }
}