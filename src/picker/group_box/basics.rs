use super::{PickerAnsiGroup, PickerBasicGroup, PickerGroupBox};

impl PickerGroupBox {
    pub fn basics() -> Self {
        Self::new(vec![
            Box::new(PickerAnsiGroup::new()),
            Box::new(PickerBasicGroup::new(
                "Other Actions".to_string(),
                4,
                1.5,
                &[
                    "INSERT",
                    "PRINT_SCREEN",
                    "SCROLL_LOCK",
                    "PAUSE",
                    "RESET",
                    "ROLL_OVER",
                    "NONE",
                ],
            )),
            // TODO numpad
            Box::new(PickerBasicGroup::new(
                "Numpad".to_string(),
                6,
                1.0,
                &[
                    "NUM_LOCK",
                    "NUM_7",
                    "NUM_8",
                    "NUM_9",
                    "NUM_MINUS",
                    "NUM_PLUS",
                    "NUM_SLASH",
                    "NUM_4",
                    "NUM_5",
                    "NUM_6",
                    "NUM_ASTERISK",
                    "NUM_ENTER",
                    "NUM_0",
                    "NUM_1",
                    "NUM_2",
                    "NUM_3",
                    "NUM_PERIOD",
                ],
            )),
            Box::new(PickerBasicGroup::new(
                "Symbols".to_string(),
                6,
                1.0,
                &["NONUS_HASH", "NONUS_BSLASH"],
            )),
            Box::new(PickerBasicGroup::new(
                "Navigation".to_string(),
                4,
                1.0,
                &["LEFT", "UP", "DOWN", "RIGHT", "HOME", "PGUP", "PGDN", "END"],
            )),
            Box::new(PickerBasicGroup::new(
                "Media".to_string(),
                3,
                1.0,
                &[
                    "MUTE",
                    "VOLUME_UP",
                    "VOLUME_DOWN",
                    "PLAY_PAUSE",
                    "MEDIA_NEXT",
                    "MEDIA_PREV",
                ],
            )),
            Box::new(PickerBasicGroup::new(
                "Controls".to_string(),
                4,
                2.0,
                &[
                    "FAN_TOGGLE",
                    "DISPLAY_TOGGLE",
                    "BRIGHTNESS_UP",
                    "BRIGHTNESS_DOWN",
                    "DISPLAY_MODE",
                    "SUSPEND",
                    "CAMERA_TOGGLE",
                    "AIRPLANE_MODE",
                    "TOUCHPAD",
                    "SYSTEM_POWER",
                ],
            )),
            Box::new(PickerBasicGroup::new(
                "LED controls".to_string(),
                4,
                1.0,
                &["KBD_TOGGLE", "KBD_UP", "KBD_DOWN", "KBD_BKL", "KBD_COLOR"],
            )),
            Box::new(PickerBasicGroup::new(
                "Layer keys".to_string(),
                4,
                2.0,
                &[
                    "LAYER_ACCESS_1",
                    "FN",
                    "LAYER_ACCESS_3",
                    "LAYER_ACCESS_4",
                    "LAYER_SWITCH_1",
                    "LAYER_SWITCH_2",
                    "LAYER_SWITCH_3",
                    "LAYER_SWITCH_4",
                ],
            )),
        ])
    }
}
