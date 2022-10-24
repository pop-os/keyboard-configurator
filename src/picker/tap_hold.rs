use cascade::cascade;
use gtk::{
    glib::{self, clone, subclass::Signal},
    pango,
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;
use std::cell::{Cell, RefCell};

use super::{group_box::PickerBasicGroup, PickerGroupBox};
use backend::{is_qmk_basic, DerefCell, Keycode, Mods};

#[derive(Clone, Copy, PartialEq)]
enum Hold {
    Mods(Mods),
    Layer(u8),
}

impl Default for Hold {
    fn default() -> Self {
        Self::Mods(Mods::default())
    }
}

static MODIFIERS: &[&str] = &[
    "LEFT_SHIFT",
    "LEFT_CTRL",
    "LEFT_SUPER",
    "LEFT_ALT",
    "RIGHT_SHIFT",
    "RIGHT_CTRL",
    "RIGHT_SUPER",
    "RIGHT_ALT",
];
pub static LAYERS: &[&str] = &["LAYER_ACCESS_1", "FN", "LAYER_ACCESS_3", "LAYER_ACCESS_4"];

#[derive(Default)]
pub struct TapHoldInner {
    shift: Cell<bool>,
    hold: Cell<Hold>,
    keycode: RefCell<Option<String>>,
    hold_group_box: DerefCell<PickerGroupBox>,
    picker_group_box: DerefCell<PickerGroupBox>,
}

#[glib::object_subclass]
impl ObjectSubclass for TapHoldInner {
    const NAME: &'static str = "S76KeyboardTapHold";
    type ParentType = gtk::Box;
    type Type = TapHold;
}

impl ObjectImpl for TapHoldInner {
    fn signals() -> &'static [Signal] {
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![Signal::builder(
                "select",
                &[Keycode::static_type().into()],
                glib::Type::UNIT.into(),
            )
            .build()]
        });
        SIGNALS.as_ref()
    }

    fn constructed(&self, widget: &Self::Type) {
        self.parent_constructed(widget);

        let picker_group_box = cascade! {
            PickerGroupBox::basics();
            ..set_sensitive(false);
            ..connect_key_pressed(clone!(@weak widget => move |name, _shift| {
                *widget.inner().keycode.borrow_mut() = Some(name);
                widget.update();
            }));
            ..set_key_visibility(|name| is_qmk_basic(name));
        };

        let hold_group_box = cascade! {
            PickerGroupBox::new(vec![
                Box::new(PickerBasicGroup::new(
                    "Modifiers".to_string(),
                    4,
                    1.5,
                    MODIFIERS,
                )),
                Box::new(PickerBasicGroup::new(
                    "Layer Keys".to_string(),
                    4,
                    1.5,
                    LAYERS,
                )),
            ]);
            ..connect_key_pressed(clone!(@weak widget => move |name, shift| {
                let new_hold = if let Some(mod_) = Mods::from_mod_str(&name) {
                    let mut new_mods = mod_;
                    if shift {
                        if let Hold::Mods(mods) = widget.inner().hold.get() {
                            new_mods = mods.toggle_mod(mod_);
                        }
                    }
                    Hold::Mods(new_mods)
                } else {
                    let n = LAYERS.iter().position(|x| *x == &name).unwrap() as u8;
                    Hold::Layer(n)
                };
                widget.inner().hold.set(new_hold);
                widget.update();
            }));
        };

        cascade! {
            widget;
            ..set_spacing(8);
            ..set_orientation(gtk::Orientation::Vertical);
            ..add(&cascade! {
                gtk::Label::new(Some("1. Select action(s) to use when the key is held."));
                ..set_attributes(Some(&cascade! {
                    pango::AttrList::new();
                    ..insert(pango::AttrInt::new_weight(pango::Weight::Bold));
                }));
                ..set_halign(gtk::Align::Start);
            });
            ..add(&hold_group_box);
            ..add(&cascade! {
                gtk::Label::new(Some("Shift + click to select multiple modifiers."));
                ..set_halign(gtk::Align::Start);
            });
            // XXX grey?
            ..add(&cascade! {
                gtk::Label::new(Some("2. Select an action to use when the key is tapped."));
                ..set_attributes(Some(&cascade! {
                    pango::AttrList::new();
                    ..insert(pango::AttrInt::new_weight(pango::Weight::Bold));
                }));
                ..set_halign(gtk::Align::Start);
            });
            ..add(&picker_group_box);
        };

        self.hold_group_box.set(hold_group_box);
        self.picker_group_box.set(picker_group_box);
    }
}

impl BoxImpl for TapHoldInner {}
impl WidgetImpl for TapHoldInner {}
impl ContainerImpl for TapHoldInner {}

glib::wrapper! {
    pub struct TapHold(ObjectSubclass<TapHoldInner>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

impl TapHold {
    pub fn new() -> Self {
        glib::Object::new(&[]).unwrap()
    }

    fn inner(&self) -> &TapHoldInner {
        TapHoldInner::from_instance(self)
    }

    fn update(&self) {
        let keycode = self.inner().keycode.borrow();
        let keycode = keycode.as_deref().unwrap_or("NONE");
        match self.inner().hold.get() {
            Hold::Mods(mods) => {
                if !mods.is_empty() {
                    self.emit_by_name::<()>("select", &[&Keycode::MT(mods, keycode.to_string())]);
                }
            }
            Hold::Layer(layer) => {
                self.emit_by_name::<()>("select", &[&Keycode::LT(layer, keycode.to_string())]);
            }
        }
    }

    pub fn connect_select<F: Fn(Keycode) + 'static>(&self, cb: F) -> glib::SignalHandlerId {
        self.connect_local("select", false, move |values| {
            cb(values[1].get::<Keycode>().unwrap());
            None
        })
    }

    pub fn set_selected(&self, scancode_names: Vec<Keycode>) {
        // XXX how to handle > 1?
        let (mods, layer, keycode) = if scancode_names.len() == 1 {
            match scancode_names.into_iter().next().unwrap() {
                Keycode::MT(mods, keycode) => (mods, None, Some(keycode)),
                Keycode::LT(layer, keycode) => (Mods::empty(), Some(layer), Some(keycode)),
                Keycode::Basic(..) => Default::default(),
            }
        } else {
            Default::default()
        };

        let mut selected_hold = Vec::new();
        for i in MODIFIERS {
            let mod_ = Mods::from_mod_str(i).unwrap();
            if mods.contains(mod_) && (mods.contains(Mods::RIGHT) == mod_.contains(Mods::RIGHT)) {
                selected_hold.push(Keycode::Basic(mod_, "NONE".to_string()));
            }
        }
        if let Some(layer) = layer {
            selected_hold.push(Keycode::Basic(
                Mods::empty(),
                LAYERS[layer as usize].to_string(),
            ));
        }
        self.inner().hold_group_box.set_selected(selected_hold);

        if let Some(keycode) = keycode.clone() {
            self.inner()
                .picker_group_box
                .set_selected(vec![Keycode::Basic(Mods::empty(), keycode)]);
        } else {
            self.inner().picker_group_box.set_selected(Vec::new());
        }

        self.inner().hold.set(if let Some(layer) = layer {
            Hold::Layer(layer)
        } else {
            Hold::Mods(mods)
        });
        *self.inner().keycode.borrow_mut() = keycode;

        self.invalidate_sensitivity();
    }

    pub fn set_shift(&self, shift: bool) {
        self.inner().shift.set(shift);
        self.invalidate_sensitivity();
    }

    fn invalidate_sensitivity(&self) {
        let shift = self.inner().shift.get();
        let hold = self.inner().hold.get();
        let hold_empty = hold == Hold::Mods(Mods::empty());
        let keycode = self.inner().keycode.borrow();

        self.inner().hold_group_box.set_key_sensitivity(|name| {
            let left_mod = name.starts_with("LEFT_");
            let right_mod = name.starts_with("RIGHT_");
            // Modifer
            if left_mod || right_mod {
                if shift {
                    match hold {
                        Hold::Mods(mods) => {
                            mods.is_empty() || (right_mod == mods.contains(Mods::RIGHT))
                        }
                        Hold::Layer(_) => false,
                    }
                } else {
                    true
                }
            // Layer
            } else {
                !shift || (hold == Hold::Mods(Mods::empty()))
            }
        });

        self.inner().picker_group_box.set_sensitive(if shift {
            !hold_empty && keycode.is_none()
        } else {
            !hold_empty
        });
    }
}
