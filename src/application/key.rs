use gtk::prelude::*;
use std::{
    collections::HashMap,
};

use super::page::Page;
use super::picker::SCANCODE_LABELS;
use super::rect::Rect;

#[derive(Clone, Debug)]
pub struct Key {
    // Logical position (row, column)
    pub(crate) logical: (u8, u8),
    // Logical name (something like K01, where 0 is the row and 1 is the column)
    pub(crate) logical_name: String,
    // Physical position and size
    pub(crate) physical: Rect,
    // Physical key name (what is printed on the keycap)
    pub(crate) physical_name: String,
    // Electrical mapping (output, input)
    pub(crate) electrical: (u8, u8),
    // Electrical name (output, input)
    pub(crate) electrical_name: String,
    // Currently loaded scancodes and their names
    pub(crate) scancodes: Vec<(u16, String)>,
    // Background color
    pub(crate) background_color: String,
    // Foreground color
    pub(crate) foreground_color: String,
    // GTK buttons by page
    //TODO: clean up this crap
    pub(crate) gtk: HashMap<Page, (gtk::Button, gtk::Label)>,
}

impl Key {
    pub fn css(&self) -> String {
        format!(
r#"
button {{
    background-image: none;
    background-color: {};
    border-image: none;
    box-shadow: none;
    color: {};
    margin: 0;
    padding: 0;
    text-shadow: none;
    -gtk-icon-effect: none;
    -gtk-icon-shadow: none;
}}

.selected {{
    border-color: #fbb86c;
    border-width: 4px;
}}
"#,
            self.background_color,
            self.foreground_color,
        )
    }

    pub fn refresh(&self) {
        for (layer, (_button, label)) in self.gtk.iter() {
            label.set_label(match layer {
                Page::Layer1 => {
                    let scancode_name = &self.scancodes[0].1;
                    SCANCODE_LABELS.get(scancode_name).unwrap_or(scancode_name)
                },
                Page::Layer2 => {
                    let scancode_name = &self.scancodes[1].1;
                    SCANCODE_LABELS.get(scancode_name).unwrap_or(scancode_name)
                },
                Page::Keycaps => &self.physical_name,
                Page::Logical => &self.logical_name,
                Page::Electrical => &self.electrical_name,
            });
        }
    }
}
