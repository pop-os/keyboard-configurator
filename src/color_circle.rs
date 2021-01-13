use cascade::cascade;
use glib::clone;
use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::Cell;
use std::f64::consts::PI;
use std::ptr;

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
    drawing_area: gtk::DrawingArea,
    button: gtk::Button,
    rgb: Cell<Rgb>,
    alpha: Cell<f64>,
    symbol: Cell<ColorCircleSymbol>,
}

impl ObjectSubclass for ColorCircleInner {
    const NAME: &'static str = "S76ColorCircle";

    type ParentType = gtk::Bin;
    type Type = ColorCircle;

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn new() -> Self {
        let drawing_area = gtk::DrawingArea::new();

        let provider = cascade! {
            gtk::CssProvider::new();
            ..load_from_data(CSS).unwrap();
        };

        let button = cascade! {
            gtk::Button::new();
            ..get_style_context().add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
            ..get_style_context().add_class("circular");
            ..get_style_context().add_class("keyboard_color_button");
            ..add(&drawing_area);
        };

        Self {
            drawing_area,
            button,
            rgb: Cell::new(Rgb::new(0, 0, 0)),
            symbol: Cell::new(ColorCircleSymbol::None),
            alpha: Cell::new(1.),
        }
    }
}

impl ObjectImpl for ColorCircleInner {
    fn constructed(&self, obj: &ColorCircle) {
        self.parent_constructed(obj);

        obj.add(&self.button);
    }
}

impl WidgetImpl for ColorCircleInner {}
impl ContainerImpl for ColorCircleInner {}
impl BinImpl for ColorCircleInner {}

glib::wrapper! {
    pub struct ColorCircle(ObjectSubclass<ColorCircleInner>)
        @extends gtk::Bin, gtk::Container, gtk::Widget;
}

impl ColorCircle {
    pub fn new(size: i32) -> Self {
        let color_circle: Self = glib::Object::new(&[]).unwrap();

        color_circle.set_size_request(size, size);
        color_circle.connect_signals();

        color_circle
    }

    fn inner(&self) -> &ColorCircleInner {
        ColorCircleInner::from_instance(self)
    }

    fn connect_signals(&self) {
        let self_ = self;

        self.inner()
            .drawing_area
            .connect_draw(clone!(@strong self_ => move |w, cr| {
                self_.draw(w, cr);
                Inhibit(false)
            }));
    }

    // `arbitrary_self_types` feature would allow `self: &Rc<Self>`
    pub fn connect_clicked<F: Fn(&Self) + 'static>(&self, cb: F) {
        let self_ = self;
        self.inner()
            .button
            .connect_clicked(clone!(@weak self_ => @default-panic, move |_| cb(&self_)));
    }

    fn draw(&self, w: &gtk::DrawingArea, cr: &cairo::Context) {
        let width = f64::from(w.get_allocated_width());
        let height = f64::from(w.get_allocated_height());

        let radius = width.min(height) / 2.;
        let (r, g, b) = self.rgb().to_floats();
        let alpha = self.inner().alpha.get();

        cr.arc(radius, radius, radius, 0., 2. * PI);
        cr.set_source_rgba(r, g, b, alpha);
        cr.fill_preserve();

        cr.new_path();
        cr.set_source_rgb(0.25, 0.25, 0.25);
        cr.set_line_width(1.5);

        match self.inner().symbol.get() {
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
        self.inner().rgb.set(color);
        self.queue_draw();
    }

    pub fn rgb(&self) -> Rgb {
        self.inner().rgb.get()
    }

    pub fn set_symbol(&self, symbol: ColorCircleSymbol) {
        self.inner().symbol.set(symbol);
        self.queue_draw();
    }

    pub fn set_alpha(&self, alpha: f64) {
        self.inner().alpha.set(alpha);
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        ptr::eq(self.inner(), other.inner())
    }
}
