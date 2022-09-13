// International section is displayed in non-standard way: two colums,
// with descriptions.

use cascade::cascade;
use gtk::prelude::*;

use super::{PickerGroup, PickerKey};

static INT_KEYS: &[(&str, &str)] = &[
    ("INT1", "JIS \\ and _"),
    ("INT2", "JIS Katakana/Hiragana"),
    ("INT3", "JIS JIS ¥ and |"),
    ("INT4", "JIS Henkan"),
    ("INT5", "JIS Muhenkan"),
    ("INT6", "JIS Numpad ,"),
    ("INT7", "International 7"),
    ("INT8", "International 8"),
    ("INT9", "International 9"),
];
static LANG_KEYS: &[(&str, &str)] = &[
    ("LANG1", "Hangul/English"),
    ("LANG2", "Hanja"),
    ("LANG3", "JIS Katakana"),
    ("LANG4", "JIS Hiragana"),
    ("LANG5", "JIS Zenkaku/Hankaku"),
    ("LANG6", "Language 6"),
    ("LANG7", "Language 7"),
    ("LANG8", "Language 8"),
    ("LANG9", "Language 9"),
];

pub struct PickerInternationalGroup {
    keys: Vec<PickerKey>,
    widget: gtk::Box,
}

fn row(keys: &mut Vec<PickerKey>, keycode: &str, description: &str) -> gtk::Box {
    let key = PickerKey::new(keycode, 1);
    keys.push(key.clone());
    cascade! {
        gtk::Box::new(gtk::Orientation::Horizontal, 0);
        ..add(&key);
        ..add(&gtk::Label::new(Some(description)));
    }
}

// Consider how this scales
impl PickerInternationalGroup {
    pub fn new() -> Self {
        let mut keys = Vec::new();

        let int_box = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 0);
        };
        for (keycode, description) in INT_KEYS {
            int_box.add(&row(&mut keys, keycode, description));
        }

        let lang_box = cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 0);
        };
        for (keycode, description) in LANG_KEYS {
            lang_box.add(&row(&mut keys, keycode, description));
        }

        let widget = cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 0);
            ..add(&int_box);
            ..add(&lang_box);
        };

        Self { keys, widget }
    }
}

impl PickerGroup for PickerInternationalGroup {
    fn keys(&self) -> &[PickerKey] {
        &self.keys
    }

    fn widget(&self) -> &gtk::Widget {
        self.widget.upcast_ref()
    }
}
