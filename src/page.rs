use crate::fl;
use crate::picker::SCANCODE_LABELS;
use backend::Key;

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
    pub fn name(&self) -> String {
        match self {
            Self::Layer1 => fl!("page-layer1"),
            Self::Layer2 => fl!("page-layer2"),
            Self::Layer3 => fl!("page-layer3"),
            Self::Layer4 => fl!("page-layer4"),
            Self::Keycaps => fl!("page-keycaps"),
            Self::Logical => fl!("page-logical"),
            Self::Electrical => fl!("page-electrical"),
            Self::Leds => fl!("page-leds"),
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
        matches!(
            self,
            Self::Keycaps | Self::Logical | Self::Electrical | Self::Leds
        )
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
        match self {
            Page::Layer1 | Page::Layer2 | Page::Layer3 | Page::Layer4 => {
                let scancode_name = key.get_scancode(self.layer().unwrap()).unwrap().1;
                SCANCODE_LABELS
                    .get(&scancode_name)
                    .unwrap_or(&scancode_name)
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
