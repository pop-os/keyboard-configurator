/// Serde based deserialization for physical.json
use serde::Deserialize;

use crate::{Rect, Rgb};

pub(crate) struct PhysicalLayoutKey {
    pub logical: (u8, u8),
    pub physical: Rect,
    pub physical_name: String,
    pub background_color: Rgb,
}

#[derive(Debug, Deserialize)]
pub struct PhysicalLayout(pub Vec<PhysicalLayoutEntry>);

impl PhysicalLayout {
    pub(crate) fn keys(&self) -> Vec<PhysicalLayoutKey> {
        let mut keys = Vec::new();

        let mut row_i = 0;
        let mut col_i = 0;
        let mut physical = Rect::new(0.0, 0.0, 1.0, 1.0);
        let mut background_color = Rgb::new(0xcc, 0xcc, 0xcc);

        for entry in &self.0 {
            if let PhysicalLayoutEntry::Row(row) = entry {
                for i in &row.0 {
                    match i {
                        PhysicalKeyEnum::Meta(meta) => {
                            debug!("Key metadata {:?}", meta);
                            physical.x += meta.x;
                            physical.y -= meta.y;
                            physical.w = meta.w.unwrap_or(physical.w);
                            physical.h = meta.h.unwrap_or(physical.h);
                            background_color = meta
                                .c
                                .as_ref()
                                .map(|c| {
                                    let err = format!("Failed to parse color {}", c);
                                    Rgb::parse(&c[1..]).expect(&err)
                                })
                                .unwrap_or(background_color);
                        }
                        PhysicalKeyEnum::Name(name) => {
                            keys.push(PhysicalLayoutKey {
                                logical: (row_i as u8, col_i as u8),
                                physical,
                                physical_name: name.clone(),
                                background_color,
                            });

                            physical.x += physical.w;

                            physical.w = 1.0;
                            physical.h = 1.0;

                            col_i += 1;
                        }
                    }
                }

                physical.x = 0.0;
                physical.y -= 1.0;

                col_i = 0;
                row_i += 1;
            }
        }

        keys
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PhysicalLayoutEntry {
    Meta(PhysicalLayoutMeta),
    Row(PhysicalRow),
}

#[derive(Debug, Deserialize)]
pub struct PhysicalLayoutMeta {
    pub name: String,
    pub author: String,
}

#[derive(Debug, Deserialize)]
pub struct PhysicalRow(pub Vec<PhysicalKeyEnum>);

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PhysicalKeyEnum {
    Name(String),
    Meta(PhysicalKeyMeta),
}

#[derive(Debug, Deserialize)]
pub struct PhysicalKeyMeta {
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    pub w: Option<f64>,
    pub h: Option<f64>,
    pub c: Option<String>,
}
