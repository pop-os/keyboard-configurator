use cascade::cascade;
use gtk::{
    cairo, gdk,
    glib::{self, clone},
    pango,
    prelude::*,
    subclass::prelude::*,
};
use once_cell::unsync::OnceCell;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
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
    wide_width: OnceCell<i32>,
    wide_height: OnceCell<i32>,
    narrow_width: OnceCell<i32>,
    testing_colors: RefCell<TestingColors>,
    linux_keymap: DerefCell<HashMap<String, u32>>,
    gdk_keymap: DerefCell<gdk::Keymap>,
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

        let display = gdk::Display::default().unwrap();
        let gdk_keymap = cascade! {
            gdk::Keymap::for_display(&display).unwrap();
            ..connect_keys_changed(clone!(@weak widget => move |_| widget.queue_draw()));
        };
        self.gdk_keymap.set(gdk_keymap);

        self.linux_keymap
            .set(serde_json::from_str(include_str!("../layouts/linux_keymap.json")).unwrap());

        widget.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecBoxed::new(
                    "selected",
                    "selected",
                    "selected",
                    SelectedKeys::static_type(),
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpecBoxed::new(
                    "testing-colors",
                    "testing-colors",
                    "testing-colors",
                    TestingColors::static_type(),
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
        match pspec.name() {
            "selected" => widget.set_selected(value.get::<&SelectedKeys>().unwrap().clone()),
            "testing-colors" => {
                self.testing_colors
                    .replace(value.get::<&TestingColors>().unwrap().clone());
                widget.queue_draw();
            }
            _ => unimplemented!(),
        }
    }

    fn property(
        &self,
        _widget: &KeyboardLayer,
        _id: usize,
        pspec: &glib::ParamSpec,
    ) -> glib::Value {
        match pspec.name() {
            "selected" => self.selected.borrow().to_value(),
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

            let mut text = widget.page().get_label(k);

            let mut text_alpha = 1.;
            let mut bg_alpha = 1.;
            if let Some(layer) = self.page.get().layer() {
                let scancode_name = k.get_scancode(layer).unwrap().1;

                use glib::translate::from_glib_none;
                if let Some(keycode) = self.linux_keymap.get(&scancode_name) {
                    let mut level_texts = Vec::new();
                    for level in 0..3 {
                        let keymap_key = unsafe {
                            from_glib_none(&gdk::ffi::GdkKeymapKey {
                                keycode: *keycode + 8,
                                group: 0,
                                level,
                            } as *const _)
                        };

                        if let Some(key) = self.gdk_keymap.lookup_key(&keymap_key) {
                            use gdk::keys::constants;
                            let level_text = match key {
                                // TODO
                                constants::BackSpace => "Backspace".to_string(),
                                constants::Delete => "Delete".to_string(),
                                constants::Escape => "Esc".to_string(),
                                constants::Return => "Enter".to_string(),
                                constants::space => "Space".to_string(),
                                constants::Tab => "Tab".to_string(),
                                _ => key
                                    .to_unicode()
                                    .map(|x| x.to_string())
                                    .unwrap_or_else(|| key.name().unwrap().to_string()),
                            };
                            level_texts.push(level_text);
                        } else {
                            break;
                        }
                    }

                    if level_texts.len() >= 2 {
                        text = format!("{}\n{}", &level_texts[1], &level_texts[0]);
                    } else if level_texts.len() >= 1 {
                        text = level_texts[0].clone();
                    }
                }

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

            if self.selectable.get() && widget.selected().contains(&i) {
                cr.set_source_rgb(selected.0, selected.1, selected.2);
                cr.set_line_width(4.);
                cr.stroke().unwrap();
            }

            // Draw label
            let layout = cascade! {
                widget.create_pango_layout(Some(&text));
                ..set_width((w * pango::SCALE as f64) as i32);
                ..set_alignment(pango::Alignment::Center);
            };
            let text_height = layout.pixel_size().1 as f64;
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

        let pos = evt.position();
        let pressed = widget
            .keys()
            .iter()
            .position(|k| widget.key_position(&k).contains(pos.0, pos.1));

        if let Some(pressed) = pressed {
            let shift = evt.state().contains(gdk::ModifierType::SHIFT_MASK);
            let mut selected = widget.selected();
            if shift {
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

    fn request_mode(&self, _widget: &Self::Type) -> gtk::SizeRequestMode {
        gtk::SizeRequestMode::HeightForWidth
    }

    fn preferred_width(&self, widget: &Self::Type) -> (i32, i32) {
        (widget.narrow_width(), widget.wide_width())
    }

    fn preferred_height_for_width(&self, widget: &Self::Type, width: i32) -> (i32, i32) {
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
        self.queue_draw();
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
        let (mut pos, width) = if self.allocated_width() < self.wide_width() {
            (self.key_position_narrow(k), self.narrow_width())
        } else {
            (self.key_position_wide(k), self.wide_width())
        };
        pos.x += (self.allocated_width() - width) as f64 / 2.;
        pos
    }
}
