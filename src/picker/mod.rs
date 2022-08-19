use cascade::cascade;
use futures::{prelude::*, stream::FuturesUnordered};
use gtk::{
    gdk,
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use crate::Keyboard;
use backend::{DerefCell, Keycode, Mods};

mod picker_group;
mod picker_group_box;
mod picker_json;
mod picker_key;
mod tap_hold;

use picker_group_box::PickerGroupBox;
use picker_json::picker_json;
use picker_key::PickerKey;
use tap_hold::TapHold;

pub use tap_hold::LAYERS;

pub static SCANCODE_LABELS: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let mut labels = HashMap::new();
    for group in picker_json() {
        for key in group.keys {
            labels.insert(key.keysym, key.label);
        }
    }
    labels
});

#[derive(Default)]
pub struct PickerInner {
    group_boxes: DerefCell<Vec<PickerGroupBox>>,
    keyboard: RefCell<Option<Keyboard>>,
    event_controller_key: RefCell<Option<gtk::EventControllerKey>>,
    selected: RefCell<Vec<Keycode>>,
    shift: Cell<bool>,
    tap_hold: DerefCell<TapHold>,
}

#[glib::object_subclass]
impl ObjectSubclass for PickerInner {
    const NAME: &'static str = "S76KeyboardPicker";
    type ParentType = gtk::Box;
    type Type = Picker;
}

impl ObjectImpl for PickerInner {
    fn constructed(&self, picker: &Picker) {
        self.parent_constructed(picker);

        let basics_group_box = cascade! {
            PickerGroupBox::new("basics");
            ..connect_key_pressed(clone!(@weak picker => move |name, shift| {
                picker.key_pressed(name, shift)
            }));
        };

        let extras_group_box = cascade! {
            PickerGroupBox::new("extras");
            ..connect_key_pressed(clone!(@weak picker => move |name, shift| {
                picker.key_pressed(name, shift)
            }));
        };

        let tap_hold = cascade! {
            tap_hold::TapHold::new();
            ..connect_selected(clone!(@weak picker => move |keycode| {
                picker.set_keycode(keycode);
            }));
        };

        // XXX translate
        let stack = cascade! {
            gtk::Stack::new();
            ..add_titled(&basics_group_box, "basics", "Basics");
            ..add_titled(&extras_group_box, "extras", "Extras");
            ..add_titled(&tap_hold, "tap-hold", "Tap-Hold");
        };

        let stack_switcher = cascade! {
            gtk::StackSwitcher::new();
            ..set_stack(Some(&stack));
        };

        cascade! {
            picker;
            ..set_orientation(gtk::Orientation::Vertical);
            ..add(&stack_switcher);
            ..add(&stack);
            ..show_all();
        };

        self.group_boxes
            .set(vec![basics_group_box, extras_group_box]);
        self.tap_hold.set(tap_hold);
    }
}

impl BoxImpl for PickerInner {}

impl WidgetImpl for PickerInner {
    fn realize(&self, widget: &Self::Type) {
        self.parent_realize(widget);

        let window = widget
            .toplevel()
            .and_then(|x| x.downcast::<gtk::Window>().ok());
        *self.event_controller_key.borrow_mut() = window.map(|window| {
            cascade! {
                 gtk::EventControllerKey::new(&window);
                 ..connect_key_pressed(clone!(@weak widget => @default-return true, move |_, keyval, _, mods| {
                     let key = gdk::keys::Key::from(keyval);
                     if key == gdk::keys::constants::Shift_L || key == gdk::keys::constants::Shift_R {
                         println!("Shift"); // XXX what if only one is held?
                     }
                     true
                 }));
                 ..connect_key_released(clone!(@weak widget => move |_, keyval, _, mods| {
                     let key = gdk::keys::Key::from(keyval);
                     if key == gdk::keys::constants::Shift_L || key == gdk::keys::constants::Shift_R {
                         println!("Unshift"); // XXX what if only one is held?
                     }
                 }));
                 ..connect_focus_out(clone!(@weak widget => move |_| {
                     println!("Unfocus");
                 }));
                 ..connect_modifiers(clone!(@weak widget => @default-return true, move |_, mods| {
                     println!("Mods: {:?}", mods);
                     let shift = mods.contains(gdk::ModifierType::SHIFT_MASK);
                     //println!("Shift: {}", shift);
                     if shift != widget.inner().shift.get() {
                        widget.inner().shift.set(shift);
                        widget.invalidate_sensitivity();
                     }
                     true
                 }));
            }
        });
    }

    fn unrealize(&self, widget: &Self::Type) {
        self.parent_unrealize(widget);
        *self.event_controller_key.borrow_mut() = None;
    }
}

impl ContainerImpl for PickerInner {}

glib::wrapper! {
    pub struct Picker(ObjectSubclass<PickerInner>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

impl Picker {
    pub fn new() -> Self {
        glib::Object::new(&[]).unwrap()
    }

    fn inner(&self) -> &PickerInner {
        PickerInner::from_instance(self)
    }

    pub(crate) fn set_keyboard(&self, keyboard: Option<Keyboard>) {
        if let Some(old_kb) = &*self.inner().keyboard.borrow() {
            old_kb.set_picker(None);
        }

        if let Some(kb) = &keyboard {
            // Check that scancode is available for the keyboard
            for group_box in self.inner().group_boxes.iter() {
                group_box.set_key_visibility(|name| {
                    kb.has_scancode(&Keycode::Basic(Mods::empty(), name.to_string()))
                });
            }
            kb.set_picker(Some(&self));
        }

        *self.inner().keyboard.borrow_mut() = keyboard;
    }

    pub(crate) fn set_selected(&self, scancode_names: Vec<Keycode>) {
        for group_box in self.inner().group_boxes.iter() {
            group_box.set_selected(scancode_names.clone());
        }
        self.inner().tap_hold.set_selected(scancode_names.clone());
        *self.inner().selected.borrow_mut() = scancode_names;
    }

    fn key_pressed(&self, name: String, shift: bool) {
        let mod_ = Mods::from_mod_str(&name);
        if shift {
            let selected = self.inner().selected.borrow();
            if selected.len() == 1 {
                if let Keycode::Basic(mods, scancode_name) = &selected[0] {
                    if let Some(mod_) = mod_ {
                        self.set_keycode(Keycode::Basic(
                            mods.toggle_mod(mod_),
                            scancode_name.to_string(),
                        ));
                        return;
                    } else if scancode_name == "NONE" {
                        self.set_keycode(Keycode::Basic(*mods, name));
                        return;
                    }
                }
            }
        }
        let keycode = if let Some(mod_) = mod_ {
            Keycode::Basic(mod_, "NONE".to_string())
        } else {
            Keycode::Basic(Mods::empty(), name)
        };
        self.set_keycode(keycode);
    }

    fn set_keycode(&self, keycode: Keycode) {
        let kb = match self.inner().keyboard.borrow().clone() {
            Some(kb) => kb,
            None => {
                return;
            }
        };

        let layer = kb.layer();
        if let Some(layer) = layer {
            let futures = FuturesUnordered::new();
            for i in kb.selected().iter() {
                let i = *i;
                futures.push(clone!(@strong kb, @strong keycode => async move {
                    kb.keymap_set(i, layer, &keycode).await;
                }));
            }
            glib::MainContext::default().spawn_local(async { futures.collect::<()>().await });
        }
    }

    fn invalidate_sensitivity(&self) {
        return;

        let shift = self.inner().shift.get();

        let mut allow_mods = true;
        let mut allow_basic = true;
        let mut allow_non_basic = true;

        if shift {
            let selected = self.inner().selected.borrow();
            if selected.len() == 1 {
                match &selected[0] {
                    Keycode::Basic(mods, keycode) => {
                        // Allow mods only if `keycode` is really basic?
                        allow_basic = keycode == "NONE";
                        allow_non_basic = false;
                    }
                    Keycode::MT(..) | Keycode::LT(..) => {
                        allow_mods = false;
                        allow_basic = false;
                        allow_non_basic = false;
                    }
                }
            }
        }

        for group_box in self.inner().group_boxes.iter() {
            // TODO: What to allow?
            group_box.set_key_sensitivity(|name| {
                if [
                    "LEFT_SHIFT",
                    "RIGHT_SHIFT",
                    "LEFT_ALT",
                    "RIGHT_ALT",
                    "LEFT_CTRL",
                    "RIGHT_CTRL",
                    "LEFT_SUPER",
                    "RIGHT_SUPER",
                ]
                .contains(&name)
                {
                    allow_mods
                } else {
                    allow_basic
                }
                // XXX non-basic?
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use backend::{layouts, Layout};
    use std::collections::HashSet;

    #[test]
    fn picker_has_keys() {
        let mut missing = HashSet::new();
        for i in layouts() {
            let layout = Layout::from_board(i).unwrap();
            for j in layout.default.map.values().flatten() {
                if SCANCODE_LABELS.keys().find(|x| x == &j).is_none() {
                    missing.insert(j.to_owned());
                }
            }
        }
        assert_eq!(missing, HashSet::new());
    }
}
