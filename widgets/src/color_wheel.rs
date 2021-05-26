// A hue/saturation color wheel that allows a color to be selected.

use cascade::cascade;
use futures::future::{abortable, AbortHandle};
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;

use crate::DerefCell;
use backend::{Hs, Rgb};

#[derive(Default)]
pub struct ColorWheelInner {
    selected_hs: Cell<Hs>,
    surface: RefCell<Option<cairo::ImageSurface>>,
    thread_pool: DerefCell<glib::ThreadPool>,
    abort_handle: RefCell<Option<AbortHandle>>,
    gesture_drag: DerefCell<gtk::GestureDrag>,
    drag_start_xy: Cell<(f64, f64)>,
}

#[glib::object_subclass]
impl ObjectSubclass for ColorWheelInner {
    const NAME: &'static str = "S76ColorWheel";
    type ParentType = gtk::DrawingArea;
    type Type = ColorWheel;
}

impl ObjectImpl for ColorWheelInner {
    fn constructed(&self, wheel: &ColorWheel) {
        self.parent_constructed(wheel);

        self.thread_pool
            .set(glib::ThreadPool::new_shared(None).unwrap());

        self.gesture_drag.set(cascade! {
            gtk::GestureDrag::new(wheel);
            ..set_propagation_phase(gtk::PropagationPhase::Bubble);
            ..connect_drag_begin(clone!(@weak wheel => move |_, start_x, start_y| {
                wheel.mouse_select((start_x, start_y));
                wheel.inner().drag_start_xy.set((start_x, start_y))
            }));
            ..connect_drag_update(clone!(@weak wheel => move |_, offset_x, offset_y| {
                let (start_x, start_y) = wheel.inner().drag_start_xy.get();
                wheel.mouse_select((start_x + offset_x, start_y + offset_y));
            }));
        });
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpec::boxed(
                    "hs",
                    "hs",
                    "hs",
                    Hs::get_type(),
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpec::double(
                    "hue",
                    "hue",
                    "hue",
                    0.,
                    360.,
                    0.,
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpec::double(
                    "saturation",
                    "saturation",
                    "saturation",
                    0.,
                    100.,
                    0.,
                    glib::ParamFlags::READWRITE,
                ),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        wheel: &ColorWheel,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.get_name() {
            "hs" => {
                wheel.set_hs(*value.get_some::<&Hs>().unwrap());
            }
            "hue" => {
                let mut hue: f64 = value.get_some().unwrap();
                hue = (hue * PI / 180.).max(0.).min(2. * PI);
                let hs = wheel.hs();
                wheel.set_hs(Hs::new(hue, *hs.s));
            }
            "saturation" => {
                let mut saturation: f64 = value.get_some().unwrap();
                saturation = (saturation / 100.).max(0.).min(1.);
                let hs = wheel.hs();
                wheel.set_hs(hs);
                wheel.set_hs(Hs::new(*hs.h, saturation));
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(&self, wheel: &ColorWheel, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.get_name() {
            "hs" => wheel.hs().to_value(),
            "hue" => {
                let mut hue = *wheel.hs().h * 180. / PI;
                hue = (360. + hue) % 360.;
                hue.to_value()
            }
            "saturation" => (wheel.hs().s * 100.).to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for ColorWheelInner {
    fn draw(&self, wheel: &ColorWheel, cr: &cairo::Context) -> Inhibit {
        self.parent_draw(wheel, cr);

        let width = f64::from(wheel.get_allocated_width());
        let height = f64::from(wheel.get_allocated_height());

        let radius = width.min(height) / 2.;

        cr.translate(width / 2. - radius, 0.);

        // Draw color wheel
        if let Some(surface) = self.surface.borrow().as_ref() {
            let pattern = cairo::SurfacePattern::create(surface);
            let scale = surface.get_width() as f64 / (radius * 2.);
            let mut matrix = cairo::Matrix::identity();
            matrix.scale(scale, scale);
            pattern.set_matrix(matrix);
            cr.set_source(&pattern);
        }
        cr.arc(radius, radius, radius, 0., 2. * PI);
        cr.fill();

        // Draw selector circle
        let hs = wheel.hs();
        let x = radius + hs.h.cos() * (*hs.s) * radius;
        let y = radius - hs.h.sin() * (*hs.s) * radius;
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
        let (future, abort_handle) = abortable(clone!(@weak wheel, @strong rect => async move {
            let surface = Some(wheel.generate_surface(&rect).await);
            wheel.inner().surface.replace(surface);
            wheel.queue_draw();
        }));
        if let Some(abort_handle) = self.abort_handle.replace(Some(abort_handle)) {
            abort_handle.abort();
        }
        glib::MainContext::default().spawn_local(async {
            let _ = future.await;
        });
    }

    fn get_request_mode(&self, _widget: &Self::Type) -> gtk::SizeRequestMode {
        gtk::SizeRequestMode::HeightForWidth
    }

    fn get_preferred_width(&self, _widget: &Self::Type) -> (i32, i32) {
        (0, 300)
    }

    fn get_preferred_height(&self, _widget: &Self::Type) -> (i32, i32) {
        (0, 300)
    }

    fn get_preferred_height_for_width(&self, _widget: &Self::Type, width: i32) -> (i32, i32) {
        (0, width)
    }

    fn get_preferred_width_for_height(&self, _widget: &Self::Type, height: i32) -> (i32, i32) {
        (0, height)
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
        self.notify("hs");
        self.notify("hue");
        self.notify("saturation");
    }

    pub fn connect_hs_changed<F: Fn(&Self) + 'static>(&self, f: F) {
        self.connect_notify_local(Some("hs"), move |wheel, _| f(wheel));
    }

    fn mouse_select(&self, pos: (f64, f64)) {
        let width = f64::from(self.get_allocated_width());
        let height = f64::from(self.get_allocated_height());

        let radius = width.min(height) / 2.;
        let (x, y) = (pos.0 - width / 2., radius - pos.1);

        let angle = y.atan2(x);
        let distance = y.hypot(x);

        self.set_hs(Hs::new(angle, (distance / radius).min(1.)));
    }

    async fn generate_surface(&self, rect: &gtk::Rectangle) -> cairo::ImageSurface {
        let size = rect.width.min(rect.height);
        let stride = cairo::Format::Rgb24.stride_for_width(size as u32).unwrap();

        let data = self
            .inner()
            .thread_pool
            .push_future(move || {
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

                data
            })
            .unwrap()
            .await;

        cairo::ImageSurface::create_for_data(data, cairo::Format::Rgb24, size, size, stride)
            .unwrap()
    }
}
