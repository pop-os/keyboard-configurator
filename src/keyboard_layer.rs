use cascade::cascade;
use gtk::{cairo, gdk, glib, glib::Propagation, pango, prelude::*, subclass::prelude::*};
use once_cell::unsync::OnceCell;
use std::{
    cell::{Cell, RefCell},
    f64::consts::PI,
};

use crate::{Page, TestingColors};
use backend::{Board, DerefCell, Key, Rect, Rgb};
use widgets::SelectedKeys;

const SCALE: f64 = 64.;
const MARGIN: f64 = 2.;
const RADIUS: f64 = 4.;
const MINIMUM_WIDTH: i32 = 320;

/// One transform for both drawing and hit testing. Never rearrange physical keys
/// when the window narrows; scale the complete keyboard down instead.
#[derive(Clone, Copy, Debug)]
struct LayoutTransform {
    scale: f64,
    offset: f64,
}

impl LayoutTransform {
    fn new(available: i32, natural: i32) -> Self {
        let scale = (f64::from(available.max(1)) / f64::from(natural.max(1))).min(1.0);
        Self {
            scale,
            offset: (f64::from(available) - f64::from(natural) * scale).max(0.0) / 2.0,
        }
    }

    fn position(self, rect: Rect) -> Rect {
        Rect {
            x: rect.x * self.scale + self.offset,
            y: rect.y * self.scale,
            w: rect.w * self.scale,
            h: rect.h * self.scale,
        }
    }
}

#[derive(Default)]
pub struct KeyboardLayerInner {
    page: Cell<Page>,
    board: DerefCell<Board>,
    selected: RefCell<SelectedKeys>,
    selectable: Cell<bool>,
    wide_width: OnceCell<i32>,
    wide_height: OnceCell<i32>,
    testing_colors: RefCell<TestingColors>,
}

#[glib::object_subclass]
impl ObjectSubclass for KeyboardLayerInner {
    const NAME: &'static str = "S76KeyboardLayer";
    type ParentType = gtk::DrawingArea;
    type Type = KeyboardLayer;
}

impl ObjectImpl for KeyboardLayerInner {
    fn constructed(&self) {
        self.parent_constructed();

        self.obj().add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecBoxed::builder::<SelectedKeys>("selected").build(),
                glib::ParamSpecBoxed::builder::<TestingColors>("testing-colors").build(),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "selected" => self
                .obj()
                .set_selected(value.get::<&SelectedKeys>().unwrap().clone()),
            "testing-colors" => {
                self.testing_colors
                    .replace(value.get::<&TestingColors>().unwrap().clone());
                self.obj().queue_draw();
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "selected" => self.selected.borrow().to_value(),
            "testing-colors" => self.testing_colors.borrow().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for KeyboardLayerInner {
    fn draw(&self, cr: &cairo::Context) -> Propagation {
        self.parent_draw(cr);

        let transform = self.obj().transform();
        cr.save().unwrap();
        cr.translate(transform.offset, 0.0);
        cr.scale(transform.scale, transform.scale);

        let selected = Rgb::new(0xfb, 0xb8, 0x6c).to_floats();

        let testing_colors = self.testing_colors.borrow();

        for (i, k) in self.obj().keys().iter().enumerate() {
            let Rect { x, y, w, h } = self.obj().key_position_wide(k);

            let mut bg = if let Some(rgb) = testing_colors
                .0
                .get(&(k.electrical.0 as usize, k.electrical.1 as usize))
            {
                rgb
            } else {
                &k.background_color
            }
            .to_floats();

            if k.pressed() {
                bg = self.board.layout().meta.pressed_color.to_floats();
            }

            let fg = if (bg.0 + bg.1 + bg.2) / 3. >= 0.5 {
                (0., 0., 0.)
            } else {
                (1., 1., 1.)
            };

            let mut text_alpha = 1.;
            let mut bg_alpha = 1.;
            if let Some(layer) = self.page.get().layer() {
                let scancode_name = k.get_scancode(layer).unwrap().1;
                if scancode_name == "NONE" || scancode_name == "ROLL_OVER" {
                    text_alpha = 0.5;
                    bg_alpha = 0.75;
                }
            }

            // Rounded rectangle
            cr.new_sub_path();
            cr.arc(x + w - RADIUS, y + RADIUS, RADIUS, -0.5 * PI, 0.);
            cr.arc(x + w - RADIUS, y + h - RADIUS, RADIUS, 0., 0.5 * PI);
            cr.arc(x + RADIUS, y + h - RADIUS, RADIUS, 0.5 * PI, PI);
            cr.arc(x + RADIUS, y + RADIUS, RADIUS, PI, 1.5 * PI);
            cr.close_path();

            cr.set_source_rgba(bg.0, bg.1, bg.2, bg_alpha);
            cr.fill_preserve().unwrap();

            if self.selectable.get() && self.obj().selected().contains(&i) {
                cr.set_source_rgb(selected.0, selected.1, selected.2);
                cr.set_line_width(4.);
                cr.stroke().unwrap();
            }

            // Draw label
            let text = self.obj().page().get_label(k);
            let layout = cascade! {
                self.obj().create_pango_layout(Some(&text));
                ..set_width((w * pango::SCALE as f64) as i32);
                ..set_alignment(pango::Alignment::Center);
            };
            let text_height = layout.pixel_size().1 as f64;
            cr.new_path();
            cr.move_to(x, y + (h - text_height) / 2.);
            cr.set_source_rgba(fg.0, fg.1, fg.2, text_alpha);
            pangocairo::show_layout(cr, &layout);
        }

        cr.restore().unwrap();
        Propagation::Proceed
    }

    fn button_press_event(&self, evt: &gdk::EventButton) -> Propagation {
        self.parent_button_press_event(evt);

        if !self.selectable.get() {
            return Propagation::Proceed;
        }

        let pos = evt.position();
        let pressed = self
            .obj()
            .keys()
            .iter()
            .position(|k| self.obj().key_position(k).contains(pos.0, pos.1));

        if let Some(pressed) = pressed {
            let shift = evt.state().contains(gdk::ModifierType::SHIFT_MASK);
            let mut selected = self.obj().selected();
            if shift {
                if selected.contains(&pressed) {
                    selected.remove(&pressed);
                } else {
                    selected.insert(pressed);
                }
            } else if selected.contains(&pressed) {
                selected.clear();
            } else {
                selected.clear();
                selected.insert(pressed);
            }
            self.obj().set_selected(selected);
        }

        Propagation::Proceed
    }

    fn request_mode(&self) -> gtk::SizeRequestMode {
        gtk::SizeRequestMode::HeightForWidth
    }

    fn preferred_width(&self) -> (i32, i32) {
        let natural = self.obj().wide_width();
        (MINIMUM_WIDTH.min(natural), natural)
    }

    fn preferred_height(&self) -> (i32, i32) {
        let height = self.obj().wide_height();
        (height, height)
    }

    fn preferred_height_for_width(&self, width: i32) -> (i32, i32) {
        let transform = LayoutTransform::new(width, self.obj().wide_width());
        let height = (f64::from(self.obj().wide_height()) * transform.scale).ceil() as i32;
        (height, height)
    }

    fn preferred_width_for_height(&self, _width: i32) -> (i32, i32) {
        self.preferred_width()
    }
}

impl DrawingAreaImpl for KeyboardLayerInner {}

glib::wrapper! {
    pub struct KeyboardLayer(ObjectSubclass<KeyboardLayerInner>)
        @extends gtk::DrawingArea, gtk::Widget;
}

impl KeyboardLayer {
    pub fn new(page: Page, board: Board) -> Self {
        let obj = glib::Object::new::<Self>();
        obj.inner().page.set(page);
        obj.inner().board.set(board);
        obj
    }

    fn inner(&self) -> &KeyboardLayerInner {
        KeyboardLayerInner::from_obj(self)
    }

    pub fn page(&self) -> Page {
        self.inner().page.get()
    }

    pub fn set_page(&self, page: Page) {
        self.inner().page.set(page);
        self.queue_draw();
    }

    pub fn keys(&self) -> &[Key] {
        self.inner().board.keys()
    }

    /// Read-only smoke metrics use the same rectangles as actual hit testing.
    pub fn fit_metrics(&self) -> serde_json::Value {
        let mut right: f64 = 0.0;
        let mut bottom: f64 = 0.0;
        let mut hits_match = true;
        for (index, key) in self.keys().iter().enumerate() {
            let rect = self.key_position(key);
            right = right.max(rect.x + rect.w);
            bottom = bottom.max(rect.y + rect.h);
            hits_match &= self.keys().iter().position(|other| {
                self.key_position(other)
                    .contains(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0)
            }) == Some(index);
        }
        serde_json::json!({"width":self.allocated_width(), "height":self.allocated_height(),
            "scale":self.transform().scale, "keys":self.keys().len(), "right":right, "bottom":bottom,
            "hits_match":hits_match,
            "fits":right <= f64::from(self.allocated_width()) + 1.0 && bottom <= f64::from(self.allocated_height()) + 1.0})
    }

    pub fn selected(&self) -> SelectedKeys {
        self.inner().selected.borrow().clone()
    }

    pub fn set_selected(&self, i: SelectedKeys) {
        self.inner().selected.replace(i);
        self.queue_draw();
        self.notify("selected");
    }

    pub fn set_selectable(&self, selectable: bool) {
        self.inner().selectable.set(selectable);
        self.queue_draw();
    }

    fn keys_maximize<F: Fn(&Key) -> i32>(&self, cell: &OnceCell<i32>, cb: F) -> i32 {
        *cell.get_or_init(|| self.keys().iter().map(cb).max().unwrap())
    }

    fn wide_width(&self) -> i32 {
        self.keys_maximize(&self.inner().wide_width, |k| {
            let pos = self.key_position_wide(k);
            (pos.x + pos.w).ceil() as i32
        })
    }

    fn wide_height(&self) -> i32 {
        self.keys_maximize(&self.inner().wide_height, |k| {
            let pos = self.key_position_wide(k);
            (pos.y + pos.h + 4.).ceil() as i32
        })
    }

    fn key_position_wide(&self, k: &Key) -> Rect {
        Rect {
            x: (k.physical.x * SCALE) + MARGIN,
            y: -(k.physical.y * SCALE) + MARGIN,
            w: (k.physical.w * SCALE) - MARGIN * 2.,
            h: (k.physical.h * SCALE) - MARGIN * 2.,
        }
    }

    fn transform(&self) -> LayoutTransform {
        LayoutTransform::new(self.allocated_width(), self.wide_width())
    }

    fn key_position(&self, k: &Key) -> Rect {
        self.transform().position(self.key_position_wide(k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_without_rearranging_and_centers_without_upscaling() {
        let right_key = Rect::new(1200.0, 64.0, 60.0, 60.0);
        let narrow = LayoutTransform::new(640, 1280);
        let pos = narrow.position(right_key);
        assert_eq!(narrow.scale, 0.5);
        assert_eq!((pos.x, pos.y, pos.w, pos.h), (600.0, 32.0, 30.0, 30.0));
        assert!(pos.contains(615.0, 47.0));
        assert!(!pos.contains(1199.0, 64.0));
        let wide = LayoutTransform::new(1500, 1280);
        assert_eq!(wide.scale, 1.0);
        assert_eq!(wide.position(right_key).x, 1310.0);
    }

    #[test]
    fn resize_preserves_every_key_center_and_bounds() {
        for width in [320, 480, 640, 900, 1280, 1500] {
            let transform = LayoutTransform::new(width, 1280);
            for column in 0..20 {
                let rect = Rect::new(f64::from(column * 64 + 2), 322.0, 60.0, 60.0);
                let pos = transform.position(rect);
                assert!(pos.x >= 0.0 && pos.x + pos.w <= f64::from(width));
                assert!(pos.contains(pos.x + pos.w / 2.0, pos.y + pos.h / 2.0));
                assert!((pos.y / transform.scale - rect.y).abs() < 1e-9);
            }
        }
    }
}
