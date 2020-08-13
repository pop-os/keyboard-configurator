use cascade::cascade;
use gtk::prelude::*;
use palette::{RgbHue, Hsv, IntoColor};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::f64::consts::PI;

pub fn color_wheel() -> gtk::Widget {
    let drawing_area = cascade! {
        gtk::DrawingArea::new();
        ..add_events(gdk::EventMask::POINTER_MOTION_MASK | gdk::EventMask::BUTTON_PRESS_MASK);
    };

    let selected_hs = Rc::new(Cell::new((0., 0.)));

    let selected_hs_clone = selected_hs.clone();
    drawing_area.connect_button_press_event(move |w, evt| {
        let width = f64::from(w.get_allocated_width());
        let height = f64::from(w.get_allocated_height());

        let radius = width.min(height) / 2.;
        let pos = evt.get_position();
        let (x, y) = (pos.0 - radius, radius - pos.1);

        let angle = y.atan2(x);
        let distance = (x.powi(2) + y.powi(2)).sqrt();

        selected_hs_clone.set((angle, (distance / radius).min(1.)));
        w.queue_draw();

        Inhibit(false)
    });

    let selected_hs_clone = selected_hs.clone();
    drawing_area.connect_motion_notify_event(move |w, evt| {
        if evt.get_state().contains(gdk::ModifierType::BUTTON1_MASK) {
            let width = f64::from(w.get_allocated_width());
            let height = f64::from(w.get_allocated_height());

            let radius = width.min(height) / 2.;
            let pos = evt.get_position();
            let (x, y) = (pos.0 - radius, radius - pos.1);

            let angle = y.atan2(x);
            let distance = (x.powi(2) + y.powi(2)).sqrt();

            selected_hs_clone.set((angle, (distance / radius).min(1.)));
            w.queue_draw();
        }
        Inhibit(false)
    });

    let surface = Rc::new(RefCell::new(cairo::ImageSurface::create(cairo::Format::Rgb24, 0, 0).unwrap()));

    let surface_clone = surface.clone();
    drawing_area.connect_size_allocate(move |w, rect| {
        let size = rect.width.min(rect.height);
        let stride = cairo::Format::Rgb24.stride_for_width(size as u32).unwrap();
        let mut data = vec![0; (size * stride) as usize];

        for row in 0..size {
            for col in 0..size {
                let radius = size as f64 / 2.;
                let (x, y) = (col as f64 - radius, radius - row as f64);

                let angle = y.atan2(x);
                let distance = (x.powi(2) + y.powi(2)).sqrt();

                let h = angle;
                let s = distance / radius;
                let v = 1.;

                let hsv = Hsv::new(RgbHue::from_radians(h), s, v);
                let rgb = hsv.into_rgb::<palette::encoding::srgb::Srgb>();
                let (r, g, b) = rgb.into_format::<u8>().into_components();

                let offset = (row * stride + col * 4) as usize;
                data[offset + 0] = b;
                data[offset + 1] = g;
                data[offset + 2] = r;
            }
        }

        let image_surface = cairo::ImageSurface::create_for_data(data, cairo::Format::Rgb24, size, size, stride).unwrap();
        surface_clone.replace(image_surface);
    });

    let surface_clone = surface.clone();
    let selected_hs_clone = selected_hs.clone();
    drawing_area.connect_draw(move |w, cr| {
        let width = f64::from(w.get_allocated_width());
        let height = f64::from(w.get_allocated_height());

        let radius = width.min(height) / 2.;

        cr.set_source_surface(&surface_clone.borrow(), 0., 0.);
        cr.arc(radius, radius, radius, 0., 2. * PI);
        cr.fill();

        let (h, s) = selected_hs_clone.get();
        let x = radius + h.cos() * s * radius;
        let y = radius - h.sin() * s * radius;
        cr.arc(x, y, 7.5, 0., 2. * PI);
        cr.set_source_rgb(1., 1., 1.);
        cr.fill_preserve();
        cr.set_source_rgb(0., 0., 0.);
        cr.set_line_width(1.);
        cr.stroke();

        Inhibit(false)
    });

    let frame = cascade! {
        gtk::AspectFrame::new(None, 0., 0., 1., false);
        ..set_shadow_type(gtk::ShadowType::None);
        ..set_size_request(500, 500);
        ..add(&drawing_area);
    };

    frame.upcast()
}