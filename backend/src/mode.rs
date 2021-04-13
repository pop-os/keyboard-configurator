use once_cell::sync::Lazy;
use std::collections::HashMap;

/// A mode/pattern for the keyboard's LEDs to display
#[non_exhaustive]
pub struct Mode {
    /// Index (as used in firmware)
    pub index: u8,
    /// Textual ID of mode, for `ListBox` or debugging
    pub id: &'static str,
    /// Display name of mode
    pub name: &'static str,
    /// Hue setting has effect in this mode
    pub has_hue: bool,
    /// Speed setting has effect in this mode
    pub has_speed: bool,
}

impl Mode {
    const fn new(
        index: u8,
        id: &'static str,
        name: &'static str,
        has_hue: bool,
        has_speed: bool,
    ) -> Self {
        Self {
            index,
            id,
            name,
            has_hue,
            has_speed,
        }
    }

    /// Return slice of all `Mode`s, ordered as they should be displayed
    pub fn all() -> &'static [Mode] {
        MODES
    }

    /// Get `Mode` corresponding to index (as used in firmware)
    pub fn from_index(index: u8) -> Option<&'static Self> {
        static MODE_BY_INDEX: Lazy<HashMap<u8, &Mode>> =
            Lazy::new(|| MODES.iter().map(|i| (i.index, i)).collect());
        MODE_BY_INDEX.get(&index).cloned()
    }

    /// Get `Mode` corresponding to textual ID
    pub fn from_id(id: &str) -> Option<&'static Self> {
        static MODE_BY_ID: Lazy<HashMap<&str, &Mode>> =
            Lazy::new(|| MODES.iter().map(|i| (i.id, i)).collect());
        MODE_BY_ID.get(&id).cloned()
    }

    /// `true` for Per Key mode, otherwise `false`
    pub fn is_per_key(&self) -> bool {
        self.index == 1
    }
}

static MODES: &[Mode] = &[
    Mode::new(0, "SOLID_COLOR", "Solid Color", true, false),
    Mode::new(1, "PER_KEY", "Per Key", true, false),
    Mode::new(2, "CYCLE_ALL", "Cosmic Background", false, true),
    Mode::new(3, "CYCLE_LEFT_RIGHT", "Horizonal Scan", false, true),
    Mode::new(4, "CYCLE_UP_DOWN", "Vertical Scan", false, true),
    Mode::new(5, "CYCLE_OUT_IN", "Event Horizon", false, true),
    Mode::new(6, "CYCLE_OUT_IN_DUAL", "Binary Galaxies", false, true),
    Mode::new(7, "RAINBOW_MOVING_CHEVRON", "Spacetime", false, true),
    Mode::new(8, "CYCLE_PINWHEEL", "Pinwheel Galaxy", false, true),
    Mode::new(9, "CYCLE_SPIRAL", "Spiral Galaxy", false, true),
    Mode::new(10, "RAINDROPS", "Elements", false, false),
    Mode::new(11, "SPLASH", "Splashdown", false, true),
    Mode::new(12, "MULTISPLASH", "Meteor Shower", false, true),
    Mode::new(13, "ACTIVE_KEYS", "Active Keys", true, false),
];
