use gtk::{cairo, pango, prelude::*};
use std::f64::consts::PI;

/// The same rounded key face is used by the keyboard and quick-switch buttons.
pub fn rounded(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64) {
    let r = 4.0;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -0.5 * PI, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, 0.5 * PI);
    cr.arc(x + r, y + h - r, r, 0.5 * PI, PI);
    cr.arc(x + r, y + r, r, PI, 1.5 * PI);
    cr.close_path();
}

pub fn button(name: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_widget_name("quick-switch-keycap");
    button.set_tooltip_text(Some(name));
    button.set_size_request(112, 64);
    button.set_can_focus(true);
    let css = gtk::CssProvider::new();
    css.load_from_data(b"#quick-switch-keycap { padding: 0; margin: 0; border: 0; background: transparent; box-shadow: none; min-width: 0; min-height: 0; }").unwrap();
    button
        .style_context()
        .add_provider(&css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    let name = name.to_string();
    button.connect_draw(move |button, cr| {
        let width = button.allocated_width() as f64;
        let height = button.allocated_height() as f64;
        let state = button.state_flags();
        let bg = if state.contains(gtk::StateFlags::ACTIVE) {
            (0.45, 0.34, 0.25)
        } else if state.contains(gtk::StateFlags::PRELIGHT) {
            (0.48, 0.48, 0.48)
        } else {
            (0.38, 0.38, 0.38)
        };
        rounded(cr, 2.0, 2.0, width - 4.0, height - 4.0);
        cr.set_source_rgb(bg.0, bg.1, bg.2);
        cr.fill().unwrap();
        let layout = button.create_pango_layout(Some(&name));
        layout.set_alignment(pango::Alignment::Center);
        layout.set_width(((width - 12.0) * pango::SCALE as f64) as i32);
        layout.set_wrap(pango::WrapMode::WordChar);
        layout.set_height(-2);
        layout.set_ellipsize(pango::EllipsizeMode::End);
        cr.move_to(6.0, (height - layout.pixel_size().1 as f64) / 2.0);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        pangocairo::show_layout(cr, &layout);
        if button.has_focus() {
            rounded(cr, 3.0, 3.0, width - 6.0, height - 6.0);
            cr.set_source_rgb(0.98, 0.72, 0.42);
            cr.set_line_width(2.0);
            cr.stroke().unwrap();
        }
        gtk::glib::Propagation::Stop
    });
    button
}
