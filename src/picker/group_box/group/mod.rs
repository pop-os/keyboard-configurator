use super::super::PickerKey;

mod ansi;
pub use ansi::picker_ansi_group;
mod basic_group;
pub use basic_group::PickerBasicGroup;
mod international;
pub use international::PickerInternationalGroup;
mod numpad;
pub use numpad::picker_numpad_group;
mod variable_width;

pub trait PickerGroup {
    fn keys(&self) -> &[PickerKey];
    fn widget(&self) -> &gtk::Widget;
    fn invalidate_filter(&self) {}
}
