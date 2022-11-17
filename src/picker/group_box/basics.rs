use super::{picker_ansi_group, picker_numpad_group, PickerBasicGroup, PickerGroupBox};

impl PickerGroupBox {
    pub fn basics() -> Self {
        Self::new(vec![
            Box::new(picker_ansi_group()),
            Box::new(PickerBasicGroup::new(
                "Other actions",
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
            // TODO label?
            Box::new(picker_numpad_group()),
            Box::new(PickerBasicGroup::new(
                "Symbols",
                6,
                1.0,
                &["NONUS_HASH", "NONUS_BSLASH"],
            )),
            Box::new(PickerBasicGroup::new(
                "Navigation",
                4,
                1.0,
                &["LEFT", "UP", "DOWN", "RIGHT", "HOME", "PGUP", "PGDN", "END"],
            )),
            Box::new(PickerBasicGroup::new(
                "Media",
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
                "Controls",
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
                "LED controls",
                4,
                1.0,
                &["KBD_TOGGLE", "KBD_UP", "KBD_DOWN", "KBD_BKL", "KBD_COLOR"],
            )),
            Box::new(PickerBasicGroup::new(
                "Layer keys",
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
