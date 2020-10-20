use cascade::cascade;
use gtk::prelude::*;
use std::rc::Rc;

use super::picker_key::PickerKey;

pub(super) struct PickerGroup {
    /// Name of the group
    pub(super) name: String,
    /// Number of keys to show in each row
    pub(super) cols: i32,
    /// Width of each key in this group
    pub(super) width: i32,
    /// Name of keys in this group
    keys: Vec<Rc<PickerKey>>,
    pub(super) vbox: gtk::Box,
    hbox_opt: Option<gtk::Box>,
    col: i32,
}

impl PickerGroup {
    pub(super) fn new(name: String, cols: i32, width: i32) -> Self {
        let label = cascade! {
            gtk::Label::new(Some(&name));
            ..set_halign(gtk::Align::Start);
            ..set_margin_bottom(8);
        };

        let vbox = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 4);
            ..add(&label);
        };

        Self {
            name,
            cols,
            width,
            keys: Vec::new(),
            vbox,
            hbox_opt: None,
            col: 0,
        }
    }

    pub(super) fn add_key(&mut self, key: Rc<PickerKey>) {
        let hbox = match self.hbox_opt.take() {
            Some(some) => some,
            None => {
                let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                self.vbox.add(&hbox);
                hbox
            }
        };

        hbox.add(&key.gtk);

        self.col += 1;
        if self.col >= self.cols {
            self.col = 0;
        } else {
            self.hbox_opt = Some(hbox);
        }

        self.keys.push(key);
    }

    pub(super) fn iter_keys(&self) -> impl Iterator<Item = &PickerKey> {
        self.keys.iter().map(|k| k.as_ref())
    }
}
