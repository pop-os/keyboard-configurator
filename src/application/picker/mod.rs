use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::Keyboard;
use crate::DerefCell;

mod picker_group;
mod picker_json;
mod picker_key;

use picker_group::PickerGroup;
use picker_json::picker_json;
use picker_key::PickerKey;

const DEFAULT_COLS: i32 = 3;
const PICKER_CSS: &str = r#"
button {
    margin: 0;
    padding: 0;
}

.selected {
    border-color: #fbb86c;
    border-width: 4px;
}
"#;

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
    groups: DerefCell<Vec<PickerGroup>>,
    keys: DerefCell<HashMap<String, Rc<PickerKey>>>,
    keyboard: RefCell<Option<Keyboard>>,
    selected: RefCell<Option<String>>,
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

        let style_provider = cascade! {
            gtk::CssProvider::new();
            ..load_from_data(&PICKER_CSS.as_bytes()).expect("Failed to parse css");
        };

        let mut groups = Vec::new();
        let mut keys = HashMap::new();

        for json_group in picker_json() {
            let mut group = PickerGroup::new(json_group.label, json_group.cols);

            for json_key in json_group.keys {
                let key = PickerKey::new(
                    json_key.keysym.clone(),
                    json_key.label,
                    json_group.width,
                    &style_provider,
                );

                group.add_key(key.clone());
                keys.insert(json_key.keysym, key);
            }

            groups.push(group);
        }

        self.keys.set(keys);
        self.groups.set(groups);

        let mut picker_hbox_opt: Option<gtk::Box> = None;
        let mut picker_col = 0;
        let picker_cols = DEFAULT_COLS;

        for group in &*picker.inner().groups {
            let picker_hbox = match picker_hbox_opt.take() {
                Some(some) => some,
                None => {
                    let picker_hbox = cascade! {
                        gtk::Box::new(gtk::Orientation::Horizontal, 64);
                    };
                    picker.add(&picker_hbox);
                    picker_hbox
                }
            };

            picker_hbox.add(&group.vbox);

            picker_col += 1;
            if picker_col >= picker_cols {
                picker_col = 0;
            } else {
                picker_hbox_opt = Some(picker_hbox);
            }
        }

        cascade! {
            picker;
            ..set_orientation(gtk::Orientation::Vertical);
            ..set_halign(gtk::Align::Center);
            ..set_spacing(32);
            ..connect_signals();
            ..show_all();
        };
    }
}

impl WidgetImpl for PickerInner {}
impl ContainerImpl for PickerInner {}
impl BoxImpl for PickerInner {}

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

    fn connect_signals(&self) {
        let picker = self;
        for group in self.inner().groups.iter() {
            for key in group.iter_keys() {
                let button = &key.gtk;
                let name = key.name.to_string();
                button.connect_clicked(clone!(@weak picker => @default-panic, move |_| {
                    let kb = match picker.inner().keyboard.borrow().clone() {
                        Some(kb) => kb,
                        None => {
                            return;
                        }
                    };
                    let layer = kb.layer();

                    info!("Clicked {} layer {:?}", name, layer);
                    if let Some(i) = kb.selected() {
                        if let Some(layer) = layer {
                            kb.keymap_set(i, layer, &name);
                        }
                    }
                }));
            }
        }
    }

    fn get_button(&self, scancode_name: &str) -> Option<&gtk::Button> {
        self.inner().keys.get(scancode_name).map(|k| &k.gtk)
    }

    pub(crate) fn set_keyboard(&self, keyboard: Option<Keyboard>) {
        if let Some(old_kb) = &*self.inner().keyboard.borrow() {
            old_kb.set_picker(None);
        }
        if let Some(kb) = &keyboard {
            for group in self.inner().groups.iter() {
                for key in group.iter_keys() {
                    // Check that scancode is available for the keyboard
                    let sensitive = kb.has_scancode(&key.name);
                    key.gtk.set_sensitive(sensitive);
                }
            }
            kb.set_picker(Some(&self));
        }
        *self.inner().keyboard.borrow_mut() = keyboard;
    }

    pub(crate) fn set_selected(&self, scancode_name: Option<String>) {
        let mut selected = self.inner().selected.borrow_mut();

        if let Some(selected) = selected.as_ref() {
            if let Some(button) = self.get_button(selected) {
                button.get_style_context().remove_class("selected");
            }
        }

        *selected = scancode_name;

        if let Some(selected) = selected.as_ref() {
            if let Some(button) = self.get_button(selected) {
                button.get_style_context().add_class("selected");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use daemon::{layouts, Layout};
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
