use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use std::{cell::Cell, collections::HashMap, convert::TryFrom, rc::Rc};

use super::{Key, Layout};
use crate::{DaemonBoard, DerefCell, Hs, KeyboardColor};

struct Mode {
    index: u8,
    id: &'static str,
    name: &'static str,
    has_hue: bool,
    has_speed: bool,
}

impl Mode {
    const fn new(
        index: u8,
        id: &'static str,
        name: &'static str,
        has_hue: bool,
        has_speed: bool,
    ) -> Self {
        Self {
            index,
            id,
            name,
            has_hue,
            has_speed,
        }
    }

    fn is_per_key(&self) -> bool {
        self.index == 1
    }
}

static MODES: &[Mode] = &[
    Mode::new(0, "SOLID_COLOR", "Solid Color", true, false),
    Mode::new(1, "PER_KEY", "Per Key", true, false),
    Mode::new(2, "CYCLE_ALL", "Cosmic Background", false, true),
    Mode::new(3, "CYCLE_LEFT_RIGHT", "Horizonal Scan", false, true),
    Mode::new(4, "CYCLE_UP_DOWN", "Vertical Scan", false, true),
    Mode::new(5, "CYCLE_OUT_IN", "Event Horizon", false, true),
    Mode::new(6, "CYCLE_OUT_IN_DUAL", "Binary Galaxies", false, true),
    Mode::new(7, "RAINBOW_MOVING_CHEVRON", "Spacetime", false, true),
    Mode::new(8, "CYCLE_PINWHEEL", "Pinwheel Galaxy", false, true),
    Mode::new(9, "CYCLE_SPIRAL", "Spiral Galaxy", false, true),
    Mode::new(10, "RAINDROPS", "Elements", false, false),
    Mode::new(11, "SPLASH", "Splashdown", false, true),
    Mode::new(12, "MULTISPLASH", "Meteor Shower", false, true),
    Mode::new(13, "ACTIVE_KEYS", "Active Keys", true, false),
];

static MODE_BY_INDEX: Lazy<HashMap<u8, &Mode>> =
    Lazy::new(|| MODES.iter().map(|i| (i.index, i)).collect());

static MODE_BY_ID: Lazy<HashMap<&str, &Mode>> =
    Lazy::new(|| MODES.iter().map(|i| (i.id, i)).collect());

#[derive(Default)]
pub struct BacklightInner {
    board: DerefCell<DaemonBoard>,
    layout: DerefCell<Rc<Layout>>,
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
    keys: DerefCell<Rc<[Key]>>,
    selected: Cell<Option<usize>>,
}

#[glib::object_subclass]
impl ObjectSubclass for BacklightInner {
    const NAME: &'static str = "S76Backlight";
    type ParentType = gtk::ListBox;
    type Type = Backlight;
}

impl ObjectImpl for BacklightInner {
    fn constructed(&self, obj: &Self::Type) {
        let mode_combobox = cascade! {
            gtk::ComboBoxText::new();
            ..connect_changed(clone!(@weak obj => move |_|
                obj.mode_speed_changed();
            ));
        };

        for mode in MODES {
            mode_combobox.append(Some(mode.id), mode.name);
        }

        let speed_scale = cascade! {
            gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., 255., 1.);
            ..set_halign(gtk::Align::Fill);
            ..set_size_request(200, 0);
            ..connect_value_changed(clone!(@weak obj => move |_|
                obj.mode_speed_changed();
            ));
        };

        let brightness_scale = cascade! {
            gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., 100., 1.);
            ..set_halign(gtk::Align::Fill);
            ..set_size_request(200, 0);
            ..connect_value_changed(clone!(@weak obj => move |_|
                obj.brightness_changed();
            ));
        };

        let saturation_scale = cascade! {
            gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., 100., 1.);
            ..set_halign(gtk::Align::Fill);
            ..set_size_request(200, 0);
            ..connect_value_changed(clone!(@weak obj => move |_|
                obj.saturation_changed();
            ));
        };

        let keyboard_color = KeyboardColor::new(None, 0xf0);

        fn row(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::ListBoxRow {
            cascade! {
                gtk::ListBoxRow::new();
                ..set_selectable(false);
                ..set_activatable(false);
                ..set_margin_start(8);
                ..set_margin_end(8);
                ..add(&cascade! {
                    gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    ..add(&cascade! {
                        gtk::Label::new(Some(label));
                        ..set_halign(gtk::Align::Start);
                    });
                    ..add(widget);
                });
            }
        }

        let mode_row = cascade! {
            row("Mode:", &mode_combobox);
            ..set_margin_top(8);
        };

        let speed_row = row("Speed:", &speed_scale);
        let saturation_row = row("Saturation:", &saturation_scale);
        let color_row = row("Color:", &keyboard_color);

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
                ..set_margin_bottom(8);
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
    pub fn new(board: DaemonBoard, keys: Rc<[Key]>, layout: Rc<Layout>) -> Self {
        let max_brightness = match board.max_brightness() {
            Ok(value) => value as f64,
            Err(err) => {
                error!("Error getting brightness: {}", err);
                100.0
            }
        };

        let obj: Self = glib::Object::new(&[]).unwrap();
        obj.inner().keys.set(keys);
        obj.inner().keyboard_color.set_board(Some(board.clone()));
        obj.inner().brightness_scale.set_range(0.0, max_brightness);
        obj.inner().board.set(board.clone());
        obj.inner().layout.set(layout);
        obj.invalidate_filter();
        obj.set_layer(0);
        obj.set_filter_func(Some(Box::new(clone!(@weak obj => move |row| {
            let inner = obj.inner();
            if row == &*inner.mode_row {
                inner.layout.meta.has_mode
            } else if row == &*inner.speed_row {
                inner.layout.meta.has_mode && obj.mode().has_speed
            } else if row == &*inner.color_row {
                obj.mode().has_hue
            } else if row == &*inner.saturation_row {
                !obj.mode().has_hue
            } else {
                true
            }
        }))));
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
            if let Some(mode) = MODE_BY_ID.get(id.as_str()) {
                return *mode;
            }
        }
        &MODES[0]
    }

    fn led_index(&self) -> u8 {
        let layer = self.inner().layer.get();
        if self.inner().layout.meta.has_per_layer {
            0xf0 + layer
        } else {
            0xff
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
        let layer = self.inner().layer.get();
        if let Err(err) = self.board().set_mode(layer, self.mode().index, speed as u8) {
            error!("Error setting keyboard mode: {}", err);
        }
    }

    fn brightness_changed(&self) {
        if self.inner().do_not_set.get() {
            return;
        }
        let value = self.inner().brightness_scale.get_value() as i32;
        if self.inner().layout.meta.has_per_layer {
            for i in 0..self.inner().layout.meta.num_layers {
                if let Err(err) = self.board().set_brightness(0xf0 + i, value) {
                    error!("Error setting brightness: {}", err);
                }
            }
        } else {
            if let Err(err) = self.board().set_brightness(0xff, value) {
                error!("Error setting brightness: {}", err);
            }
        }
        debug!("Brightness: {}", value)
    }

    fn saturation_changed(&self) {
        if self.inner().do_not_set.get() {
            return;
        }

        let value = self.inner().saturation_scale.get_value();

        let hs = Hs::new(0., value / 100.);

        if let Err(err) = self.board().set_color(self.led_index(), hs) {
            error!("Error setting color: {}", err);
        }

        debug!("Saturation: {}", value)
    }

    pub fn set_layer(&self, layer: u8) {
        self.inner().layer.set(layer);

        let (mode, speed) = if self.inner().layout.meta.has_mode {
            self.board().mode(layer).unwrap_or_else(|err| {
                error!("Error getting keyboard mode: {}", err);
                (0, 128)
            })
        } else {
            (0, 128)
        };

        let mode = MODE_BY_INDEX.get(&mode).map(|x| x.id);

        let brightness = match self.board().brightness(self.led_index()) {
            Ok(value) => value as f64,
            Err(err) => {
                error!("{}", err);
                0.0
            }
        };

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
            let k = &self.inner().keys[selected];
            if !k.leds.is_empty() {
                sensitive = true;
                self.inner().keyboard_color.set_index(k.leds[0]);
            }
        }
        self.inner().keyboard_color.set_sensitive(sensitive);
    }
}
