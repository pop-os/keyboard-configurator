use gtk::{
    gdk,
    glib::{self, clone, subclass::Signal, SignalHandlerId},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;
use std::{cell::RefCell, collections::HashMap};

use backend::{DerefCell, Keycode};

use super::picker_key::PickerKey;

mod basics;
mod extras;
mod group;
pub use group::*;

const DEFAULT_COLS: usize = 3;
const HSPACING: i32 = 64;
const VSPACING: i32 = 32;

#[derive(Default)]
pub struct PickerGroupBoxInner {
    groups: DerefCell<Vec<Box<dyn PickerGroup>>>,
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
        let width = self
            .groups
            .iter()
            .map(|x| x.widget().preferred_width().1)
            .max()
            .unwrap_or(0);
        (width, width)
    }

    fn preferred_height_for_width(&self, widget: &Self::Type, width: i32) -> (i32, i32) {
        let rows = widget.rows_for_width(width);
        let height = total_height_for_rows(&rows);
        (height, height)
    }

    fn size_allocate(&self, obj: &Self::Type, allocation: &gtk::Allocation) {
        self.parent_size_allocate(obj, allocation);

        let rows = obj.rows_for_width(allocation.width());

        let mut y = 0;
        for row in rows {
            let mut x = 0;
            for group in row {
                let height = group.widget().preferred_height().1;
                let width = group.widget().preferred_width().1;
                group
                    .widget()
                    .size_allocate(&gtk::Allocation::new(x, y, width, height));
                x += width + HSPACING;
            }
            y += row
                .iter()
                .map(|x| x.widget().preferred_height().1)
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
            cb.call(group.widget().upcast_ref());
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
    pub fn new(groups: Vec<Box<dyn PickerGroup>>) -> Self {
        let widget: Self = glib::Object::new(&[]).unwrap();

        let mut keys = HashMap::new();

        for group in &groups {
            group.widget().show();
            group.widget().set_parent(&widget);
            for key in group.keys() {
                keys.insert(key.name().to_string(), key.clone());
            }
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
    pub fn set_key_visibility<F: Fn(&str) -> bool>(&self, f: F) {
        for group in self.inner().groups.iter() {
            let group_visible = group.keys().iter().fold(false, |group_visible, key| {
                key.set_visible(f(&key.name()));
                group_visible || key.get_visible()
            });

            group.widget().set_visible(group_visible);
            group.invalidate_filter();
        }
    }

    pub fn set_key_sensitivity<F: Fn(&str) -> bool>(&self, f: F) {
        for key in self.inner().keys.values() {
            key.set_sensitive(f(&key.name()));
        }
    }

    pub fn set_selected(&self, scancode_names: Vec<Keycode>) {
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

    fn rows_for_width(&self, container_width: i32) -> Vec<&[Box<dyn PickerGroup>]> {
        let mut rows = Vec::new();
        let groups = &*self.inner().groups;

        let mut row_start = 0;
        let mut row_width = 0;
        for (i, group) in groups.iter().enumerate() {
            let width = group.widget().preferred_width().1;

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

fn max_width_for_rows(rows: &[&[Box<dyn PickerGroup>]]) -> i32 {
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|x| x.widget().preferred_width().1)
                .sum::<i32>()
                + (row.len() as i32 - 1) * HSPACING
        })
        .max()
        .unwrap_or(0)
}

fn total_height_for_rows(rows: &[&[Box<dyn PickerGroup>]]) -> i32 {
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|x| x.widget().preferred_height().1)
                .max()
                .unwrap_or(0)
        })
        .sum::<i32>()
        + (rows.len() as i32 - 1) * VSPACING
}
