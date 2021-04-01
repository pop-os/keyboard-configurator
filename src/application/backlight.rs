use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use std::{cell::Cell, convert::TryFrom};

use crate::{DerefCell, KeyboardColor};
use backend::{DaemonBoard, Hs, Mode};

#[derive(Default)]
pub struct BacklightInner {
    board: DerefCell<DaemonBoard>,
    keyboard_color: DerefCell<KeyboardColor>,
    color_row: DerefCell<gtk::ListBoxRow>,
    brightness_scale: DerefCell<gtk::Scale>,
    saturation_scale: DerefCell<gtk::Scale>,
    saturation_row: DerefCell<gtk::ListBoxRow>,
    mode_combobox: DerefCell<gtk::ComboBoxText>,
    mode_row: DerefCell<gtk::ListBoxRow>,
    speed_scale: DerefCell<gtk::Scale>,
    speed_row: DerefCell<gtk::ListBoxRow>,
    layer: Cell<u8>,
    do_not_set: Cell<bool>,
    selected: Cell<Option<usize>>,
    has_led_save: Cell<bool>,
    changed: Cell<bool>,
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
            mode_combobox.append(Some(mode.id), mode.name);
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

        let keyboard_color = cascade! {
            KeyboardColor::new(None, 0xf0);
            ..connect_local("notify::hs", false, clone!(@weak obj => move |_| {
                obj.inner().changed.set(true);
                None
            })).unwrap();
        };

        let saturation_adjustment = cascade! {
            gtk::Adjustment::new(0., 0., 100., 1., 1., 0.);
            ..bind_property("value", &keyboard_color, "hs")
                .transform_from(|_, value| {
                    let hs: &Hs = value.get_some().unwrap();
                    Some((hs.s * 100.).to_value())
                })
                .transform_to(|_, value| {
                    let s: f64 = value.get_some().unwrap();
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

        fn row(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::ListBoxRow {
            cascade! {
                gtk::ListBoxRow::new();
                ..set_selectable(false);
                ..set_activatable(false);
                ..set_property_margin(8);
                ..add(&cascade! {
                    gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    ..add(&cascade! {
                        gtk::Label::new(Some(label));
                        ..set_halign(gtk::Align::Start);
                    });
                    ..pack_end(widget, false, false, 0);
                });
            }
        }

        let mode_row = row("Layer Color Pattern:", &mode_combobox);
        let speed_row = row("Layer Animation Speed:", &speed_scale);
        let saturation_row = row("Layer Saturation:", &saturation_scale);
        let color_row = row("Layer Color:", &keyboard_color);

        cascade! {
            obj;
            ..set_valign(gtk::Align::Start);
            ..get_style_context().add_class("frame");
            ..add(&mode_row);
            ..add(&speed_row);
            ..add(&saturation_row);
            ..add(&color_row);
            ..add(&cascade! {
                row("Brightness (all layers):", &brightness_scale);
            });
        };

        self.keyboard_color.set(keyboard_color);
        self.color_row.set(color_row);
        self.brightness_scale.set(brightness_scale);
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
                glib::ParamSpec::string("mode", "mode", "mode", None, glib::ParamFlags::READABLE),
                glib::ParamSpec::int(
                    "selected",
                    "selected",
                    "selected",
                    -1,
                    i32::MAX,
                    -1,
                    glib::ParamFlags::WRITABLE,
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
        match pspec.get_name() {
            "selected" => {
                let v: i32 = value.get_some().unwrap();
                let selected = usize::try_from(v).ok();
                obj.inner().selected.set(selected);
                obj.update_per_key();
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.get_name() {
            "mode" => obj.mode().id.to_value(),
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
    pub fn new(board: DaemonBoard) -> Self {
        let max_brightness = board.max_brightness() as f64;
        let has_led_save = board.led_save().is_ok();

        let obj: Self = glib::Object::new(&[]).unwrap();
        obj.inner().board.set(board.clone());
        obj.inner().keyboard_color.set_index(obj.led_index());
        obj.inner().keyboard_color.set_board(Some(board));
        obj.inner().brightness_scale.set_range(0.0, max_brightness);
        obj.inner().has_led_save.set(has_led_save);
        obj.invalidate_filter();
        obj.set_layer(0);
        obj.set_filter_func(Some(Box::new(clone!(@weak obj => move |row|
            obj.filter_func(row)
        ))));
        obj.set_header_func(Some(Box::new(clone!(@weak obj => move |row, before|
            obj.header_func(row, before)
        ))));

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

    fn board(&self) -> &DaemonBoard {
        &self.inner().board
    }

    fn mode(&self) -> &'static Mode {
        if let Some(id) = self.inner().mode_combobox.get_active_id() {
            if let Some(mode) = Mode::from_id(id.as_str()) {
                return mode;
            }
        }
        &Mode::all()[0]
    }

    fn led_index(&self) -> u8 {
        let layer = self.inner().layer.get();
        if self.board().layout().meta.has_per_layer {
            0xf0 + layer
        } else {
            0xff
        }
    }

    fn header_func(&self, row: &gtk::ListBoxRow, before: Option<&gtk::ListBoxRow>) {
        if before.is_none() {
            row.set_header::<gtk::Widget>(None)
        } else if row.get_header().is_none() {
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
            self.mode().has_hue
        } else if row == &*inner.saturation_row {
            !self.mode().has_hue
        } else {
            true
        }
    }

    fn mode_speed_changed(&self) {
        self.notify("mode");

        if self.mode().is_per_key() {
            self.update_per_key();
        } else {
            self.inner().keyboard_color.set_sensitive(true);
            self.inner().keyboard_color.set_index(self.led_index());
        }
        self.invalidate_filter();

        if self.inner().do_not_set.get() {
            return;
        }

        let speed = self.inner().speed_scale.get_value();
        let layer = &self.board().layers()[self.inner().layer.get() as usize];
        if let Err(err) = layer.set_mode(self.mode().index, speed as u8) {
            error!("Error setting keyboard mode: {}", err);
        }
        self.inner().changed.set(true);
    }

    fn brightness_changed(&self) {
        if self.inner().do_not_set.get() {
            return;
        }
        let value = self.inner().brightness_scale.get_value() as i32;
        for layer in self.board().layers() {
            if let Err(err) = layer.set_brightness(value) {
                error!("Error setting brightness: {}", err);
            }
        }
        self.inner().changed.set(true);
        debug!("Brightness: {}", value)
    }

    pub fn set_layer(&self, layer: u8) {
        self.inner().layer.set(layer);

        let layer = &self.board().layers()[layer as usize];

        let (mode, speed) = layer.mode().unwrap_or((0, 128));
        let mode = Mode::from_index(mode).map(|x| x.id);
        let brightness = layer.brightness() as f64;

        self.inner().do_not_set.set(true);

        self.inner().mode_combobox.set_active_id(mode);
        self.inner().speed_scale.set_value(speed.into());
        self.inner().brightness_scale.set_value(brightness);
        self.inner().keyboard_color.set_index(self.led_index());

        self.inner().do_not_set.set(false);
    }

    fn update_per_key(&self) {
        if !self.mode().is_per_key() {
            return;
        }

        let mut sensitive = false;
        if let Some(selected) = self.inner().selected.get() {
            let k = &self.inner().board.keys()[selected];
            if !k.leds.is_empty() {
                sensitive = true;
                self.inner().keyboard_color.set_index(k.leds[0]);
            }
        }
        self.inner().keyboard_color.set_sensitive(sensitive);
    }

    fn led_save(&self) {
        if self.inner().has_led_save.get() && self.inner().changed.replace(false) {
            if let Err(err) = self.board().led_save() {
                error!("Failed to save LEDs: {}", err);
            } else {
                debug!("led_save");
            }
        }
    }
}
