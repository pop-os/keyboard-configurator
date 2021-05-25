use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
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
const HALF_KEYBOARD_VSPACING: f64 = 16.;

#[derive(Default)]
pub struct KeyboardLayerInner {
    page: Cell<Page>,
    board: DerefCell<Board>,
    selected: RefCell<SelectedKeys>,
    selectable: Cell<bool>,
    multiple: Cell<bool>,
    wide_width: OnceCell<i32>,
    wide_height: OnceCell<i32>,
    narrow_width: OnceCell<i32>,
    testing_colors: RefCell<TestingColors>,
}

#[glib::object_subclass]
impl ObjectSubclass for KeyboardLayerInner {
    const NAME: &'static str = "S76KeyboardLayer";
    type ParentType = gtk::DrawingArea;
    type Type = KeyboardLayer;
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
                glib::ParamSpec::boxed(
                    "selected",
                    "selected",
                    "selected",
                    SelectedKeys::get_type(),
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpec::boolean(
                    "multiple",
                    "multiple",
                    "multiple",
                    false,
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpec::boxed(
                    "testing-colors",
                    "testing-colors",
                    "testing-colors",
                    TestingColors::get_type(),
                    glib::ParamFlags::READWRITE,
                ),
            ]
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
            "selected" => widget.set_selected(value.get_some::<&SelectedKeys>().unwrap().clone()),
            "multiple" => widget.set_multiple(value.get_some().unwrap()),
            "testing-colors" => {
                self.testing_colors
                    .replace(value.get_some::<&TestingColors>().unwrap().clone());
                widget.queue_draw();
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
            "selected" => self.selected.borrow().to_value(),
            "multiple" => self.multiple.get().to_value(),
            "testing-colors" => self.testing_colors.borrow().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for KeyboardLayerInner {
    fn draw(&self, widget: &KeyboardLayer, cr: &cairo::Context) -> Inhibit {
        self.parent_draw(widget, cr);

        let selected = Rgb::new(0xfb, 0xb8, 0x6c).to_floats();

        let testing_colors = self.testing_colors.borrow();

        for (i, k) in widget.keys().iter().enumerate() {
            let Rect { x, y, w, h } = widget.key_position(&k);

            let mut bg = if let Some(rgb) = testing_colors.0.get(&i) {
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
            cr.fill_preserve();

            if widget.selected().contains(&i) {
                cr.set_source_rgb(selected.0, selected.1, selected.2);
                cr.set_line_width(4.);
                cr.stroke();
            }

            // Draw label
            let text = widget.page().get_label(k);
            let layout = cascade! {
                widget.create_pango_layout(Some(&text));
                ..set_width((w * pango::SCALE as f64) as i32);
                ..set_alignment(pango::Alignment::Center);
            };
            let text_height = layout.get_pixel_size().1 as f64;
            cr.new_path();
            cr.move_to(x, y + (h - text_height) / 2.);
            cr.set_source_rgba(fg.0, fg.1, fg.2, text_alpha);
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
            .position(|k| widget.key_position(&k).contains(pos.0, pos.1));

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

    fn get_request_mode(&self, _widget: &Self::Type) -> gtk::SizeRequestMode {
        gtk::SizeRequestMode::HeightForWidth
    }

    fn get_preferred_width(&self, widget: &Self::Type) -> (i32, i32) {
        (widget.narrow_width(), widget.wide_width())
    }

    fn get_preferred_height_for_width(&self, widget: &Self::Type, width: i32) -> (i32, i32) {
        let height = if width < widget.wide_width() {
            widget.narrow_height()
        } else {
            widget.wide_height()
        };
        (height, height)
    }
}

impl DrawingAreaImpl for KeyboardLayerInner {}

glib::wrapper! {
    pub struct KeyboardLayer(ObjectSubclass<KeyboardLayerInner>)
        @extends gtk::DrawingArea, gtk::Widget;
}

impl KeyboardLayer {
    pub fn new(page: Page, board: Board) -> Self {
        let obj = glib::Object::new::<Self>(&[]).unwrap();
        board.connect_matrix_changed(clone!(@weak obj => move || obj.queue_draw()));
        obj.inner().page.set(page);
        obj.inner().board.set(board);
        obj
    }

    fn inner(&self) -> &KeyboardLayerInner {
        KeyboardLayerInner::from_instance(self)
    }

    pub fn page(&self) -> Page {
        self.inner().page.get()
    }

    pub fn set_page(&self, page: Page) {
        self.inner().page.set(page);
        self.queue_draw();
    }

    pub fn keys(&self) -> &[Key] {
        &self.inner().board.keys()
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
        if !selectable {
            self.set_selected(SelectedKeys::new());
        }
    }

    pub fn set_multiple(&self, multiple: bool) {
        self.inner().multiple.set(multiple);
        self.notify("multiple");
    }

    fn keys_maximize<F: Fn(&Key) -> i32>(&self, cell: &OnceCell<i32>, cb: F) -> i32 {
        *cell.get_or_init(|| self.keys().iter().map(cb).max().unwrap())
    }

    fn wide_width(&self) -> i32 {
        self.keys_maximize(&self.inner().wide_width, |k| {
            let pos = self.key_position_wide(k);
            (pos.x + pos.w) as i32
        })
    }

    fn wide_height(&self) -> i32 {
        self.keys_maximize(&self.inner().wide_height, |k| {
            let pos = self.key_position_wide(k);
            (pos.y + pos.h + 4.) as i32
        })
    }

    fn narrow_width(&self) -> i32 {
        self.keys_maximize(&self.inner().narrow_width, |k| {
            let mut pos = self.key_position_wide(k);
            let width = self.wide_width() as f64 / 2.;
            if pos.x + pos.w / 2. > width {
                pos.x -= width;
            }
            (pos.x + pos.w) as i32
        })
    }

    fn narrow_height(&self) -> i32 {
        self.wide_height() * 2 + HALF_KEYBOARD_VSPACING as i32
    }

    fn key_position_wide(&self, k: &Key) -> Rect {
        Rect {
            x: (k.physical.x * SCALE) + MARGIN,
            y: -(k.physical.y * SCALE) + MARGIN,
            w: (k.physical.w * SCALE) - MARGIN * 2.,
            h: (k.physical.h * SCALE) - MARGIN * 2.,
        }
    }

    fn key_position_narrow(&self, k: &Key) -> Rect {
        let mut rect = self.key_position_wide(k);
        let width = self.wide_width() as f64 / 2.;
        if rect.x + rect.w / 2. > width {
            rect.x -= (self.wide_width() - self.narrow_width()) as f64;
            rect.y += self.wide_height() as f64 + HALF_KEYBOARD_VSPACING;
        }
        rect
    }

    fn key_position(&self, k: &Key) -> Rect {
        let (mut pos, width) = if self.get_allocated_width() < self.wide_width() {
            (self.key_position_narrow(k), self.narrow_width())
        } else {
            (self.key_position_wide(k), self.wide_width())
        };
        pos.x += (self.get_allocated_width() - width) as f64 / 2.;
        pos
    }
}
