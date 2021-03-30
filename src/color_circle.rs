use cascade::cascade;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::Cell;
use std::f64::consts::PI;
use std::ptr;

use daemon::Hs;

const BORDER: f64 = 1.;

#[derive(Default)]
pub struct ColorCircleInner {
    hs: Cell<Hs>,
}

#[glib::object_subclass]
impl ObjectSubclass for ColorCircleInner {
    const NAME: &'static str = "S76ColorCircle";
    type ParentType = gtk::Button;
    type Type = ColorCircle;
}

impl ObjectImpl for ColorCircleInner {}

impl WidgetImpl for ColorCircleInner {
    fn draw(&self, widget: &ColorCircle, cr: &cairo::Context) -> Inhibit {
        let width = f64::from(widget.get_allocated_width());
        let height = f64::from(widget.get_allocated_height());

        let flags = widget.get_state_flags();

        let radius = width.min(height) / 2.;
        let (r, g, b) = widget.hs().to_rgb().to_floats();
        let alpha = if flags.contains(gtk::StateFlags::INSENSITIVE) {
            0.5
        } else {
            1.
        };

        cr.arc(radius, radius, radius - 2. * BORDER, 0., 2. * PI);
        cr.set_source_rgba(r, g, b, alpha);
        cr.fill_preserve();
        if flags.contains(gtk::StateFlags::PRELIGHT) {
            cr.set_source_rgba(0., 0., 0., 0.2);
            cr.fill_preserve();
        }
        cr.set_line_width(BORDER);
        cr.set_source_rgb(0.5, 0.5, 0.5);
        cr.stroke();

        Inhibit(false)
    }
}

impl ContainerImpl for ColorCircleInner {}
impl BinImpl for ColorCircleInner {}
impl ButtonImpl for ColorCircleInner {}

glib::wrapper! {
    pub struct ColorCircle(ObjectSubclass<ColorCircleInner>)
        @extends gtk::Button, gtk::Bin, gtk::Container, gtk::Widget;
}

impl ColorCircle {
    pub fn new(size: i32) -> Self {
        cascade! {
            glib::Object::new::<Self>(&[]).unwrap();
            ..set_size_request(size, size);
        }
    }

    fn inner(&self) -> &ColorCircleInner {
        ColorCircleInner::from_instance(self)
    }

    pub fn set_hs(&self, color: Hs) {
        self.inner().hs.set(color);
        self.queue_draw();
    }

    fn hs(&self) -> Hs {
        self.inner().hs.get()
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        ptr::eq(self.inner(), other.inner())
    }
}
