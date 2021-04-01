use std::collections::HashSet;

#[derive(Clone, Default, glib::GBoxed)]
#[gboxed(type_name = "S76SelectedKeys")]
pub struct SelectedKeys(HashSet<usize>);

impl SelectedKeys {
    pub fn new() -> Self {
        Self(HashSet::new())
    }
}

impl std::ops::Deref for SelectedKeys {
    type Target = HashSet<usize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SelectedKeys {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
