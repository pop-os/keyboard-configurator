/// Serde based deserialization for physical.json
use serde::Deserialize;

use crate::{Rect, Rgb};

pub(crate) struct PhysicalLayout {
    pub name: String,
    pub author: String,
    pub pressed_color: Rgb,
    pub keys: Vec<PhysicalLayoutKey>,
}

impl PhysicalLayout {
    pub fn from_str(physical_json: &str) -> Self {
        let json = serde_json::from_str::<PhysicalLayoutJson>(physical_json).unwrap();

        let mut keys = Vec::new();

        let mut row_i = 0;
        let mut col_i = 0;
        let mut physical = Rect::new(0.0, 0.0, 1.0, 1.0);
        let mut background_color = Rgb::new(0xcc, 0xcc, 0xcc);

        for row in json.rows {
            for i in &row.0 {
                match i {
                    PhysicalKeyEnum::Meta(meta) => {
                        debug!("Key metadata {:?}", meta);
                        physical.x += meta.x;
                        physical.y -= meta.y;
                        physical.w = meta.w.unwrap_or(physical.w);
                        physical.h = meta.h.unwrap_or(physical.h);
                        background_color = meta.c.unwrap_or(background_color);
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

        Self {
            name: json.name,
            author: json.author,
            pressed_color: json.pressed_color,
            keys,
        }
    }
}

pub(crate) struct PhysicalLayoutKey {
    pub logical: (u8, u8),
    pub physical: Rect,
    pub physical_name: String,
    pub background_color: Rgb,
}

#[derive(Debug, Deserialize)]
struct PhysicalLayoutJson {
    name: String,
    author: String,
    pressed_color: Rgb,
    rows: Vec<PhysicalRow>,
}

#[derive(Debug, Deserialize)]
struct PhysicalRow(Vec<PhysicalKeyEnum>);

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PhysicalKeyEnum {
    Name(String),
    Meta(PhysicalKeyMeta),
}

#[derive(Debug, Deserialize)]
struct PhysicalKeyMeta {
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
    w: Option<f64>,
    h: Option<f64>,
    c: Option<Rgb>,
}
