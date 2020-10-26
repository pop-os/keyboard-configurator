/// Serde based deserialization for physical.json
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct PhysicalLayout(pub Vec<PhysicalLayoutEntry>);

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum PhysicalLayoutEntry {
    Meta(PhysicalLayoutMeta),
    Row(PhysicalRow),
}

#[derive(Debug, Deserialize)]
pub(crate) struct PhysicalLayoutMeta {
    pub name: String,
    pub author: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PhysicalRow(pub Vec<PhysicalKeyEnum>);

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum PhysicalKeyEnum {
    Name(String),
    Meta(PhysicalKeyMeta),
}

#[derive(Debug, Deserialize)]
pub(crate) struct PhysicalKeyMeta {
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    pub w: Option<f64>,
    pub h: Option<f64>,
    pub c: Option<String>,
    pub t: Option<String>,
}
