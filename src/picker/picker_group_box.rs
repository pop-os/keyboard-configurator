use cascade::cascade;
use gtk::{
    gdk,
    glib::{self, clone, subclass::Signal, SignalHandlerId},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;
use std::{cell::RefCell, collections::HashMap};

use backend::{DerefCell, Keycode};

use super::{picker_group::PickerGroup, picker_json::picker_json, picker_key::PickerKey};

const DEFAULT_COLS: usize = 3;
const HSPACING: i32 = 64;
const VSPACING: i32 = 32;

#[derive(Default)]
pub struct PickerGroupBoxInner {
    groups: DerefCell<Vec<PickerGroup>>,
    keys: DerefCell<HashMap<String, PickerKey>>,
    selected: RefCell<Vec<Keycode>>,
}

#[glib::object_subclass]
impl ObjectSubclass for PickerGroupBoxInner {
    const NAME: &'static str = "S76KeyboardPickerGroupBox";
    type ParentType = gtk::Container;
    type Type = PickerGroupBox;
}

impl ObjectImpl for PickerGroupBoxInner {
    fn signals() -> &'static [Signal] {
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![Signal::builder(
                "key-pressed",
                &[String::static_type().into(), bool::static_type().into()],
                glib::Type::UNIT.into(),
            )
            .build()]
        });
        SIGNALS.as_ref()
    }
}

impl WidgetImpl for PickerGroupBoxInner {
    fn request_mode(&self, _widget: &Self::Type) -> gtk::SizeRequestMode {
        gtk::SizeRequestMode::HeightForWidth
    }

    fn preferred_width(&self, _widget: &Self::Type) -> (i32, i32) {
        let minimum_width = self
            .groups
            .iter()
            .map(|x| x.vbox.preferred_width().1)
            .max()
            .unwrap_or(0);
        let natural_width = self
            .groups
            .chunks(3)
            .map(|row| row.iter().map(|x| x.vbox.preferred_width().1).sum::<i32>())
            .max()
            .unwrap_or(0)
            + 2 * HSPACING;
        (minimum_width, natural_width)
    }

    fn preferred_height_for_width(&self, widget: &Self::Type, width: i32) -> (i32, i32) {
        let rows = widget.rows_for_width(width);
        let height = rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|x| x.vbox.preferred_height().1)
                    .max()
                    .unwrap_or(0)
            })
            .sum::<i32>()
            + (rows.len() as i32 - 1) * VSPACING;

        (height, height)
    }

    fn size_allocate(&self, obj: &Self::Type, allocation: &gtk::Allocation) {
        self.parent_size_allocate(obj, allocation);

        let rows = obj.rows_for_width(allocation.width());

        let total_width = rows
            .iter()
            .map(|row| {
                row.iter().map(|x| x.vbox.preferred_width().1).sum::<i32>()
                    + (row.len() as i32 - 1) * HSPACING
            })
            .max()
            .unwrap_or(0);

        let mut y = 0;
        for row in rows {
            let mut x = (allocation.width() - total_width) / 2;
            for group in row {
                let height = group.vbox.preferred_height().1;
                let width = group.vbox.preferred_width().1;
                group
                    .vbox
                    .size_allocate(&gtk::Allocation::new(x, y, width, height));
                x += width + HSPACING;
            }
            y += row
                .iter()
                .map(|x| x.vbox.preferred_height().1)
                .max()
                .unwrap()
                + VSPACING;
        }
    }

    fn realize(&self, widget: &Self::Type) {
        let allocation = widget.allocation();
        widget.set_realized(true);

        let attrs = gdk::WindowAttr {
            x: Some(allocation.x()),
            y: Some(allocation.y()),
            width: allocation.width(),
            height: allocation.height(),
            window_type: gdk::WindowType::Child,
            event_mask: widget.events(),
            wclass: gdk::WindowWindowClass::InputOutput,
            ..Default::default()
        };

        let window = gdk::Window::new(widget.parent_window().as_ref(), &attrs);
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
    pub fn new(section: &str) -> Self {
        let widget: Self = glib::Object::new(&[]).unwrap();

        let mut groups = Vec::new();
        let mut keys = HashMap::new();

        for json_group in picker_json() {
            if json_group.section != section {
                continue;
            }

            let mut group = PickerGroup::new(json_group.label, json_group.cols);

            for json_key in json_group.keys {
                let key = PickerKey::new(&json_key.keysym, &json_key.label, json_group.width);

                group.add_key(key.clone());
                keys.insert(json_key.keysym, key);
            }

            groups.push(group);
        }

        for group in &groups {
            group.vbox.show();
            group.vbox.set_parent(&widget);
        }

        widget.inner().keys.set(keys);
        widget.inner().groups.set(groups);
        widget.connect_signals();

        widget
    }

    fn inner(&self) -> &PickerGroupBoxInner {
        PickerGroupBoxInner::from_instance(self)
    }

    fn connect_signals(&self) {
        let picker = self;
        for group in self.inner().groups.iter() {
            for key in group.keys() {
                let button = &key;
                let name = key.name().to_string();
                button.connect_clicked_with_shift(
                    clone!(@weak picker => @default-panic, move |_, shift| {
                        picker.emit_by_name::<()>("key-pressed", &[&name, &shift]);
                    }),
                );
            }
        }
    }

    pub fn connect_key_pressed<F: Fn(String, bool) + 'static>(&self, cb: F) -> SignalHandlerId {
        self.connect_local("key-pressed", false, move |values| {
            cb(
                values[1].get::<String>().unwrap(),
                values[2].get::<bool>().unwrap(),
            );
            None
        })
    }

    // XXX need to enable/disable features; show/hide just plain keycodes
    pub(crate) fn set_key_visibility<F: Fn(&str) -> bool>(&self, f: F) {
        for group in self.inner().groups.iter() {
            let group_visible = group.keys().fold(false, |group_visible, key| {
                key.set_visible(f(&key.name()));
                group_visible || key.get_visible()
            });

            group.vbox.set_visible(group_visible);
            group.invalidate_filter();
        }
    }

    pub(crate) fn set_key_sensitivity<F: Fn(&str) -> bool>(&self, f: F) {
        for key in self.inner().keys.values() {
            key.set_sensitive(f(&key.name()));
        }
    }

    pub(crate) fn set_selected(&self, scancode_names: Vec<Keycode>) {
        for button in self.inner().keys.values() {
            button.set_selected(false);
        }

        for i in scancode_names.iter() {
            match i {
                Keycode::Basic(mods, scancode_name) => {
                    if let Some(button) = self.inner().keys.get(scancode_name) {
                        if !(scancode_name == "NONE" && !mods.is_empty()) {
                            button.set_selected(true);
                        }
                    }
                    for scancode_name in mods.mod_names() {
                        if let Some(button) = self.inner().keys.get(scancode_name) {
                            button.set_selected(true);
                        }
                    }
                }
                Keycode::MT(..) | Keycode::LT(..) => {}
            }
        }

        *self.inner().selected.borrow_mut() = scancode_names;
    }

    fn rows_for_width(&self, container_width: i32) -> Vec<&[PickerGroup]> {
        let mut rows = Vec::new();
        let groups = &*self.inner().groups;

        let mut row_start = 0;
        let mut row_width = 0;
        for (i, group) in groups.iter().enumerate() {
            let width = group.vbox.preferred_width().1;

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
