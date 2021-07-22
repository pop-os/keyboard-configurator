use crate::Rgb;
use serde::Deserialize;

fn num_layers_default() -> u8 {
    2
}

/// Metadata for keyboard
#[derive(Debug, Deserialize)]
pub struct Meta {
    /// Display name for keyboard
    pub display_name: String,
    /// Keyboard has per-key controllable LEDs supporting various modes
    #[serde(default)]
    pub has_mode: bool,
    /// LED settings are per-layer, not for the whole keyboard
    #[serde(default)]
    pub has_per_layer: bool,
    /// Has LED with brightness
    pub has_brightness: bool,
    /// Has LED with color (i.e. not monochrome)
    pub has_color: bool,
    /// Supports mod-tap bindings (assumes QMK mod-tap encoding)
    #[serde(default)]
    pub has_mod_tap: bool,
    /// Number or layers; e.g. 2 where layer 2 is used when `Fn` is held
    #[serde(default = "num_layers_default")]
    pub num_layers: u8,
    pub pressed_color: Rgb,
}
