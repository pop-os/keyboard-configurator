#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Layer1,
    Layer2,
    Keycaps,
    Logical,
    Electrical,
}

impl Page {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Layer1 => "Layer 1",
            Self::Layer2 => "Layer 2",
            Self::Keycaps => "Keycaps",
            Self::Logical => "Logical",
            Self::Electrical => "Electrical",
        }
    }

    pub fn iter_all() -> impl Iterator<Item = Self> {
        vec![
            Self::Layer1,
            Self::Layer2,
            Self::Keycaps,
            Self::Logical,
            Self::Electrical,
        ]
        .into_iter()
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::Layer1
    }
}
