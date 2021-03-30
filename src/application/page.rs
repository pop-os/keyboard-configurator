use super::{Key, SCANCODE_LABELS};
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

    pub fn get_label(&self, key: &Key) -> String {
        let scancodes = key.scancodes.borrow();
        match self {
            Page::Layer1 | Page::Layer2 | Page::Layer3 | Page::Layer4 => {
                let scancode_name = &scancodes[self.layer().unwrap()].1;
                SCANCODE_LABELS
                    .get(scancode_name)
                    .unwrap_or(scancode_name)
                    .into()
            }
            Page::Keycaps => key.physical_name.clone(),
            Page::Logical => key.logical_name.clone(),
            Page::Electrical => key.electrical_name.clone(),
            Page::Leds => key.led_name.clone(),
        }
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::Layer1
    }
}
