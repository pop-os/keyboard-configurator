use gtk::glib;
use std::collections::BTreeSet;

#[derive(Clone, Default, glib::Boxed)]
#[boxed_type(name = "S76SelectedKeys")]
pub struct SelectedKeys(BTreeSet<usize>);

impl SelectedKeys {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }
}

impl std::ops::Deref for SelectedKeys {
    type Target = BTreeSet<usize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SelectedKeys {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
