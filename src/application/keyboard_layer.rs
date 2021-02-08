use cascade::cascade;
use glib::subclass;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{
    cell::Cell,
    convert::TryFrom,
    f64::consts::PI,
    rc::Rc,
};
use once_cell::unsync::OnceCell;

use crate::Rgb;
use super::{Key, Page};

const SCALE: f64 = 64.0;
const MARGIN: f64 = 2.;
const RADIUS: f64 = 4.;

#[derive(Default)]
pub struct KeyboardLayerInner {
    page: Cell<Page>,
    keys: OnceCell<Rc<[Key]>>,
    selected: Cell<Option<usize>>,
    selectable: Cell<bool>,
}

impl ObjectSubclass for KeyboardLayerInner {
    const NAME: &'static str = "S76KeyboardLayer";

    type ParentType = gtk::DrawingArea;
    type Type = KeyboardLayer;

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;
    type Interfaces = ();

    glib::object_subclass!();

    fn new() -> Self {
        Self {
            selectable: Cell::new(true),
            ..Self::default()
        }
    }
}

impl ObjectImpl for KeyboardLayerInner {
    fn constructed(&self, widget: &KeyboardLayer) {
        self.parent_constructed(widget);

        widget.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpec::int(
                    "selected",
                    "selected",
                    "selected",
                    -1,
                    i32::MAX,
                    -1,
                    glib::ParamFlags::READWRITE,
                )
            ]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(&self, widget: &KeyboardLayer, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.get_name() {
            "selected" => {
                let v: i32 = value.get_some().unwrap();
                let selected = usize::try_from(v).ok();
                widget.set_selected(selected);
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(&self, widget: &KeyboardLayer, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.get_name() {
            "selected" => {
                widget.selected().map(|v| v as i32).unwrap_or(-1).to_value()
            }
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for KeyboardLayerInner {
    fn draw(&self, widget: &KeyboardLayer, cr: &cairo::Context) -> Inhibit {
        self.parent_draw(widget, cr);

        let selected = Rgb::new(0xfb, 0xb8, 0x6c).to_floats();
        for (i, k) in widget.keys().iter().enumerate() {
            let x = (k.physical.x * SCALE) + MARGIN;
            let y = -(k.physical.y * SCALE) + MARGIN;
            let w = (k.physical.w * SCALE) - MARGIN * 2.;
            let h = (k.physical.h * SCALE) - MARGIN * 2.;

            let bg = k.background_color.to_floats();
            let fg = k.foreground_color.to_floats();

            // Rounded rectangle
            cr.new_sub_path();
            cr.arc(x + w - RADIUS, y + RADIUS, RADIUS, -0.5 * PI, 0.);
            cr.arc(x + w - RADIUS, y + h - RADIUS, RADIUS, 0., 0.5 * PI);
            cr.arc(x + RADIUS, y + h - RADIUS, RADIUS, 0.5 * PI, PI);
            cr.arc(x + RADIUS, y + RADIUS, RADIUS, PI, 1.5 * PI);
            cr.close_path();

            cr.set_source_rgb(bg.0, bg.1, bg.2);
            cr.fill_preserve();

            if widget.selected() == Some(i) {
                cr.set_source_rgb(selected.0, selected.1, selected.2);
                cr.set_line_width(4.);
                cr.stroke();
            }

            // Draw label
            let text = k.get_label(widget.page());
            let layout = cascade! {
                widget.create_pango_layout(Some(&text));
                ..set_width((w * pango::SCALE as f64) as i32);
                ..set_alignment(pango::Alignment::Center);
            };
            let text_height = layout.get_pixel_size().1 as f64;
            cr.new_path();
            cr.move_to(x, y + (h - text_height) / 2.);
            cr.set_source_rgb(fg.0, fg.1, fg.2);
            pangocairo::show_layout(cr, &layout);
        }

        Inhibit(false)
    }

    fn button_press_event(&self, widget: &KeyboardLayer, evt: &gdk::EventButton) -> Inhibit {
        self.parent_button_press_event(widget, evt);

        if !self.selectable.get() {
            return Inhibit(false);
        }

        let pos = evt.get_position();
        for (i, k) in widget.keys().iter().enumerate() {
            let x = (k.physical.x * SCALE) + MARGIN;
            let y = -(k.physical.y * SCALE) + MARGIN;
            let w = (k.physical.w * SCALE) - MARGIN * 2.;
            let h = (k.physical.h * SCALE) - MARGIN * 2.;

            if (x..=x+w).contains(&pos.0) && (y..=y+h).contains(&pos.1) {
                if widget.selected() == Some(i) {
                    widget.set_selected(None);
                } else {
                    widget.set_selected(Some(i));
                }
            }
        }

        Inhibit(false)
    }
}

impl DrawingAreaImpl for KeyboardLayerInner {}

glib::wrapper! {
    pub struct KeyboardLayer(ObjectSubclass<KeyboardLayerInner>)
        @extends gtk::DrawingArea, gtk::Widget;
}

impl KeyboardLayer {
    pub fn new(page: Page, keys: Rc<[Key]>) -> Self {
        let obj: Self = glib::Object::new(&[]).unwrap();

        let (width, height) = keys.iter().map(|k| {
            let w = (k.physical.w + k.physical.x) * SCALE - MARGIN;
            let h = (k.physical.h - k.physical.y) * SCALE - MARGIN;
            (w as i32, h as i32)
        }).max().unwrap();
        obj.set_size_request(width, height);

        obj.inner().page.set(page);
        obj.inner().keys.set(keys).unwrap();

        obj
    }

    fn inner(&self) -> &KeyboardLayerInner {
        KeyboardLayerInner::from_instance(self)
    }

    pub fn page(&self) -> Page {
        self.inner().page.get()
    }

    pub fn keys(&self) -> &[Key] {
        self.inner().keys.get().unwrap()
    }

    pub fn selected(&self) -> Option<usize> {
        self.inner().selected.get()
    }

    pub fn set_selected(&self, i: Option<usize>) {
        self.inner().selected.set(i);
        self.queue_draw();
        self.notify("selected");
    }

    pub fn set_selectable(&self, selectable: bool) {
        self.inner().selectable.set(selectable);
        if !selectable {
            self.set_selected(None);
        }
    }
}
