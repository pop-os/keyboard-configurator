use super::{PickerBasicGroup, PickerGroupBox, PickerInternationalGroup};

impl PickerGroupBox {
    pub fn extras() -> Self {
        Self::new(vec![
            Box::new(PickerBasicGroup::new(
                "Additional Function Keys".to_string(),
                6,
                1.0,
                &[
                    "F13", "F14", "F15", "F16", "F17", "F18", "F19", "F20", "F21", "F22", "F23",
                    "F24",
                ],
            )),
            Box::new(PickerBasicGroup::new(
                "Mouse Actions".to_string(),
                5,
                2.0,
                &[
                    "MS_UP",
                    "MS_DOWN",
                    "MS_LEFT",
                    "MS_RIGHT",
                    "MS_BTN1",
                    "MS_BTN2",
                    "MS_BTN3",
                    "MS_BTN4",
                    "MS_BTN5",
                    "MS_BTN6",
                    "MS_BTN7",
                    "MS_BTN8",
                    "MS_WH_UP",
                    "MS_WH_DOWN",
                    "MS_WH_LEFT",
                    "MS_WH_RIGHT",
                    "MS_ACCEL0",
                    "MS_ACCEL1",
                    "MS_ACCEL2",
                ],
            )),
            Box::new(PickerInternationalGroup::new()),
        ])
    }
}
