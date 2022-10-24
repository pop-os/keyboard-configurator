use gtk::prelude::*;

use super::{PickerGroup, PickerKey};

const KEY_WIDTH: f64 = 48.0;
const KEY_SPACE: f64 = 4.0;

// A 2U key takes same space as 2 1U including spacing
// 2 1.5U keys take same space as 3 1U
// Space bar is the same as 3 1U + 1 1.5U to line up with previous row
static KEY_WIDTHS: &[(f64, &[&str])] = &[
    (
        1.5 * KEY_WIDTH + 0.5 * KEY_SPACE,
        &[
            "DEL",
            "BKSP",
            "TAB",
            "CAPS",
            "LEFT_CTRL",
            "LEFT_ALT",
            "LEFT_SUPER",
            "RIGHT_SUPER",
            "RIGHT_CTRL",
        ],
    ),
    (
        2.0 * KEY_WIDTH + KEY_SPACE,
        &["LEFT_SHIFT", "RIGHT_SHIFT", "ENTER"],
    ),
    (4.5 * KEY_WIDTH + 3.5 * KEY_SPACE, &["SPACE"]),
];

static ROWS: &[&[&str]] = &[
    &[
        "ESC", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "DEL",
    ],
    &[
        "TICK", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "MINUS", "EQUALS", "BKSP",
    ],
    &[
        "TAB",
        "Q",
        "W",
        "E",
        "R",
        "T",
        "Y",
        "U",
        "I",
        "O",
        "P",
        "BRACE_OPEN",
        "BRACE_CLOSE",
        "BACKSLASH",
    ],
    &[
        "CAPS",
        "A",
        "S",
        "D",
        "F",
        "G",
        "H",
        "J",
        "K",
        "L",
        "SEMICOLON",
        "QUOTE",
        "ENTER",
    ],
    &[
        "LEFT_SHIFT",
        "Z",
        "X",
        "C",
        "V",
        "B",
        "N",
        "M",
        "COMMA",
        "PERIOD",
        "SLASH",
        "RIGHT_SHIFT",
    ],
    &[
        "LEFT_CTRL",
        "LEFT_ALT",
        "LEFT_SUPER",
        "SPACE",
        "RIGHT_SUPER",
        "RIGHT_ALT",
        "APP",
        "RIGHT_CTRL",
    ],
];

pub struct PickerAnsiGroup {
    keys: Vec<PickerKey>,
    widget: gtk::Fixed,
}

impl PickerAnsiGroup {
    pub fn new() -> Self {
        let mut keys = Vec::new();
        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let fixed = gtk::Fixed::new();

        let mut y = 0;
        for row in ROWS {
            let mut x = 0;
            for name in *row {
                let width = KEY_WIDTHS
                    .iter()
                    .find_map(|(width, keys)| {
                        if keys.contains(name) {
                            Some(*width)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(KEY_WIDTH);
                let key = PickerKey::new(name, width / KEY_WIDTH);
                fixed.put(&key, x, y);
                keys.push(key);
                x += width as i32 + 4
            }
            y += KEY_WIDTH as i32 + 4;
        }

        PickerAnsiGroup {
            keys,
            widget: fixed,
        }
    }
}

impl PickerGroup for PickerAnsiGroup {
    fn keys(&self) -> &[PickerKey] {
        &self.keys
    }

    fn widget(&self) -> &gtk::Widget {
        self.widget.upcast_ref()
    }
}
