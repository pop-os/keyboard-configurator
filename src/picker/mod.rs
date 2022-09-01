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
use backend::{is_qmk_basic, DerefCell, Keycode, Mods};

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
    stack_switcher: DerefCell<gtk::StackSwitcher>,
    basics_group_box: DerefCell<PickerGroupBox>,
    extras_group_box: DerefCell<PickerGroupBox>,
    keyboard: RefCell<Option<Keyboard>>,
    selected: RefCell<Vec<Keycode>>,
    shift: Cell<bool>,
    tap_hold: DerefCell<TapHold>,
    is_qmk: Cell<bool>,
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
            ..connect_select(clone!(@weak picker => move |keycode| {
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

        self.stack_switcher.set(stack_switcher);
        self.basics_group_box.set(basics_group_box);
        self.extras_group_box.set(extras_group_box);
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
        if let Some(window) = &window {
            window.add_events(gdk::EventMask::FOCUS_CHANGE_MASK);
            window.connect_event(clone!(@weak widget => @default-return Inhibit(false), move |_, evt| {
                use gdk::keys::{Key, constants};
                let is_shift_key = matches!(evt.keyval().map(Key::from), Some(constants::Shift_L | constants::Shift_R));
                // XXX Distinguish lshift, rshift if both are held?
                let shift = match evt.event_type() {
                    gdk::EventType::KeyPress if is_shift_key => true,
                    gdk::EventType::KeyRelease if is_shift_key => false,
                    gdk::EventType::FocusChange => false,
                    _ => { return Inhibit(false); }
                };
                widget.inner().shift.set(shift);
                widget.invalidate_sensitivity();
                widget.inner().tap_hold.set_shift(shift);
                Inhibit(false)
            }));
        }
    }

    fn unrealize(&self, widget: &Self::Type) {
        self.parent_unrealize(widget);
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

    fn group_boxes(&self) -> [&PickerGroupBox; 2] {
        [
            &*self.inner().basics_group_box,
            &*self.inner().extras_group_box,
        ]
    }

    pub(crate) fn set_keyboard(&self, keyboard: Option<Keyboard>) {
        if let Some(old_kb) = &*self.inner().keyboard.borrow() {
            old_kb.set_picker(None);
        }

        if let Some(kb) = &keyboard {
            // Check that scancode is available for the keyboard
            for group_box in self.group_boxes() {
                group_box.set_key_visibility(|name| kb.layout().has_scancode(name));
            }
            let is_qmk = kb.layout().meta.is_qmk;
            self.inner().extras_group_box.set_visible(is_qmk);
            self.inner().tap_hold.set_visible(is_qmk);
            self.inner().stack_switcher.set_visible(is_qmk);
            self.inner().is_qmk.set(is_qmk);
            kb.set_picker(Some(&self));
        }

        *self.inner().keyboard.borrow_mut() = keyboard;
    }

    pub(crate) fn set_selected(&self, scancode_names: Vec<Keycode>) {
        for group_box in self.group_boxes() {
            group_box.set_selected(scancode_names.clone());
        }
        self.inner().tap_hold.set_selected(scancode_names.clone());
        *self.inner().selected.borrow_mut() = scancode_names;

        self.invalidate_sensitivity();
    }

    fn key_pressed(&self, name: String, shift: bool) {
        let mod_ = Mods::from_mod_str(&name);
        if shift && self.inner().is_qmk.get() {
            let selected = self.inner().selected.borrow();
            if selected.len() == 1 {
                if let Keycode::Basic(mods, scancode_name) = &selected[0] {
                    if let Some(mod_) = mod_ {
                        self.set_keycode(Keycode::Basic(
                            mods.toggle_mod(mod_),
                            scancode_name.to_string(),
                        ));
                        return;
                    } else if scancode_name == &name && !mods.is_empty() {
                        self.set_keycode(Keycode::Basic(*mods, "NONE".to_string()));
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
        let shift = self.inner().shift.get();

        let mut allow_left_mods = false;
        let mut allow_right_mods = false;
        let mut allow_basic = false;
        let mut allow_non_basic = false;

        let mut keycode_mods = Mods::empty();
        let mut basic_keycode = None;

        if shift && self.inner().is_qmk.get() {
            let selected = self.inner().selected.borrow();
            if selected.len() == 1 {
                match &selected[0] {
                    Keycode::Basic(mods, keycode) => {
                        // Allow mods only if `keycode` is really basic?
                        // Allow deselecting current key
                        let no_mod = mods.is_empty();
                        let right = mods.contains(Mods::RIGHT);
                        allow_left_mods = no_mod || !right;
                        allow_right_mods = no_mod || right;
                        allow_basic = keycode == "NONE" && !mods.is_empty();
                        keycode_mods = *mods;
                        basic_keycode = Some(keycode.clone());
                    }
                    Keycode::MT(..) | Keycode::LT(..) => {}
                }
            }
        } else {
            allow_left_mods = true;
            allow_right_mods = true;
            allow_basic = true;
            allow_non_basic = true;
        }

        for group_box in self.group_boxes() {
            group_box.set_key_sensitivity(|name| {
                if ["LEFT_SHIFT", "LEFT_ALT", "LEFT_CTRL", "LEFT_SUPER"].contains(&name) {
                    allow_left_mods
                } else if ["RIGHT_SHIFT", "RIGHT_ALT", "RIGHT_CTRL", "RIGHT_SUPER"].contains(&name)
                {
                    allow_right_mods
                } else if basic_keycode.as_deref() == Some(name) && !keycode_mods.is_empty() {
                    true
                } else if is_qmk_basic(name) {
                    allow_basic
                } else {
                    allow_non_basic
                }
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
