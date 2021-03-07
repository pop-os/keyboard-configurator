use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::Cell;

use crate::{DaemonBoard, DerefCell, KeyboardColor};

static MODE_MAP: &[&str] = &[
    "SOLID_COLOR",
    "PER_KEY",
    "CYCLE_ALL",
    "CYCLE_LEFT_RIGHT",
    "CYCLE_UP_DOWN",
    "CYCLE_OUT_IN",
    "CYCLE_OUT_IN_DUAL",
    "RAINBOW_MOVING_CHEVRON",
    "CYCLE_PINWHEEL",
    "CYCLE_SPIRAL",
    "RAINDROPS",
    "SPLASH",
    "MULTISPLASH",
    "ACTIVE_KEYS",
];

#[derive(Default)]
pub struct BacklightInner {
    board: DerefCell<DaemonBoard>,
    keyboard_color: DerefCell<KeyboardColor>,
    brightness_scale: DerefCell<gtk::Scale>,
    mode_combobox: DerefCell<gtk::ComboBoxText>,
    speed_scale: DerefCell<gtk::Scale>,
    layer: Cell<u8>,
    do_not_set: Cell<bool>,
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
            ..append(Some("SOLID_COLOR"), "Solid Color");
            ..append(Some("PER_KEY"), "Per Key");
            ..append(Some("CYCLE_ALL"), "Cosmic Background");
            ..append(Some("CYCLE_LEFT_RIGHT"), "Horizonal Scan");
            ..append(Some("CYCLE_UP_DOWN"), "Vertical Scan");
            ..append(Some("CYCLE_OUT_IN"), "Event Horizon");
            ..append(Some("CYCLE_OUT_IN_DUAL"), "Binary Galaxies");
            ..append(Some("RAINBOW_MOVING_CHEVRON"), "Spacetime");
            ..append(Some("CYCLE_PINWHEEL"), "Pinwheel Galaxy");
            ..append(Some("CYCLE_SPIRAL"), "Spiral Galaxy");
            ..append(Some("RAINDROPS"), "Elements");
            ..append(Some("SPLASH"), "Splashdown");
            ..append(Some("MULTISPLASH"), "Meteor Shower");
            ..append(Some("ACTIVE_KEYS"), "Active Keys");
            ..connect_changed(clone!(@weak obj => move |_|
                obj.mode_speed_changed();
            ));
        };

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

        cascade! {
            obj;
            ..set_valign(gtk::Align::Start);
            ..get_style_context().add_class("frame");
            ..add(&cascade! {
                row("Mode:", &mode_combobox);
                ..set_margin_top(8);
            });
            ..add(&row("Speed:", &speed_scale));
            ..add(&row("Brightness:", &brightness_scale));
            ..add(&cascade! {
                row("Color:", &keyboard_color);
                ..set_margin_bottom(8);
            });
        };

        self.keyboard_color.set(keyboard_color);
        self.brightness_scale.set(brightness_scale);
        self.mode_combobox.set(mode_combobox);
        self.speed_scale.set(speed_scale);
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
        let max_brightness = match board.max_brightness() {
            Ok(value) => value as f64,
            Err(err) => {
                error!("{}", err);
                100.0
            }
        };

        let obj: Self = glib::Object::new(&[]).unwrap();
        obj.inner().keyboard_color.set_board(Some(board.clone()));
        obj.inner().brightness_scale.set_range(0.0, max_brightness);
        obj.inner().board.set(board.clone());
        obj.set_layer(0);
        obj
    }

    fn inner(&self) -> &BacklightInner {
        BacklightInner::from_instance(self)
    }

    fn board(&self) -> &DaemonBoard {
        &self.inner().board
    }

    fn mode_speed_changed(&self) {
        if self.inner().do_not_set.get() {
            return;
        }
        if let Some(id) = self.inner().mode_combobox.get_active_id() {
            if let Some(mode) = MODE_MAP.iter().position(|i| id == **i) {
                let speed = self.inner().speed_scale.get_value();
                let layer = self.inner().layer.get();
                if let Err(err) = self.board().set_mode(layer, mode as u8, speed as u8) {
                    error!("Error setting keyboard mode: {}", err);
                }
            }
        }
    }

    fn brightness_changed(&self) {
        if self.inner().do_not_set.get() {
            return;
        }
        let value = self.inner().brightness_scale.get_value() as i32;
        let layer = self.inner().layer.get();
        if let Err(err) = self.board().set_brightness(0xf0 + layer, value) {
            error!("{}", err);
        }
        debug!("Brightness: {}", value)
    }

    pub fn set_layer(&self, layer: u8) {
        let (mode, speed) = self.board().mode(layer).unwrap_or_else(|err| {
            error!("Error getting keyboard mode: {}", err);
            (0, 128)
        });

        let mode = MODE_MAP.get(mode as usize).cloned();

        let brightness = match self.board().brightness(0xf0 + layer) {
            Ok(value) => value as f64,
            Err(err) => {
                error!("{}", err);
                0.0
            }
        };

        self.inner().do_not_set.set(true);

        self.inner().layer.set(layer);
        self.inner().mode_combobox.set_active_id(mode);
        self.inner().speed_scale.set_value(speed.into());
        self.inner().brightness_scale.set_value(brightness);
        self.inner().keyboard_color.set_index(0xf0 + layer);

        self.inner().do_not_set.set(false);
    }
}
