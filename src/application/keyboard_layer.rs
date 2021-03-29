use cascade::cascade;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    convert::TryFrom,
    f64::consts::PI,
    rc::Rc,
};

use super::{Key, Page, Rect};
use crate::DerefCell;
use daemon::Rgb;

const SCALE: f64 = 64.0;
const MARGIN: f64 = 2.;
const RADIUS: f64 = 4.;

#[derive(Default)]
pub struct KeyboardLayerInner {
    page: Cell<Page>,
    keys: DerefCell<Rc<[Key]>>,
    selected: RefCell<HashSet<usize>>,
    selectable: Cell<bool>,
    multiple: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for KeyboardLayerInner {
    const NAME: &'static str = "S76KeyboardLayer";
    type ParentType = gtk::DrawingArea;
    type Type = KeyboardLayer;

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
            vec![glib::ParamSpec::int(
                "selected",
                "selected",
                "selected",
                -1,
                i32::MAX,
                -1,
                glib::ParamFlags::READWRITE,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        widget: &KeyboardLayer,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.get_name() {
            "selected" => {
                let v: i32 = value.get_some().unwrap();
                let selected = usize::try_from(v).ok();
                widget.set_selected(selected.into_iter().collect());
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(
        &self,
        _widget: &KeyboardLayer,
        _id: usize,
        pspec: &glib::ParamSpec,
    ) -> glib::Value {
        match pspec.get_name() {
            "selected" => {
                let selected = self.selected.borrow();
                if selected.len() == 1 {
                    *selected.iter().next().unwrap() as i32
                } else {
                    -1
                }
                .to_value()
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
            let Rect { x, y, w, h } = scale_rect(&k.physical);

            let mut bg = k.background_color.to_floats();

            if k.pressed.get() {
                // Invert colors if pressed
                bg.0 = 1.0 - bg.0;
                bg.1 = 1.0 - bg.1;
                bg.2 = 1.0 - bg.2;
            }

            let fg = if (bg.0 + bg.1 + bg.2) / 3. >= 0.5 {
                (0., 0., 0.)
            } else {
                (1., 1., 1.)
            };

            // Rounded rectangle
            cr.new_sub_path();
            cr.arc(x + w - RADIUS, y + RADIUS, RADIUS, -0.5 * PI, 0.);
            cr.arc(x + w - RADIUS, y + h - RADIUS, RADIUS, 0., 0.5 * PI);
            cr.arc(x + RADIUS, y + h - RADIUS, RADIUS, 0.5 * PI, PI);
            cr.arc(x + RADIUS, y + RADIUS, RADIUS, PI, 1.5 * PI);
            cr.close_path();

            cr.set_source_rgb(bg.0, bg.1, bg.2);
            cr.fill_preserve();

            if widget.selected().contains(&i) {
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
        let pressed = widget
            .keys()
            .iter()
            .position(|k| scale_rect(&k.physical).contains(pos.0, pos.1));

        if let Some(pressed) = pressed {
            let shift = evt.get_state().contains(gdk::ModifierType::SHIFT_MASK);
            let mut selected = widget.selected();
            if shift && self.multiple.get() {
                if selected.contains(&pressed) {
                    selected.remove(&pressed);
                } else {
                    selected.insert(pressed);
                }
            } else {
                if selected.contains(&pressed) {
                    selected.clear();
                } else {
                    selected.clear();
                    selected.insert(pressed);
                }
            }
            widget.set_selected(selected);
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

        let (width, height) = keys
            .iter()
            .map(|k| {
                let w = (k.physical.w + k.physical.x) * SCALE - MARGIN;
                let h = (k.physical.h - k.physical.y) * SCALE - MARGIN;
                (w as i32, h as i32)
            })
            .max()
            .unwrap();
        obj.set_size_request(width, height);

        obj.inner().page.set(page);
        obj.inner().keys.set(keys);

        obj
    }

    fn inner(&self) -> &KeyboardLayerInner {
        KeyboardLayerInner::from_instance(self)
    }

    pub fn page(&self) -> Page {
        self.inner().page.get()
    }

    pub fn keys(&self) -> &[Key] {
        &self.inner().keys
    }

    pub fn selected(&self) -> HashSet<usize> {
        self.inner().selected.borrow().clone()
    }

    pub fn set_selected(&self, i: HashSet<usize>) {
        self.inner().selected.replace(i);
        self.queue_draw();
        self.notify("selected");
    }

    pub fn set_selectable(&self, selectable: bool) {
        self.inner().selectable.set(selectable);
        if !selectable {
            self.set_selected(HashSet::new());
        }
    }

    pub fn set_multiple(&self, multiple: bool) {
        self.inner().multiple.set(multiple)
    }
}

fn scale_rect(rect: &Rect) -> Rect {
    Rect {
        x: (rect.x * SCALE) + MARGIN,
        y: -(rect.y * SCALE) + MARGIN,
        w: (rect.w * SCALE) - MARGIN * 2.,
        h: (rect.h * SCALE) - MARGIN * 2.,
    }
}
