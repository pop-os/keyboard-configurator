#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Layer1,
    Layer2,
    Layer3,
    Layer4,
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
            Self::Layer3 => "Layer 3",
            Self::Layer4 => "Layer 4",
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
            Self::Layer3 => Some(2),
            Self::Layer4 => Some(3),
            _ => None,
        }
    }

    pub fn is_debug(&self) -> bool {
        match self {
            Self::Logical | Self::Electrical | Self::Leds => true,
            _ => false,
        }
    }

    pub fn iter_all() -> impl Iterator<Item = Self> {
        vec![
            Self::Layer1,
            Self::Layer2,
            Self::Layer3,
            Self::Layer4,
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
