use serde::Deserialize;

fn num_layers_default() -> u8 {
    2
}

#[derive(Debug, Deserialize)]
pub struct Meta {
    pub display_name: String,
    #[serde(default)]
    pub has_mode: bool,
    #[serde(default)]
    pub has_per_layer: bool,
    #[serde(default = "num_layers_default")]
    pub num_layers: u8,
}
