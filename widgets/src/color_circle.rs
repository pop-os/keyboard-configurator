use cascade::cascade;
use gtk::{cairo, glib, prelude::*, subclass::prelude::*};
use std::{cell::RefCell, collections::BTreeSet, f64::consts::PI};

use backend::Hs;

const BORDER: f64 = 1.;

#[derive(Default)]
pub struct ColorCircleInner {
    colors: RefCell<BTreeSet<Hs>>,
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
        let width = f64::from(widget.allocated_width());
        let height = f64::from(widget.allocated_height());

        let flags = widget.state_flags();

        let radius = width.min(height) / 2.;
        let alpha = if flags.contains(gtk::StateFlags::INSENSITIVE) {
            0.5
        } else {
            1.
        };

        let colors = self.colors.borrow();
        let total = colors.len() as f64;

        let mut angle1 = 0.;
        for hs in colors.iter() {
            let angle2 = angle1 + (2. * PI) / total;
            cr.move_to(radius, radius);
            cr.arc(radius, radius, radius - 2. * BORDER, angle1, angle2);
            cr.close_path();
            let (r, g, b) = hs.to_rgb().to_floats();
            cr.set_source_rgba(r, g, b, alpha);
            cr.fill().unwrap();
            angle1 = angle2;
        }

        cr.arc(radius, radius, radius - 2. * BORDER, 0., 2. * PI);
        if flags.contains(gtk::StateFlags::PRELIGHT) {
            cr.set_source_rgba(0., 0., 0., 0.2);
            cr.fill_preserve().unwrap();
        }
        cr.set_line_width(BORDER);
        cr.set_source_rgb(0.5, 0.5, 0.5);
        cr.stroke().unwrap();

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

    pub fn set_colors(&self, colors: BTreeSet<Hs>) {
        self.inner().colors.replace(colors);
        self.queue_draw();
    }
}
