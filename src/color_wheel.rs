// A hue/saturation color wheel that allows a color to be selected.

use cascade::cascade;
use gtk::prelude::*;
use palette::{RgbHue, IntoColor};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::f64::consts::PI;

#[derive(Clone, Copy)]
struct Hs {
    /// Hue, in radians
    h: f64,
    /// Saturation, from 0.0 to 1.0
    s: f64,
}

impl Hs {
    fn new(h: f64, s: f64) -> Self {
        Self { h, s }
    }

    fn to_rgb(self) -> Rgb {
        let hue = RgbHue::from_radians(self.h);
        let hsv = palette::Hsv::new(hue, self.s, 1.);
        let rgb = hsv.into_rgb::<palette::encoding::srgb::Srgb>();
        let (r, g, b) = rgb.into_format::<u8>().into_components();
        Rgb::new(r, g, b)
    }
}

#[derive(Clone, Copy)]
struct Rgb {
    /// Red
    r: u8,
    /// Green
    g: u8,
    /// Blue
    b: u8,
}

impl Rgb {
    fn new(r: u8, g: u8, b: u8) -> Self {
        Self {r, g, b}
    }
}

struct ColorWheelInner {
    selected_hs: Cell<Hs>,
    surface: RefCell<cairo::ImageSurface>,
    drawing_area: gtk::DrawingArea,
    frame: gtk::AspectFrame,
}

#[derive(Clone)]
struct ColorWheel(Rc<ColorWheelInner>);

impl ColorWheel {
    fn new() -> Self {
        let drawing_area = cascade! {
            gtk::DrawingArea::new();
            ..add_events(gdk::EventMask::POINTER_MOTION_MASK | gdk::EventMask::BUTTON_PRESS_MASK);
        };

        let frame = cascade! {
            gtk::AspectFrame::new(None, 0., 0., 1., false);
            ..set_shadow_type(gtk::ShadowType::None);
            ..set_size_request(500, 500);
            ..add(&drawing_area);
        };

        let wheel = Self(Rc::new(ColorWheelInner {
            selected_hs: Cell::new(Hs::new(0., 0.)),
            surface: RefCell::new(cairo::ImageSurface::create(cairo::Format::Rgb24, 0, 0).unwrap()),
            drawing_area,
            frame,
        }));

        wheel.connect_signals();

        wheel
    }

    fn widget(&self) -> &gtk::Widget {
        self.0.frame.upcast_ref()
    }

    fn connect_signals(&self) {
        let self_clone = self.clone();
        self.0.drawing_area.connect_draw(move |w, cr| {
            self_clone.draw(w, cr);
            Inhibit(false)
        });

        let self_clone = self.clone();
        self.0.drawing_area.connect_size_allocate(move |w, rect| {
            self_clone.generate_surface(rect);
        });

        let self_clone = self.clone();
        self.0.drawing_area.connect_button_press_event(move |w, evt| {
            self_clone.mouse_select(w, evt.get_position());
            Inhibit(false)
        });

        let self_clone = self.clone();
        self.0.drawing_area.connect_motion_notify_event(move |w, evt| {
            if evt.get_state().contains(gdk::ModifierType::BUTTON1_MASK) {
                self_clone.mouse_select(w, evt.get_position());
            }
            Inhibit(false)
        });
    }

    fn draw(&self, w: &gtk::DrawingArea, cr: &cairo::Context) {
        let width = f64::from(w.get_allocated_width());
        let height = f64::from(w.get_allocated_height());

        let radius = width.min(height) / 2.;

        cr.set_source_surface(&self.0.surface.borrow(), 0., 0.);
        cr.arc(radius, radius, radius, 0., 2. * PI);
        cr.fill();

        let Hs {h, s} = self.0.selected_hs.get();
        let x = radius + h.cos() * s * radius;
        let y = radius - h.sin() * s * radius;
        cr.arc(x, y, 7.5, 0., 2. * PI);
        cr.set_source_rgb(1., 1., 1.);
        cr.fill_preserve();
        cr.set_source_rgb(0., 0., 0.);
        cr.set_line_width(1.);
        cr.stroke();
    }

    fn generate_surface(&self, rect: &gtk::Rectangle) {
        let size = rect.width.min(rect.height);
        let stride = cairo::Format::Rgb24.stride_for_width(size as u32).unwrap();
        let mut data = vec![0; (size * stride) as usize];

        for row in 0..size {
            for col in 0..size {
                let radius = size as f64 / 2.;
                let (x, y) = (col as f64 - radius, radius - row as f64);

                let angle = y.atan2(x);
                let distance = y.hypot(x);

                let Rgb {r, g, b} = Hs::new(angle, distance / radius).to_rgb();

                let offset = (row * stride + col * 4) as usize;
                data[offset + 0] = b;
                data[offset + 1] = g;
                data[offset + 2] = r;
            }
        }

        let image_surface = cairo::ImageSurface::create_for_data(data, cairo::Format::Rgb24, size, size, stride).unwrap();
        self.0.surface.replace(image_surface);
    }

    fn mouse_select(&self, w: &gtk::DrawingArea, pos: (f64, f64)) {
        let width = f64::from(w.get_allocated_width());
        let height = f64::from(w.get_allocated_height());

        let radius = width.min(height) / 2.;
        let (x, y) = (pos.0 - radius, radius - pos.1);

        let angle = y.atan2(x);
        let distance = y.hypot(x);

        self.0.selected_hs.set(Hs::new(angle, (distance / radius).min(1.)));

        w.queue_draw();
    }
}

pub fn color_wheel() -> gtk::Widget {
    ColorWheel::new().widget().clone()
}