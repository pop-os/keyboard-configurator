use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{cell::Cell, rc::Rc};

use super::Layout;
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
    layout: DerefCell<Rc<Layout>>,
    keyboard_color: DerefCell<KeyboardColor>,
    brightness_scale: DerefCell<gtk::Scale>,
    mode_combobox: DerefCell<gtk::ComboBoxText>,
    mode_row: DerefCell<gtk::ListBoxRow>,
    speed_scale: DerefCell<gtk::Scale>,
    speed_row: DerefCell<gtk::ListBoxRow>,
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

        let mode_row = cascade! {
            row("Mode:", &mode_combobox);
            ..set_margin_top(8);
            ..show_all();
            ..set_no_show_all(true);
        };

        let speed_row = cascade! {
            row("Speed:", &speed_scale);
            ..show_all();
            ..set_no_show_all(true);
        };

        cascade! {
            obj;
            ..set_valign(gtk::Align::Start);
            ..get_style_context().add_class("frame");
            ..add(&mode_row);
            ..add(&speed_row);
            ..add(&cascade! {
                row("Color:", &keyboard_color);
                ..set_margin_bottom(8);
            });
            ..add(&row("Brightness (all layers):", &brightness_scale));
        };

        self.keyboard_color.set(keyboard_color);
        self.brightness_scale.set(brightness_scale);
        self.mode_combobox.set(mode_combobox);
        self.mode_row.set(mode_row);
        self.speed_scale.set(speed_scale);
        self.speed_row.set(speed_row);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpec::string(
                "mode",
                "mode",
                "mode",
                None,
                glib::ParamFlags::READABLE,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn get_property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.get_name() {
            "mode" => obj.mode().to_value(),
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
    pub fn new(board: DaemonBoard, layout: Rc<Layout>) -> Self {
        let max_brightness = match board.max_brightness() {
            Ok(value) => value as f64,
            Err(err) => {
                error!("Error getting brightness: {}", err);
                100.0
            }
        };

        let obj: Self = glib::Object::new(&[]).unwrap();
        obj.inner().keyboard_color.set_board(Some(board.clone()));
        obj.inner().brightness_scale.set_range(0.0, max_brightness);
        obj.inner().board.set(board.clone());
        obj.inner().layout.set(layout);
        let has_mode = obj.inner().layout.meta.has_mode;
        obj.inner().mode_row.set_visible(has_mode);
        obj.inner().speed_row.set_visible(has_mode);
        obj.set_layer(0);
        obj
    }

    fn inner(&self) -> &BacklightInner {
        BacklightInner::from_instance(self)
    }

    fn board(&self) -> &DaemonBoard {
        &self.inner().board
    }

    pub fn mode(&self) -> Option<String> {
        self.inner()
            .mode_combobox
            .get_active_id()
            .map(|x| x.to_string())
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

        if self.inner().do_not_set.get() {
            return;
        }
        if let Some(id) = self.mode() {
            if let Some(mode) = MODE_MAP.iter().position(|i| id == *i) {
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

        let mode = MODE_MAP.get(mode as usize).cloned();

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
}
