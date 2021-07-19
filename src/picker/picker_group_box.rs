use cascade::cascade;
use glib::{clone, subclass::Signal, SignalHandlerId};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use backend::DerefCell;

use super::{picker_group::PickerGroup, picker_json::picker_json, picker_key::PickerKey};

const DEFAULT_COLS: usize = 3;
const HSPACING: i32 = 64;
const VSPACING: i32 = 32;
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

#[derive(Default)]
pub struct PickerGroupBoxInner {
    groups: DerefCell<Vec<PickerGroup>>,
    keys: DerefCell<HashMap<String, Rc<PickerKey>>>,
    selected: RefCell<Vec<String>>,
}

#[glib::object_subclass]
impl ObjectSubclass for PickerGroupBoxInner {
    const NAME: &'static str = "S76KeyboardPickerGroupBox";
    type ParentType = gtk::Container;
    type Type = PickerGroupBox;
}

impl ObjectImpl for PickerGroupBoxInner {
    fn constructed(&self, widget: &PickerGroupBox) {
        self.parent_constructed(widget);

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

        for group in &groups {
            group.vbox.show();
            group.vbox.set_parent(widget);
        }

        self.keys.set(keys);
        self.groups.set(groups);

        cascade! {
            widget;
            ..connect_signals();
            ..show_all();
        };
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![Signal::builder(
                "key-pressed",
                &[String::static_type().into()],
                glib::Type::UNIT.into(),
            )
            .build()]
        });
        SIGNALS.as_ref()
    }
}

impl WidgetImpl for PickerGroupBoxInner {
    fn get_request_mode(&self, _widget: &Self::Type) -> gtk::SizeRequestMode {
        gtk::SizeRequestMode::HeightForWidth
    }

    fn get_preferred_width(&self, _widget: &Self::Type) -> (i32, i32) {
        let minimum_width = self
            .groups
            .iter()
            .map(|x| x.vbox.get_preferred_width().1)
            .max()
            .unwrap();
        let natural_width = self
            .groups
            .chunks(3)
            .map(|row| {
                row.iter()
                    .map(|x| x.vbox.get_preferred_width().1)
                    .sum::<i32>()
            })
            .max()
            .unwrap()
            + 2 * HSPACING;
        (minimum_width, natural_width)
    }

    fn get_preferred_height_for_width(&self, widget: &Self::Type, width: i32) -> (i32, i32) {
        let rows = widget.rows_for_width(width);
        let height = rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|x| x.vbox.get_preferred_height().1)
                    .max()
                    .unwrap()
            })
            .sum::<i32>()
            + (rows.len() as i32 - 1) * VSPACING;

        (height, height)
    }

    fn size_allocate(&self, obj: &Self::Type, allocation: &gtk::Allocation) {
        self.parent_size_allocate(obj, allocation);

        let rows = obj.rows_for_width(allocation.width);

        let total_width = rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|x| x.vbox.get_preferred_width().1)
                    .sum::<i32>()
                    + (row.len() as i32 - 1) * HSPACING
            })
            .max()
            .unwrap();

        let mut y = 0;
        for row in rows {
            let mut x = (allocation.width - total_width) / 2;
            for group in row {
                let height = group.vbox.get_preferred_height().1;
                let width = group.vbox.get_preferred_width().1;
                group.vbox.size_allocate(&gtk::Allocation {
                    x,
                    y,
                    width,
                    height,
                });
                x += width + HSPACING;
            }
            y += row
                .iter()
                .map(|x| x.vbox.get_preferred_height().1)
                .max()
                .unwrap()
                + VSPACING;
        }
    }

    fn realize(&self, widget: &Self::Type) {
        let allocation = widget.get_allocation();
        widget.set_realized(true);

        let attrs = gdk::WindowAttr {
            x: Some(allocation.x),
            y: Some(allocation.y),
            width: allocation.width,
            height: allocation.height,
            window_type: gdk::WindowType::Child,
            event_mask: widget.get_events(),
            wclass: gdk::WindowWindowClass::InputOutput,
            ..Default::default()
        };

        let window = gdk::Window::new(widget.get_parent_window().as_ref(), &attrs);
        widget.register_window(&window);
        widget.set_window(&window);
    }
}

impl ContainerImpl for PickerGroupBoxInner {
    fn forall(
        &self,
        _obj: &Self::Type,
        _include_internals: bool,
        cb: &gtk::subclass::container::Callback,
    ) {
        for group in self.groups.iter() {
            cb.call(group.vbox.upcast_ref());
        }
    }

    fn remove(&self, _obj: &Self::Type, child: &gtk::Widget) {
        child.unparent();
    }
}

glib::wrapper! {
    pub struct PickerGroupBox(ObjectSubclass<PickerGroupBoxInner>)
        @extends gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

impl PickerGroupBox {
    pub fn new() -> Self {
        glib::Object::new(&[]).unwrap()
    }

    fn inner(&self) -> &PickerGroupBoxInner {
        PickerGroupBoxInner::from_instance(self)
    }

    fn connect_signals(&self) {
        let picker = self;
        for group in self.inner().groups.iter() {
            for key in group.iter_keys() {
                let button = &key.gtk;
                let name = key.name.to_string();
                button.connect_clicked(clone!(@weak picker => @default-panic, move |_| {
                    picker.emit_by_name("key-pressed", &[&name]).unwrap();
                }));
            }
        }
    }

    pub fn connect_key_pressed<F: Fn(String) + 'static>(&self, cb: F) -> SignalHandlerId {
        self.connect_local("key-pressed", false, move |values| {
            cb(values[1].get::<String>().unwrap().unwrap());
            None
        })
        .unwrap()
    }

    fn get_button(&self, scancode_name: &str) -> Option<&gtk::Button> {
        self.inner().keys.get(scancode_name).map(|k| &k.gtk)
    }

    pub(crate) fn set_key_visibility<F: Fn(&str) -> (bool, bool)>(&self, f: F) {
        for key in self.inner().keys.values() {
            let (visible, sensitive) = f(&key.name);
            key.gtk.set_visible(visible);
            key.gtk.set_sensitive(sensitive);
        }
    }

    pub(crate) fn set_selected(&self, scancode_names: Vec<String>) {
        let mut selected = self.inner().selected.borrow_mut();

        for i in selected.iter() {
            if let Some(button) = self.get_button(i) {
                button.get_style_context().remove_class("selected");
            }
        }

        *selected = scancode_names;

        for i in selected.iter() {
            if let Some(button) = self.get_button(i) {
                button.get_style_context().add_class("selected");
            }
        }
    }

    fn rows_for_width(&self, container_width: i32) -> Vec<&[PickerGroup]> {
        let mut rows = Vec::new();
        let groups = &*self.inner().groups;

        let mut row_start = 0;
        let mut row_width = 0;
        for (i, group) in groups.iter().enumerate() {
            let width = group.vbox.get_preferred_width().1;

            row_width += width;
            if i != 0 {
                row_width += HSPACING;
            }
            if i - row_start >= DEFAULT_COLS || row_width > container_width {
                rows.push(&groups[row_start..i]);
                row_start = i;
                row_width = width;
            }
        }

        if !groups[row_start..].is_empty() {
            rows.push(&groups[row_start..]);
        }

        rows
    }
}
