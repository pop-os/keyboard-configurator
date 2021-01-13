use super::keyboard::Keyboard;
use cascade::cascade;
use glib::clone;
use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

mod picker_csv;
mod picker_group;
mod picker_key;

use picker_csv::{picker_csv, PickerCsv};
use picker_group::PickerGroup;
use picker_key::PickerKey;

const DEFAULT_COLS: i32 = 3;
const PICKER_CSS: &'static str = r#"
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
    for record in picker_csv() {
        match record {
            PickerCsv::Group { .. } => {}
            PickerCsv::Key { name, top, bottom } => {
                let text = if bottom.is_empty() {
                    top
                } else {
                    format!("{}\n{}", top, bottom)
                };
                labels.insert(name, text);
            }
        }
    }
    labels
});

pub struct PickerInner {
    groups: Vec<PickerGroup>,
    keys: HashMap<String, Rc<PickerKey>>,
    keyboard: RefCell<Option<Keyboard>>,
    selected: RefCell<Option<String>>,
}

impl ObjectSubclass for PickerInner {
    const NAME: &'static str = "S76KeyboardPicker";

    type ParentType = gtk::Box;
    type Type = Picker;

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::object_subclass!();

    fn new() -> Self {
        let style_provider = cascade! {
            gtk::CssProvider::new();
            ..load_from_data(&PICKER_CSS.as_bytes()).expect("Failed to parse css");
        };

        let mut groups = Vec::new();
        let mut keys = HashMap::new();

        for record in picker_csv() {
            match record {
                PickerCsv::Group { name, cols, width } => {
                    groups.push(PickerGroup::new(name, cols, width));
                }
                PickerCsv::Key { name, top, bottom } => {
                    let text = if bottom.is_empty() {
                        top
                    } else {
                        format!("{}\n{}", top, bottom)
                    };

                    let key = PickerKey::new(
                        name.clone(),
                        text,
                        groups.last().map(|g| g.width).unwrap_or(1),
                        &style_provider,
                    );

                    if let Some(group) = groups.last_mut() {
                        group.add_key(key.clone());
                    }

                    keys.insert(name, key);
                }
            }
        }

        Self {
            groups,
            keys,
            keyboard: RefCell::new(None),
            selected: RefCell::new(None),
        }
    }
}

impl ObjectImpl for PickerInner {
    fn constructed(&self, picker: &Picker) {
        self.parent_constructed(picker);

        picker.set_orientation(gtk::Orientation::Vertical);
        picker.set_spacing(32);

        let mut picker_hbox_opt: Option<gtk::Box> = None;
        let mut picker_col = 0;
        let picker_cols = DEFAULT_COLS;

        for group in &picker.inner().groups {
            let picker_hbox = match picker_hbox_opt.take() {
                Some(some) => some,
                None => {
                    let picker_hbox = cascade! {
                        gtk::Box::new(gtk::Orientation::Horizontal, 64);
                        ..set_halign(gtk::Align::Center);
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
        let picker: Self = glib::Object::new(&[]).unwrap();

        picker.connect_signals();

        picker
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

                    println!("Clicked {} layer {:?}", name, layer);
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
