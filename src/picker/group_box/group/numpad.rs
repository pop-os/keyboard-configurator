use super::variable_width::{PickerVariableWidthGroup, KEY_SIZE, KEY_SPACE};

static KEY_WIDTHS: &[(f64, &[&str])] = &[(2.0 * KEY_SIZE + KEY_SPACE, &["NUM_0"])];

static KEY_HEIGHTS: &[(f64, &[&str])] = &[(2.0 * KEY_SIZE + KEY_SPACE, &["NUM_PLUS", "NUM_ENTER"])];

static ROWS: &[&[&str]] = &[
    &["NUM_LOCK", "NUM_SLASH", "NUM_ASTERISK", "NUM_MINUS"],
    &["NUM_7", "NUM_8", "NUM_9", "NUM_PLUS"],
    &["NUM_4", "NUM_5", "NUM_6"],
    &["NUM_1", "NUM_2", "NUM_3", "NUM_ENTER"],
    &["NUM_0", "NUM_PERIOD"],
];

pub fn picker_numpad_group() -> PickerVariableWidthGroup {
    PickerVariableWidthGroup::new(ROWS, KEY_WIDTHS, KEY_HEIGHTS, Some("Numpad"), None)
}
