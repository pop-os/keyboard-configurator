use cascade::cascade;
use gtk::{pango, prelude::*};

use super::{PickerGroup, PickerKey};

pub const KEY_SIZE: f64 = 48.0;
pub const KEY_SPACE: f64 = 4.0;

pub struct PickerVariableWidthGroup {
    keys: Vec<PickerKey>,
    widget: gtk::Box,
}

impl PickerVariableWidthGroup {
    pub fn new(
        rows: &[&[&str]],
        widths: &[(f64, &[&str])],
        heights: &[(f64, &[&str])],
        label: Option<&str>,
        desc: Option<&str>,
    ) -> Self {
        let mut keys = Vec::new();

        let vbox = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 4);
            ..show();
        };

        if let Some(label) = label {
            let label = cascade! {
                gtk::Label::new(Some(&label));
                ..set_attributes(Some(&cascade! {
                    pango::AttrList::new();
                    ..insert(pango::AttrInt::new_weight(pango::Weight::Bold));
                } ));
                ..set_halign(gtk::Align::Start);
                ..set_margin_bottom(8);
                ..show();
            };
            vbox.add(&label);
        }

        let fixed = gtk::Fixed::new();
        vbox.add(&fixed);

        let mut y = 0;
        for row in rows {
            let mut x = 0;
            for name in *row {
                let width = widths
                    .iter()
                    .find_map(|(width, keys)| {
                        if keys.contains(name) {
                            Some(*width)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(KEY_SIZE);
                let height = heights
                    .iter()
                    .find_map(|(height, keys)| {
                        if keys.contains(name) {
                            Some(*height)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(KEY_SIZE);
                let key = PickerKey::new(name, width / KEY_SIZE, height / KEY_SIZE);
                fixed.put(&key, x, y);
                keys.push(key);
                x += width as i32 + 4
            }
            y += KEY_SIZE as i32 + 4;
        }

        if let Some(desc) = desc {
            let label = cascade! {
                gtk::Label::new(Some(&desc));
                ..set_halign(gtk::Align::Start);
                ..show();
            };
            vbox.add(&label);
        }

        Self { keys, widget: vbox }
    }
}

impl PickerGroup for PickerVariableWidthGroup {
    fn keys(&self) -> &[PickerKey] {
        &self.keys
    }

    fn widget(&self) -> &gtk::Widget {
        self.widget.upcast_ref()
    }
}
