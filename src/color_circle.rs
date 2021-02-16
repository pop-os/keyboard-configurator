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

#[derive(Default)]
pub struct ColorCircleInner {
    drawing_area: gtk::DrawingArea,
    rgb: Cell<Rgb>,
    alpha: Cell<f64>,
    symbol: Cell<&'static str>,
}

impl ObjectSubclass for ColorCircleInner {
    const NAME: &'static str = "S76ColorCircle";

    type ParentType = gtk::Button;
    type Type = ColorCircle;
    type Interfaces = ();

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn new() -> Self {
        Self {
            alpha: Cell::new(1.),
            ..Default::default()
        }
    }
}

impl ObjectImpl for ColorCircleInner {
    fn constructed(&self, obj: &ColorCircle) {
        self.parent_constructed(obj);

        let provider = cascade! {
            gtk::CssProvider::new();
            ..load_from_data(CSS).unwrap();
        };

        let context = obj.get_style_context();
        context.add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        context.add_class("circular");
        context.add_class("keyboard_color_button");

        self.drawing_area
            .connect_draw(clone!(@weak obj => @default-panic, move |w, cr| {
                obj.draw(w, cr);
                Inhibit(false)
            }));

        obj.add(&self.drawing_area);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpec::boxed(
                "rgb",
                "rgb",
                "rgb",
                Rgb::get_type(),
                glib::ParamFlags::READWRITE,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        widget: &ColorCircle,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.get_name() {
            "rgb" => {
                let rgb: &Rgb = value.get_some().unwrap();
                widget.set_rgb(*rgb);
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(
        &self,
        widget: &ColorCircle,
        _id: usize,
        pspec: &glib::ParamSpec,
    ) -> glib::Value {
        match pspec.get_name() {
            "rgb" => widget.rgb().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for ColorCircleInner {}
impl ContainerImpl for ColorCircleInner {}
impl BinImpl for ColorCircleInner {}
impl ButtonImpl for ColorCircleInner {}

glib::wrapper! {
    pub struct ColorCircle(ObjectSubclass<ColorCircleInner>)
        @extends gtk::Button, gtk::Bin, gtk::Container, gtk::Widget;
}

impl ColorCircle {
    pub fn new(size: i32) -> Self {
        let color_circle: Self = glib::Object::new(&[]).unwrap();

        color_circle.set_size_request(size, size);

        color_circle
    }

    fn inner(&self) -> &ColorCircleInner {
        ColorCircleInner::from_instance(self)
    }

    fn draw(&self, w: &gtk::DrawingArea, cr: &cairo::Context) {
        let width = f64::from(w.get_allocated_width());
        let height = f64::from(w.get_allocated_height());

        let style = w.get_style_context();
        let fg = style.get_color(gtk::StateFlags::NORMAL);

        let radius = width.min(height) / 2.;
        let (r, g, b) = self.rgb().to_floats();
        let alpha = self.inner().alpha.get();

        cr.arc(radius, radius, radius, 0., 2. * PI);
        cr.set_source_rgba(r, g, b, alpha);
        cr.fill_preserve();

        let text = self.inner().symbol.get();
        let attrs = cascade! {
            pango::AttrList::new();
            ..insert(pango::Attribute::new_size(14 * pango::SCALE));
        };
        let layout = cascade! {
            w.create_pango_layout(Some(text));
            ..set_width((width * pango::SCALE as f64) as i32);
            ..set_alignment(pango::Alignment::Center);
            ..set_attributes(Some(&attrs));
        };
        let text_height = layout.get_pixel_size().1 as f64;
        cr.new_path();
        cr.move_to(0., (height - text_height) / 2.);
        cr.set_source_rgb(fg.red, fg.green, fg.blue);
        pangocairo::show_layout(cr, &layout);

        cr.stroke();
    }

    pub fn set_rgb(&self, color: Rgb) {
        self.inner().rgb.set(color);
        self.notify("rgb");
        self.queue_draw();
    }

    pub fn rgb(&self) -> Rgb {
        self.inner().rgb.get()
    }

    pub fn set_symbol(&self, symbol: &'static str) {
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
