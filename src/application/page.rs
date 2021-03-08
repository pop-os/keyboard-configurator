#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Layer1,
    Layer2,
    Keycaps,
    Logical,
    Electrical,
    Leds,
}

impl Page {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Layer1 => "Layer 1",
            Self::Layer2 => "Layer 2",
            Self::Keycaps => "Keycaps",
            Self::Logical => "Logical",
            Self::Electrical => "Electrical",
            Self::Leds => "LEDs",
        }
    }

    pub fn layer(&self) -> Option<usize> {
        match self {
            Self::Layer1 => Some(0),
            Self::Layer2 => Some(1),
            _ => None,
        }
    }

    pub fn iter_all() -> impl Iterator<Item = Self> {
        vec![
            Self::Layer1,
            Self::Layer2,
            Self::Keycaps,
            Self::Logical,
            Self::Electrical,
            Self::Leds,
        ]
        .into_iter()
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::Layer1
    }
}
