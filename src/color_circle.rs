use cascade::cascade;
use gtk::prelude::*;
use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;

// The standard "circular" class includes padding, so disable that
const CSS: &[u8] = b"
    button.keyboard_color_button {
        padding: 0px;
    }
";

pub struct ColorCircle {
    frame: gtk::AspectFrame,
    button: gtk::Button,
    rgb: Cell<(f64, f64, f64)>,
}

impl ColorCircle {
    pub fn new(size: i32) -> Rc<Self> {
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

        let color_circle = Rc::new(Self {
            frame,
            button: button.clone(),
            rgb: Cell::new((0., 0., 0.)),
        });

        let color_circle_clone = color_circle.clone();
        drawing_area.connect_draw(move |w, cr| {
            color_circle_clone.draw(w, cr);
            Inhibit(false)
        });

        color_circle
    }

    // `arbitrary_self_types` feature would allow `self: &Rc<Self>`
    pub fn connect_clicked<F: Fn(&Rc<Self>) + 'static>(self: Rc<Self>, cb: F) {
        let self_clone = self.clone();
        self.button.connect_clicked(move |_| cb(&self_clone));
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.frame.upcast_ref::<gtk::Widget>()
    }

    fn draw(&self, w: &gtk::DrawingArea, cr: &cairo::Context) {
        let width = f64::from(w.get_allocated_width());
        let height = f64::from(w.get_allocated_height());

        let radius = width.min(height) / 2.;
        let rgb = self.rgb();

        cr.arc(radius, radius, radius, 0., 2. * PI);
        cr.set_source_rgb(rgb.0, rgb.1, rgb.2);
        cr.fill_preserve();

        /*
        cr.new_path();
        cr.set_source_rgb(0., 0., 0.);
        cr.set_font_size(radius);
        let extents = cr.text_extents("+");
        cr.translate(radius - extents.width / 2., radius * 2. - extents.height);
        cr.show_text("+");
        cr.stroke();
        */
    }

    pub fn set_rgb(&self, color: (f64, f64, f64)) {
        self.rgb.set(color);
        self.widget().queue_draw();
    }

    pub fn rgb(&self) -> (f64, f64, f64) {
        self.rgb.get()
    }
}