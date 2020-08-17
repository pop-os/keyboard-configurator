use cascade::cascade;
use glib::clone;
use glib::clone::{Downgrade, Upgrade};
use gtk::prelude::*;
use std::cell::Cell;
use std::f64::consts::PI;
use std::ptr;
use std::rc::{Rc, Weak};

use crate::color::Rgb;

// The standard "circular" class includes padding, so disable that
const CSS: &[u8] = b"
    button.keyboard_color_button {
        padding: 0px;
    }
";

#[derive(Clone, Copy)]
pub enum ColorCircleSymbol {
    Check,
    Plus,
    None,
}

pub struct ColorCircleInner {
    frame: gtk::AspectFrame,
    drawing_area: gtk::DrawingArea,
    button: gtk::Button,
    rgb: Cell<Rgb>,
    alpha: Cell<f64>,
    symbol: Cell<ColorCircleSymbol>,
}

#[derive(Clone)]
pub struct ColorCircle(Rc<ColorCircleInner>);

pub struct ColorCircleWeak(Weak<ColorCircleInner>);

impl Downgrade for ColorCircle {
    type Weak = ColorCircleWeak;

    fn downgrade(&self) -> Self::Weak {
        ColorCircleWeak(self.0.downgrade())
    }
}

impl Upgrade for ColorCircleWeak {
    type Strong = ColorCircle;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(ColorCircle)
    }
}

impl ColorCircle {
    pub fn new(size: i32) -> Self {
        let drawing_area = gtk::DrawingArea::new();

        let provider = cascade! {
            gtk::CssProvider::new();
            ..load_from_data(CSS).unwrap();
        };

        let button = cascade! {
            gtk::Button::new();
            ..get_style_context().add_provider(&provider, gtk_sys::GTK_STYLE_PROVIDER_PRIORITY_APPLICATION as u32);
            ..get_style_context().add_class("circular");
            ..get_style_context().add_class("keyboard_color_button");
            ..add(&drawing_area);
        };

        let frame = cascade! {
            gtk::AspectFrame::new(None, 0., 0., 1., false);
            ..set_shadow_type(gtk::ShadowType::None);
            ..set_size_request(size, size);
            ..add(&button);
        };

        let color_circle = Self(Rc::new(ColorCircleInner {
            frame,
            drawing_area,
            button: button.clone(),
            rgb: Cell::new(Rgb::new(0, 0, 0)),
            symbol: Cell::new(ColorCircleSymbol::None),
            alpha: Cell::new(1.),
        }));

        color_circle.connect_signals();

        color_circle
    }

    fn connect_signals(&self) {
        let self_ = self;

        self.0
            .drawing_area
            .connect_draw(clone!(@strong self_ => move |w, cr| {
                self_.draw(w, cr);
                Inhibit(false)
            }));
    }

    // `arbitrary_self_types` feature would allow `self: &Rc<Self>`
    pub fn connect_clicked<F: Fn(&Self) + 'static>(&self, cb: F) {
        let self_ = self;
        self.0
            .button
            .connect_clicked(clone!(@weak self_ => @default-panic, move |_| cb(&self_)));
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.0.frame.upcast_ref::<gtk::Widget>()
    }

    fn draw(&self, w: &gtk::DrawingArea, cr: &cairo::Context) {
        let width = f64::from(w.get_allocated_width());
        let height = f64::from(w.get_allocated_height());

        let radius = width.min(height) / 2.;
        let (r, g, b) = self.rgb().to_floats();
        let alpha = self.0.alpha.get();

        cr.arc(radius, radius, radius, 0., 2. * PI);
        cr.set_source_rgba(r, g, b, alpha);
        cr.fill_preserve();

        cr.new_path();
        cr.set_source_rgb(0.25, 0.25, 0.25);
        cr.set_line_width(1.5);

        match self.0.symbol.get() {
            ColorCircleSymbol::Plus => {
                cr.move_to(radius, 0.8 * radius);
                cr.line_to(radius, 1.2 * radius);
                cr.move_to(0.8 * radius, radius);
                cr.line_to(1.2 * radius, radius);
            }
            ColorCircleSymbol::Check => {
                cr.move_to(0.8 * radius, radius);
                cr.line_to(radius, 1.2 * radius);
                cr.line_to(1.2 * radius, 0.8 * radius);
            }
            ColorCircleSymbol::None => {}
        }

        cr.stroke();
    }

    pub fn set_rgb(&self, color: Rgb) {
        self.0.rgb.set(color);
        self.widget().queue_draw();
    }

    pub fn rgb(&self) -> Rgb {
        self.0.rgb.get()
    }

    pub fn set_symbol(&self, symbol: ColorCircleSymbol) {
        self.0.symbol.set(symbol);
        self.widget().queue_draw();
    }

    pub fn set_alpha(&self, alpha: f64) {
        self.0.alpha.set(alpha);
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        ptr::eq(self.0.as_ref(), other.0.as_ref())
    }
}
