// A hue/saturation color wheel that allows a color to be selected.

use glib::subclass;
use glib::subclass::prelude::*;
use glib::translate::{FromGlibPtrFull, ToGlib, ToGlibPtr};
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

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn new() -> Self {
        Self {
            selected_hs: Cell::new(Hs::new(0., 0.)),
            surface: RefCell::new(cairo::ImageSurface::create(cairo::Format::Rgb24, 0, 0).unwrap()),
            hs_changed_handlers: RefCell::new(Vec::new()),
        }
    }
}

impl ObjectImpl for ColorWheelInner {
    glib_object_impl!();
}
impl WidgetImpl for ColorWheelInner {}
impl DrawingAreaImpl for ColorWheelInner {}

glib_wrapper! {
    pub struct ColorWheel(
        Object<subclass::simple::InstanceStruct<ColorWheelInner>,
        subclass::simple::ClassStruct<ColorWheelInner>, ColorWheelClass>)
        @extends gtk::DrawingArea, gtk::Widget;

    match fn {
        get_type => || ColorWheelInner::get_type().to_glib(),
    }
}

impl ColorWheel {
    pub fn new() -> Self {
        let wheel: Self = glib::Object::new(Self::static_type(), &[])
            .unwrap()
            .downcast()
            .unwrap();

        wheel.set_size_request(500, 500);
        wheel.add_events(gdk::EventMask::POINTER_MOTION_MASK | gdk::EventMask::BUTTON_PRESS_MASK);
        wheel.connect_signals();

        wheel
    }

    fn inner(&self) -> &ColorWheelInner {
        ColorWheelInner::from_instance(self)
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.upcast_ref()
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

    fn connect_signals(&self) {
        self.connect_draw(|self_, cr| {
            self_.draw(cr);
            Inhibit(false)
        });

        self.connect_size_allocate(|self_, rect| {
            self_.generate_surface(rect);
        });

        self.connect_button_press_event(|self_, evt| {
            self_.mouse_select(evt.get_position());
            Inhibit(false)
        });

        self.connect_motion_notify_event(|self_, evt| {
            if evt.get_state().contains(gdk::ModifierType::BUTTON1_MASK) {
                self_.mouse_select(evt.get_position());
            }
            Inhibit(false)
        });
    }

    fn draw(&self, cr: &cairo::Context) {
        let width = f64::from(self.get_allocated_width());
        let height = f64::from(self.get_allocated_height());

        let radius = width.min(height) / 2.;

        // Draw color wheel
        cr.set_source_surface(&self.inner().surface.borrow(), 0., 0.);
        cr.arc(radius, radius, radius, 0., 2. * PI);
        cr.fill();

        // Draw selector circle
        let Hs { h, s } = self.hs();
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

                let Rgb { r, g, b } = Hs::new(angle, distance / radius).to_rgb();

                let offset = (row * stride + col * 4) as usize;
                data[offset + 0] = b;
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
