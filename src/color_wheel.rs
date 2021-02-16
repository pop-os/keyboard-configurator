// A hue/saturation color wheel that allows a color to be selected.

use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;

use crate::color::{Hs, Rgb};

pub struct ColorWheelInner {
    selected_hs: Cell<Hs>,
    surface: RefCell<cairo::ImageSurface>,
    hs_changed_handlers: RefCell<Vec<Box<dyn Fn(&ColorWheel) + 'static>>>,
}

impl ObjectSubclass for ColorWheelInner {
    const NAME: &'static str = "S76ColorWheel";

    type ParentType = gtk::DrawingArea;
    type Type = ColorWheel;
    type Interfaces = ();

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn new() -> Self {
        Self {
            selected_hs: Cell::new(Hs::new(0., 0.)),
            surface: RefCell::new(cairo::ImageSurface::create(cairo::Format::Rgb24, 0, 0).unwrap()),
            hs_changed_handlers: RefCell::new(Vec::new()),
        }
    }
}

impl ObjectImpl for ColorWheelInner {
    fn constructed(&self, wheel: &ColorWheel) {
        self.parent_constructed(wheel);

        wheel.add_events(gdk::EventMask::POINTER_MOTION_MASK | gdk::EventMask::BUTTON_PRESS_MASK);
    }
}

impl WidgetImpl for ColorWheelInner {
    fn draw(&self, wheel: &ColorWheel, cr: &cairo::Context) -> Inhibit {
        self.parent_draw(wheel, cr);

        let width = f64::from(wheel.get_allocated_width());
        let height = f64::from(wheel.get_allocated_height());

        let radius = width.min(height) / 2.;

        // Draw color wheel
        cr.set_source_surface(&self.surface.borrow(), 0., 0.);
        cr.arc(radius, radius, radius, 0., 2. * PI);
        cr.fill();

        // Draw selector circle
        let Hs { h, s } = wheel.hs();
        let x = radius + h.cos() * s * radius;
        let y = radius - h.sin() * s * radius;
        cr.arc(x, y, 7.5, 0., 2. * PI);
        cr.set_source_rgb(1., 1., 1.);
        cr.fill_preserve();
        cr.set_source_rgb(0., 0., 0.);
        cr.set_line_width(1.);
        cr.stroke();

        Inhibit(false)
    }

    fn size_allocate(&self, wheel: &ColorWheel, rect: &gdk::Rectangle) {
        self.parent_size_allocate(wheel, rect);
        wheel.generate_surface(rect);
    }

    fn button_press_event(&self, wheel: &ColorWheel, evt: &gdk::EventButton) -> Inhibit {
        wheel.mouse_select(evt.get_position());
        Inhibit(false)
    }

    fn motion_notify_event(&self, wheel: &ColorWheel, evt: &gdk::EventMotion) -> Inhibit {
        if evt.get_state().contains(gdk::ModifierType::BUTTON1_MASK) {
            wheel.mouse_select(evt.get_position());
        }
        Inhibit(false)
    }
}

impl DrawingAreaImpl for ColorWheelInner {}

glib::wrapper! {
    pub struct ColorWheel(ObjectSubclass<ColorWheelInner>)
        @extends gtk::DrawingArea, gtk::Widget;
}

impl ColorWheel {
    pub fn new() -> Self {
        glib::Object::new(&[]).unwrap()
    }

    fn inner(&self) -> &ColorWheelInner {
        ColorWheelInner::from_instance(self)
    }

    pub fn hs(&self) -> Hs {
        self.inner().selected_hs.get()
    }

    pub fn set_hs(&self, hs: Hs) {
        self.inner().selected_hs.set(hs);
        self.queue_draw();
        for handler in self.inner().hs_changed_handlers.borrow().iter() {
            handler(self);
        }
    }

    pub fn connect_hs_changed<F: Fn(&Self) + 'static>(&self, f: F) {
        self.inner()
            .hs_changed_handlers
            .borrow_mut()
            .push(std::boxed::Box::new(f) as Box<dyn Fn(&Self)>);
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

                let Rgb { r, g, b } = Hs::new(angle, distance / radius).to_rgb();

                let offset = (row * stride + col * 4) as usize;
                data[offset] = b;
                data[offset + 1] = g;
                data[offset + 2] = r;
            }
        }

        let image_surface =
            cairo::ImageSurface::create_for_data(data, cairo::Format::Rgb24, size, size, stride)
                .unwrap();
        self.inner().surface.replace(image_surface);
    }

    fn mouse_select(&self, pos: (f64, f64)) {
        let width = f64::from(self.get_allocated_width());
        let height = f64::from(self.get_allocated_height());

        let radius = width.min(height) / 2.;
        let (x, y) = (pos.0 - radius, radius - pos.1);

        let angle = y.atan2(x);
        let distance = y.hypot(x);

        self.set_hs(Hs::new(angle, (distance / radius).min(1.)));
    }
}
