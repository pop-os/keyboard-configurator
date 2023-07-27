use cascade::cascade;
use futures::{prelude::*, stream::FuturesUnordered};
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;
use std::{cell::RefCell, collections::HashMap};

use crate::Keyboard;
use backend::DerefCell;

mod picker_group;
mod picker_group_box;
mod picker_json;
mod picker_key;

use picker_group_box::PickerGroupBox;
use picker_json::picker_json;
use picker_key::PickerKey;

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
    group_box: DerefCell<PickerGroupBox>,
    keyboard: RefCell<Option<glib::WeakRef<Keyboard>>>,
}

#[glib::object_subclass]
impl ObjectSubclass for PickerInner {
    const NAME: &'static str = "S76KeyboardPicker";
    type ParentType = gtk::Box;
    type Type = Picker;
}

impl ObjectImpl for PickerInner {
    fn constructed(&self) {
        self.parent_constructed();

        let picker = self.obj();

        let group_box = cascade! {
            PickerGroupBox::new();
            ..connect_key_pressed(clone!(@weak picker => move |name| {
                picker.key_pressed(name)
            }));
        };

        cascade! {
            picker;
            ..add(&group_box);
            ..show_all();
        };

        self.group_box.set(group_box);
    }
}

impl BoxImpl for PickerInner {}

impl WidgetImpl for PickerInner {}

impl ContainerImpl for PickerInner {}

glib::wrapper! {
    pub struct Picker(ObjectSubclass<PickerInner>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

impl Picker {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn inner(&self) -> &PickerInner {
        PickerInner::from_obj(self)
    }

    fn keyboard(&self) -> Option<Keyboard> {
        self.inner()
            .keyboard
            .borrow()
            .as_ref()
            .and_then(|x| x.upgrade())
    }

    pub(crate) fn set_keyboard(&self, keyboard: Option<Keyboard>) {
        if let Some(old_kb) = self.keyboard() {
            old_kb.set_picker(None);
        }

        if let Some(widget) = self.parent() {
            widget.downcast::<gtk::Container>().unwrap().remove(self);
        }

        if let Some(kb) = &keyboard {
            // Check that scancode is available for the keyboard
            self.inner()
                .group_box
                .set_key_visibility(|name| kb.has_scancode(name));
            kb.set_picker(Some(self));
        }

        *self.inner().keyboard.borrow_mut() = keyboard.map(|x| x.downgrade());
    }

    pub(crate) fn set_selected(&self, scancode_names: Vec<String>) {
        self.inner().group_box.set_selected(scancode_names);
    }

    fn key_pressed(&self, name: String) {
        let kb = match self.keyboard() {
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
                futures.push(clone!(@strong kb, @strong name => async move {
                    kb.keymap_set(i, layer, &name).await;
                }));
            }
            glib::MainContext::default().spawn_local(async { futures.collect::<()>().await });
        }
    }
}

impl Default for Picker {
    fn default() -> Self {
        Self::new()
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
            let layout = Layout::from_board(i, "dummy").unwrap();
            for j in layout.default.map.values().flatten() {
                if SCANCODE_LABELS.keys().find(|x| x == &j).is_none() {
                    missing.insert(j.to_owned());
                }
            }
        }
        assert_eq!(missing, HashSet::new());
    }
}
