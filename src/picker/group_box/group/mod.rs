use super::super::PickerKey;

mod basic_group;
pub use basic_group::PickerBasicGroup;
mod international;
pub use international::PickerInternationalGroup;

pub trait PickerGroup {
    fn keys(&self) -> &[PickerKey];
    fn widget(&self) -> &gtk::Widget;
    fn invalidate_filter(&self) {}
}
