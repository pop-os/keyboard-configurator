use cascade::cascade;
use gtk::{pango, prelude::*};

use super::PickerKey;

pub(super) struct PickerGroup {
    /// Name of keys in this group
    keys: Vec<PickerKey>,
    pub vbox: gtk::Box,
    flow_box: gtk::FlowBox,
}

impl PickerGroup {
    pub fn new(name: String, cols: u32) -> Self {
        let label = cascade! {
            gtk::Label::new(Some(&name));
            ..set_attributes(Some(&cascade! {
                pango::AttrList::new();
                ..insert(pango::AttrInt::new_weight(pango::Weight::Bold));
            } ));
            ..set_halign(gtk::Align::Start);
            ..set_margin_bottom(8);
            ..show();
        };

        let flow_box = cascade! {
            gtk::FlowBox::new();
            ..set_column_spacing(4);
            ..set_row_spacing(4);
            ..set_max_children_per_line(cols);
            ..set_min_children_per_line(cols);
            ..set_filter_func(Some(Box::new(|child: &gtk::FlowBoxChild| child.child().unwrap().get_visible())));
            ..show();
        };

        let vbox = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 4);
            ..set_no_show_all(true);
            ..add(&label);
            ..add(&flow_box);
        };

        Self {
            keys: Vec::new(),
            vbox,
            flow_box,
        }
    }

    pub fn add_key(&mut self, key: PickerKey) {
        self.flow_box.add(&key);
        self.keys.push(key);
    }

    pub fn keys(&self) -> impl Iterator<Item = &PickerKey> {
        self.keys.iter().map(|k| k.as_ref())
    }

    pub fn invalidate_filter(&self) {
        self.flow_box.invalidate_filter();
    }
}
