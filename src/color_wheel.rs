use cascade::cascade;
use gtk::prelude::*;
use palette::{Component, Srgb, IntoColor};
use std::cell::RefCell;
use std::rc::Rc;

pub fn color_wheel() -> gtk::Widget {
    let drawing_area = gtk::DrawingArea::new();

    let surface = Rc::new(RefCell::new(cairo::ImageSurface::create(cairo::Format::ARgb32, 0, 0).unwrap()));

    let surface_clone = surface.clone();
    drawing_area.connect_size_allocate(move |w, rect| {
        let size = rect.width.min(rect.height);
        let stride = cairo::Format::ARgb32.stride_for_width(size as u32).unwrap();
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

                let hsv = palette::Hsv::new(palette::RgbHue::from_radians(h), s, v);
                let rgb = hsv.into_rgb::<palette::encoding::srgb::Srgb>();
                let (mut r, mut g, mut b) = rgb.into_format::<u8>().into_components();

                let a = (radius - distance).max(0.).min(1.).convert::<u8>();

                let offset = (row * stride + col * 4) as usize;
                data[offset + 0] = b;
                data[offset + 1] = g;
                data[offset + 2] = r;
                data[offset + 3] = a;
            }
        }

        let image_surface = cairo::ImageSurface::create_for_data(data, cairo::Format::ARgb32, size, size, stride).unwrap();
        surface_clone.replace(image_surface);
    });

    let surface_clone = surface.clone();
    drawing_area.connect_draw(move |w, cr| {
        let width = f64::from(w.get_allocated_width());
        let height = f64::from(w.get_allocated_height());

        println!("{:?}", (width, height));
        cr.set_source_surface(&surface_clone.borrow(), 0., 0.);
        cr.paint();

        Inhibit(false)
    });

    let frame = cascade! {
        gtk::AspectFrame::new(None, 0., 0., 1., false);
        ..set_shadow_type(gtk::ShadowType::None);
        ..set_size_request(100, 100);
        ..add(&drawing_area);
    };

    frame.upcast()
}