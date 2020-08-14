// A hue/saturation color wheel that allows a color to be selected.

use cascade::cascade;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::f64::consts::PI;

use crate::color::{Rgb, Hs};

struct ColorWheelInner {
    selected_hs: Cell<Hs>,
    surface: RefCell<cairo::ImageSurface>,
    drawing_area: gtk::DrawingArea,
    frame: gtk::AspectFrame,
    hs_changed_handlers: RefCell<Vec<Box<dyn Fn(&ColorWheel) + 'static>>>,
}

#[derive(Clone)]
pub struct ColorWheel(Rc<ColorWheelInner>);

impl ColorWheel {
    pub fn new() -> Self {
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
            hs_changed_handlers: RefCell::new(Vec::new()),
        }));

        wheel.connect_signals();

        wheel
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.0.frame.upcast_ref()
    }

    pub fn hs(&self) -> Hs {
        self.0.selected_hs.get()
    }

    pub fn set_hs(&self, hs: Hs) {
        self.0.selected_hs.set(hs);
        self.0.drawing_area.queue_draw();
        for handler in self.0.hs_changed_handlers.borrow().iter() {
            handler(self);
        }
    }

    pub fn connect_hs_changed<F: Fn(&Self) + 'static>(&self, f: F) {
        self.0.hs_changed_handlers.borrow_mut().push(std::boxed::Box::new(f) as Box<dyn Fn(&Self)>);
    }

    fn connect_signals(&self) {
        let self_clone = self.clone();
        self.0.drawing_area.connect_draw(move |w, cr| {
            self_clone.draw(w, cr);
            Inhibit(false)
        });

        let self_clone = self.clone();
        self.0.drawing_area.connect_size_allocate(move |_w, rect| {
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

        let Hs {h, s} = self.hs();
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

        self.set_hs(Hs::new(angle, (distance / radius).min(1.)));
    }
}