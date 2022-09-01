use crate::fl;
use crate::picker::{LAYERS, SCANCODE_LABELS};
use backend::{Key, Keycode, Mods};

static MOD_LABELS: &[(Mods, &str)] = &[
    (Mods::CTRL, "Ctrl"),
    (Mods::SHIFT, "Shift"),
    (Mods::ALT, "Alt"),
    (Mods::SUPER, "Super"),
];

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

    pub fn get_label(&self, key: &Key) -> Vec<String> {
        match self {
            Page::Layer1 | Page::Layer2 | Page::Layer3 | Page::Layer4 => {
                let (scancode, scancode_name) = key.get_scancode(self.layer().unwrap()).unwrap();
                match scancode_name {
                    Some(keycode) => {
                        keycode_label(&keycode).unwrap_or_else(|| vec![format!("{:?}", keycode)])
                    }
                    None => vec![format!("{}", scancode)],
                }
            }
            Page::Keycaps => vec![key.physical_name.clone()],
            Page::Logical => vec![key.logical_name.clone()],
            Page::Electrical => vec![key.electrical_name.clone()],
            Page::Leds => vec![key.led_name.clone()],
        }
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::Layer1
    }
}

// TODO: represent mod-tap/layer-tap by rendering button with a seperator?
fn keycode_label(keycode: &Keycode) -> Option<Vec<String>> {
    match keycode {
        Keycode::Basic(mods, keycode) => {
            if mods.is_empty() {
                Some(vec![SCANCODE_LABELS.get(keycode)?.clone()])
            } else {
                let mut label = mods_label(*mods);
                if keycode != "NONE" {
                    let keycode_label = SCANCODE_LABELS.get(keycode)?;
                    label.push_str(" + ");
                    label.push_str(keycode_label);
                }
                Some(vec![label])
            }
        }
        Keycode::MT(mods, keycode) => {
            let mods_label = mods_label(*mods);
            let keycode_label = SCANCODE_LABELS.get(keycode)?.clone();
            Some(vec![mods_label, keycode_label])
        }
        Keycode::LT(layer, keycode) => {
            let layer_id = *LAYERS.get(usize::from(*layer))?;
            let layer_label = SCANCODE_LABELS.get(layer_id)?.clone();
            let keycode_label = SCANCODE_LABELS.get(keycode)?.clone();
            Some(vec![layer_label, keycode_label])
        }
    }
}

fn mods_label(mods: Mods) -> String {
    if mods.is_empty() {
        return String::new();
    }

    let mut label = if mods.contains(Mods::RIGHT) {
        "Right "
    } else {
        "Left "
    }
    .to_string();
    let mut first = true;
    for (mod_, mod_label) in MOD_LABELS {
        if mods.contains(*mod_) {
            if !first {
                label.push_str(" + ");
            }
            label.push_str(mod_label);
            first = false;
        }
    }
    label
}
