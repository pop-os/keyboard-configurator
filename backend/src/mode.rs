use crate::fl;
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
    pub name: String,
    /// Hue setting has effect in this mode
    pub has_hue: bool,
    /// Speed setting has effect in this mode
    pub has_speed: bool,
}

impl Mode {
    const fn new(
        index: u8,
        id: &'static str,
        name: String,
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
        &MODES
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

    pub fn is_disabled(&self) -> bool {
        self.index == 14
    }
}

static MODES: Lazy<Vec<Mode>> = Lazy::new(|| {
    vec![
        Mode::new(14, "DISABLED", fl!("mode-disabled"), false, false),
        Mode::new(0, "SOLID_COLOR", fl!("mode-solid-color"), true, false),
        Mode::new(1, "PER_KEY", fl!("mode-per-key"), true, false),
        Mode::new(13, "ACTIVE_KEYS", fl!("mode-active-keys"), true, false),
        Mode::new(2, "CYCLE_ALL", fl!("mode-cycle-all"), false, true),
        Mode::new(
            3,
            "CYCLE_LEFT_RIGHT",
            fl!("mode-cycle-left-right"),
            false,
            true,
        ),
        Mode::new(4, "CYCLE_UP_DOWN", fl!("mode-cycle-up-down"), false, true),
        Mode::new(5, "CYCLE_OUT_IN", fl!("mode-cycle-out-in"), false, true),
        Mode::new(
            6,
            "CYCLE_OUT_IN_DUAL",
            fl!("mode-cycle-out-in-dual"),
            false,
            true,
        ),
        Mode::new(
            7,
            "RAINBOW_MOVING_CHEVRON",
            fl!("mode-rainbow-moving-chevron"),
            false,
            true,
        ),
        Mode::new(8, "CYCLE_PINWHEEL", fl!("mode-cycle-pinwheel"), false, true),
        Mode::new(9, "CYCLE_SPIRAL", fl!("mode-cycle-spiral"), false, true),
        Mode::new(10, "RAINDROPS", fl!("mode-raindrops"), false, false),
        Mode::new(11, "SPLASH", fl!("mode-splash"), false, true),
        Mode::new(12, "MULTISPLASH", fl!("mode-multisplash"), false, true),
    ]
});
