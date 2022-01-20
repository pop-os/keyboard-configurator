use crate::fl;
use cascade::cascade;
use futures::{prelude::*, stream::FuturesUnordered};
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;
use std::cell::{Cell, RefCell};

use backend::{Board, DerefCell, Hs, Mode};
use widgets::{KeyboardColor, KeyboardColorIndex, SelectedKeys};

#[derive(Default)]
pub struct BacklightInner {
    board: DerefCell<Board>,
    disable_color_button: DerefCell<gtk::Button>,
    keyboard_color: DerefCell<KeyboardColor>,
    color_label: DerefCell<gtk::Label>,
    color_row: DerefCell<gtk::ListBoxRow>,
    brightness_scale: DerefCell<gtk::Scale>,
    brightness_label: DerefCell<gtk::Label>,
    brightness_row: DerefCell<gtk::ListBoxRow>,
    saturation_scale: DerefCell<gtk::Scale>,
    saturation_row: DerefCell<gtk::ListBoxRow>,
    mode_combobox: DerefCell<gtk::ComboBoxText>,
    mode_row: DerefCell<gtk::ListBoxRow>,
    speed_scale: DerefCell<gtk::Scale>,
    speed_row: DerefCell<gtk::ListBoxRow>,
    layer: Cell<usize>,
    do_not_set: Cell<bool>,
    selected: RefCell<SelectedKeys>,
}

#[glib::object_subclass]
impl ObjectSubclass for BacklightInner {
    const NAME: &'static str = "S76Backlight";
    type ParentType = gtk::ListBox;
    type Type = Backlight;
}

impl ObjectImpl for BacklightInner {
    fn constructed(&self, obj: &Self::Type) {
        self.do_not_set.set(true);

        let mode_combobox = cascade! {
            gtk::ComboBoxText::new();
            ..connect_changed(clone!(@weak obj => move |_|
                obj.mode_speed_changed();
            ));
        };

        for mode in Mode::all() {
            mode_combobox.append(Some(mode.id), &mode.name);
        }

        let speed_scale = cascade! {
            gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., 255., 1.);
            ..set_halign(gtk::Align::Fill);
            ..set_value_pos(gtk::PositionType::Right);
            ..set_size_request(200, 0);
            ..connect_value_changed(clone!(@weak obj => move |_|
                obj.mode_speed_changed();
            ));
        };

        let brightness_scale = cascade! {
            gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., 100., 1.);
            ..set_halign(gtk::Align::Fill);
            ..set_value_pos(gtk::PositionType::Right);
            ..set_size_request(200, 0);
            ..connect_value_changed(clone!(@weak obj => move |_|
                obj.brightness_changed();
            ));
        };

        let keyboard_color = KeyboardColor::new(None, KeyboardColorIndex::Layer(0));

        let saturation_adjustment = cascade! {
            gtk::Adjustment::new(0., 0., 100., 1., 1., 0.);
            ..bind_property("value", &keyboard_color, "hs")
                .transform_from(|_, value| {
                    let hs: &Hs = value.get().unwrap();
                    Some((hs.s * 100.).to_value())
                })
                .transform_to(|_, value| {
                    let s: f64 = value.get().unwrap();
                    Some(Hs::new(0., s / 100.).to_value())
                })
                .flags(glib::BindingFlags::BIDIRECTIONAL)
                .build();
        };

        let saturation_scale = cascade! {
            gtk::Scale::new(gtk::Orientation::Horizontal, Some(&saturation_adjustment));
            ..set_halign(gtk::Align::Fill);
            ..set_value_pos(gtk::PositionType::Right);
            ..set_digits(0);
            ..set_size_request(200, 0);
        };

        fn row(widget: &impl IsA<gtk::Widget>) -> gtk::ListBoxRow {
            cascade! {
                gtk::ListBoxRow::new();
                ..set_selectable(false);
                ..set_activatable(false);
                ..set_margin(8);
                ..add(widget);
            }
        }

        fn label_row(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::ListBoxRow {
            row(&cascade! {
                gtk::Box::new(gtk::Orientation::Horizontal, 8);
                ..add(&cascade! {
                    gtk::Label::new(Some(label));
                    ..set_halign(gtk::Align::Start);
                });
                ..pack_end(widget, false, false, 0);
            })
        }

        let disable_color_button = cascade! {
            gtk::Button::with_label(&fl!("button-disable"));
            ..set_no_show_all(true);
            ..connect_clicked(clone!(@weak obj => move |_| obj.disable_color_clicked()));
        };

        let color_label = gtk::Label::new(None);
        let brightness_label = gtk::Label::new(Some(&fl!("layer-all-brightness")));

        let mode_row = label_row(&fl!("layer-color-pattern"), &mode_combobox);
        let speed_row = label_row(&fl!("layer-animation-speed"), &speed_scale);
        let saturation_row = label_row(&fl!("layer-saturation"), &saturation_scale);
        let color_row = row(&cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 8);
            ..add(&color_label);
            ..pack_end(&keyboard_color, false, false, 0);
            ..pack_end(&disable_color_button, false, false, 0);
        });
        let brightness_row = row(&cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 8);
            ..add(&brightness_label);
            ..pack_end(&brightness_scale, false, false, 0);
        });

        cascade! {
            obj;
            ..set_valign(gtk::Align::Start);
            ..style_context().add_class("frame");
            ..add(&mode_row);
            ..add(&speed_row);
            ..add(&saturation_row);
            ..add(&color_row);
            ..add(&brightness_row);
        };

        self.disable_color_button.set(disable_color_button);
        self.keyboard_color.set(keyboard_color);
        self.color_label.set(color_label);
        self.color_row.set(color_row);
        self.brightness_label.set(brightness_label);
        self.brightness_scale.set(brightness_scale);
        self.brightness_row.set(brightness_row);
        self.mode_combobox.set(mode_combobox);
        self.mode_row.set(mode_row);
        self.speed_scale.set(speed_scale);
        self.speed_row.set(speed_row);
        self.saturation_scale.set(saturation_scale);
        self.saturation_row.set(saturation_row);
    }

    fn dispose(&self, obj: &Self::Type) {
        obj.led_save();
    }

    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecString::new(
                    "mode",
                    "mode",
                    "mode",
                    None,
                    glib::ParamFlags::READABLE,
                ),
                glib::ParamSpecBoxed::new(
                    "selected",
                    "selected",
                    "selected",
                    SelectedKeys::static_type(),
                    glib::ParamFlags::WRITABLE,
                ),
                glib::ParamSpecBoolean::new(
                    "is-per-key",
                    "is-per-key",
                    "is-per-key",
                    false,
                    glib::ParamFlags::READABLE,
                ),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        obj: &Self::Type,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.name() {
            "selected" => {
                let selected: &SelectedKeys = value.get().unwrap();
                obj.inner().selected.replace(selected.clone());
                obj.update_per_key();
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "mode" => obj.mode().id.to_value(),
            "is-per-key" => obj.mode().is_per_key().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for BacklightInner {}
impl ContainerImpl for BacklightInner {}
impl ListBoxImpl for BacklightInner {}

glib::wrapper! {
    pub struct Backlight(ObjectSubclass<BacklightInner>)
        @extends gtk::ListBox, gtk::Container, gtk::Widget;
}

impl Backlight {
    pub fn new(board: Board) -> Self {
        let max_brightness = board.max_brightness() as f64;
        let has_led_save = board.has_led_save();

        let obj: Self = glib::Object::new(&[]).unwrap();
        obj.inner().board.set(board.clone());
        obj.inner().keyboard_color.set_board(Some(board));
        obj.inner().brightness_scale.set_range(0.0, max_brightness);
        obj.invalidate_filter();
        obj.set_layer(0);
        obj.set_filter_func(Some(Box::new(
            clone!(@weak obj => @default-panic, move |row|
                obj.filter_func(row)
            ),
        )));
        obj.set_header_func(Some(Box::new(clone!(@weak obj => move |row, before|
            obj.header_func(row, before)
        ))));

        if !obj.board().layout().meta.has_per_layer {
            obj.inner()
                .brightness_label
                .set_label(&fl!("keyboard-brightness"));
        }

        if has_led_save {
            glib::timeout_add_seconds_local(
                10,
                clone!(@weak obj => @default-return Continue(false), move || {
                    obj.led_save();
                    Continue(true)
                }),
            );
        }

        obj
    }

    fn inner(&self) -> &BacklightInner {
        BacklightInner::from_instance(self)
    }

    fn board(&self) -> &Board {
        &self.inner().board
    }

    pub fn mode(&self) -> &'static Mode {
        if let Some(id) = self.inner().mode_combobox.active_id() {
            if let Some(mode) = Mode::from_id(id.as_str()) {
                return mode;
            }
        }
        Mode::from_id("SOLID_COLOR").unwrap()
    }

    fn header_func(&self, row: &gtk::ListBoxRow, before: Option<&gtk::ListBoxRow>) {
        if before.is_none() {
            row.set_header(None::<&gtk::Widget>)
        } else if row.header().is_none() {
            row.set_header(Some(&cascade! {
                gtk::Separator::new(gtk::Orientation::Horizontal);
                ..show();
            }));
        }
    }

    fn filter_func(&self, row: &gtk::ListBoxRow) -> bool {
        let inner = self.inner();
        let layout = inner.board.layout();
        if row == &*inner.mode_row {
            layout.meta.has_mode
        } else if row == &*inner.speed_row {
            layout.meta.has_mode && self.mode().has_speed
        } else if row == &*inner.color_row {
            layout.meta.has_color && (!layout.meta.has_mode || self.mode().has_hue)
        } else if row == &*inner.saturation_row {
            !self.mode().has_hue && !self.mode().is_disabled()
        } else if row == &*inner.brightness_row {
            layout.meta.has_brightness && (!layout.meta.has_mode || !self.mode().is_disabled())
        } else {
            true
        }
    }

    fn mode_speed_changed(&self) {
        self.notify("mode");
        self.notify("is-per-key");

        if self.mode().is_per_key() {
            self.update_per_key();
            self.inner().color_label.set_label(&fl!("key-color"));
        } else {
            self.inner().keyboard_color.set_sensitive(true);
            self.inner()
                .keyboard_color
                .set_index(KeyboardColorIndex::Layer(self.inner().layer.get()));
            if self.board().layout().meta.has_per_layer {
                self.inner().color_label.set_label(&fl!("layer-color"));
            } else {
                self.inner().color_label.set_label(&fl!("keyboard-color"));
            }
        }
        self.inner()
            .disable_color_button
            .set_visible(self.mode().is_per_key());
        self.invalidate_filter();

        if self.inner().do_not_set.get() {
            return;
        }

        let board = self.board().clone();
        let speed = self.inner().speed_scale.value();
        let mode = self.mode();
        let layer = self.inner().layer.get() as usize;
        glib::MainContext::default().spawn_local(async move {
            let layer = &board.layers()[layer];
            if let Err(err) = layer.set_mode(mode, speed as u8).await {
                error!("{}: {}", fl!("error-set-keyboard-mode"), err);
            }
        });
    }

    fn brightness_changed(&self) {
        if self.inner().do_not_set.get() {
            return;
        }
        let value = self.inner().brightness_scale.value() as i32;
        let board = self.board().clone();
        glib::MainContext::default().spawn_local(async move {
            for layer in board.layers() {
                if let Err(err) = layer.set_brightness(value).await {
                    error!("{}: {}", fl!("error-set-keyboard-brightness"), err);
                }
            }
        });
        debug!("Brightness: {}", value)
    }

    pub fn set_layer(&self, mut layer: usize) {
        if !self.board().layout().meta.has_per_layer {
            layer = 0;
        }

        self.inner().layer.set(layer);

        let layer = &self.board().layers()[layer];

        let (mode, speed) = layer.mode().unwrap_or((&Mode::all()[0], 128));
        let brightness = layer.brightness() as f64;

        self.inner().do_not_set.set(true);

        self.inner().mode_combobox.set_active_id(Some(mode.id));
        self.inner().speed_scale.set_value(speed.into());
        self.inner().brightness_scale.set_value(brightness);
        if !self.mode().is_per_key() {
            self.inner()
                .keyboard_color
                .set_index(KeyboardColorIndex::Layer(self.inner().layer.get()));
        }

        self.inner().do_not_set.set(false);
    }

    fn update_per_key(&self) {
        if !self.mode().is_per_key() {
            return;
        }

        let selected = self.inner().selected.borrow();
        self.inner()
            .keyboard_color
            .set_index(KeyboardColorIndex::Keys(selected.clone()));
        self.inner()
            .keyboard_color
            .set_sensitive(!selected.is_empty());
        self.inner()
            .disable_color_button
            .set_sensitive(!selected.is_empty());
    }

    fn disable_color_clicked(&self) {
        let self_ = self.clone();
        let selected = self.inner().selected.borrow().clone();
        glib::MainContext::default().spawn_local(async move {
            let futures = FuturesUnordered::new();
            for i in selected.iter() {
                futures.push(self_.board().keys()[*i].set_color(None));
            }
            if let Err(err) = futures.try_collect::<()>().await {
                error!("{}: {}", fl!("error-disable-key"), err);
            }
            self_.update_per_key();
        });
    }

    fn led_save(&self) {
        if self.board().has_led_save() {
            let board = self.board().clone();
            glib::MainContext::default().spawn_local(async move {
                if let Err(err) = board.led_save().await {
                    error!("{}: {}", fl!("error-save-leds"), err);
                }
            });
        }
    }
}
