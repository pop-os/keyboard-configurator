use super::keyboard::Keyboard;
use cascade::cascade;
use glib::clone;
use glib::subclass;
use glib::subclass::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use glib::translate::{FromGlibPtrFull, ToGlib, ToGlibPtr};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

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

pub struct PickerInner {
    groups: Vec<PickerGroup>,
    keys: HashMap<String, Rc<PickerKey>>,
    keyboard: RefCell<Option<Rc<Keyboard>>>,
}

impl ObjectSubclass for PickerInner {
    const NAME: &'static str = "S76KeyboardPicker";

    type ParentType = gtk::Box;

    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

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
        }
    }
}

impl ObjectImpl for PickerInner {
    glib_object_impl!();

    fn constructed(&self, obj: &glib::Object) {
        self.parent_constructed(obj);

        let picker: &Picker = obj.downcast_ref().unwrap();
        picker.set_orientation(gtk::Orientation::Vertical);
        picker.set_spacing(32);

        let mut picker_hbox_opt: Option<gtk::Box> = None;
        let mut picker_col = 0;
        let picker_cols = DEFAULT_COLS;

        for group in &picker.inner().groups {
            let picker_hbox = match picker_hbox_opt.take() {
                Some(some) => some,
                None => {
                    let picker_hbox = gtk::Box::new(gtk::Orientation::Horizontal, 64);
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
 
glib_wrapper! {
    pub struct Picker(
        Object<subclass::simple::InstanceStruct<PickerInner>,
        subclass::simple::ClassStruct<PickerInner>, PickerClass>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;

    match fn {
        get_type => || PickerInner::get_type().to_glib(),
    }
}

impl Picker {
    pub fn new() -> Self {
        let picker: Self = glib::Object::new(Self::static_type(), &[])
            .unwrap()
            .downcast()
            .unwrap();

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

                    println!("Clicked {} layer {}", name, layer);
                    let selected = *kb.selected.borrow();
                    if let Some(i) = selected {
                        let mut keys = kb.keys.borrow_mut();
                        let k = &mut keys[i];
                        let mut found = false;
                        if let Some(scancode) = kb.keymap.get(name.as_str()) {
                            k.deselect(&picker, layer);
                            k.scancodes[layer] = (*scancode, name.clone());
                            k.refresh(&picker);
                            k.select(&picker, layer);
                            found = true;
                        }
                        if !found {
                            return;
                        }
                        println!(
                            "  set {}, {}, {} to {:04X}",
                            layer, k.electrical.0, k.electrical.1, k.scancodes[layer].0
                        );
                        if let Err(err) = kb.daemon.keymap_set(
                            kb.daemon_board,
                            layer as u8,
                            k.electrical.0,
                            k.electrical.1,
                            k.scancodes[layer].0,
                        ) {
                            eprintln!("Failed to set keymap: {:?}", err);
                        }
                    }
                }));
            }
        }
    }

    pub(crate) fn get_button(&self, scancode_name: &str) -> Option<&gtk::Button> {
        self.inner().keys.get(scancode_name).map(|k| &k.gtk)
    }

    pub(crate) fn get_text(&self, scancode_name: &str) -> Option<&str> {
        self.inner().keys.get(scancode_name).map(|k| k.text.as_ref())
    }

    pub(crate) fn set_keyboard(&self, keyboard: Option<Rc<Keyboard>>) {
        if let Some(old_kb) = &*self.inner().keyboard.borrow() {
            old_kb.set_picker(None);
        }
        if let Some(kb) = &keyboard {
            for group in self.inner().groups.iter() {
                for key in group.iter_keys() {
                    // Check that scancode is available for the keyboard
                    let sensitive = kb.keymap.contains_key(key.name.as_str());
                    key.gtk.set_sensitive(sensitive);
                }
            }
            kb.set_picker(Some(&self));
        }
        *self.inner().keyboard.borrow_mut() = keyboard;
    }
}
