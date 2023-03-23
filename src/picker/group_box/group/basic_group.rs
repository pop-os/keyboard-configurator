use cascade::cascade;
use gtk::{pango, prelude::*};

use super::{PickerGroup, PickerKey};

trait Group {
    fn keys(&self) -> &[PickerKey];
    fn widget(&self) -> &gtk::Widget;
}

pub struct PickerBasicGroup {
    keys: Vec<PickerKey>,
    vbox: gtk::Box,
    flow_box: gtk::FlowBox,
}

impl PickerBasicGroup {
    pub fn new(name: &str, cols: u32, width: f64, key_names: &[&str]) -> Self {
        let label = cascade! {
            gtk::Label::new(Some(name));
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

        let keys: Vec<_> = key_names
            .iter()
            .map(|name| PickerKey::new(name, width, 1.0))
            .collect();
        for key in &keys {
            flow_box.add(key);
        }

        Self {
            keys,
            vbox,
            flow_box,
        }
    }
}

impl PickerGroup for PickerBasicGroup {
    fn keys(&self) -> &[PickerKey] {
        &self.keys
    }

    fn widget(&self) -> &gtk::Widget {
        self.vbox.upcast_ref()
    }

    fn invalidate_filter(&self) {
        self.flow_box.invalidate_filter();
    }
}
