use palette::{Component, IntoColor, RgbHue};
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug)]
pub struct Hs {
    /// Hue, in radians
    pub h: f64,
    /// Saturation, from 0.0 to 1.0
    pub s: f64,
}

impl Hs {
    pub fn new(h: f64, s: f64) -> Self {
        Self { h, s }
    }

    pub fn to_rgb(self) -> Rgb {
        let hue = RgbHue::from_radians(self.h);
        let hsv = palette::Hsv::new(hue, self.s, 1.);
        let rgb = hsv.into_rgb::<palette::encoding::srgb::Srgb>();
        let (r, g, b) = rgb.into_format::<u8>().into_components();
        Rgb::new(r, g, b)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, glib::GBoxed)]
#[gboxed(type_name = "S76Rgb")]
pub struct Rgb {
    /// Red
    pub r: u8,
    /// Green
    pub g: u8,
    /// Blue
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_floats(r: f64, g: f64, b: f64) -> Self {
        Self { r: r.convert(), g: g.convert(), b: b.convert() }
    }

    pub fn to_floats(self) -> (f64, f64, f64) {
        (self.r.convert(), self.g.convert(), self.b.convert())
    }

    pub fn to_string(self) -> String {
        format!("{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn parse(s: &str) -> Option<Self> {
        if s.len() == 6 {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some(Self::new(r, g, b))
        } else {
            None
        }
    }

    pub fn to_hs_lossy(self) -> Hs {
        let rgb = palette::Srgb::new(self.r, self.g, self.b);
        let rgb = rgb.into_format::<f64>();
        let hsv = rgb.into_hsv::<palette::encoding::srgb::Srgb>();
        let (h, s, _) = hsv.into_components();
        Hs::new(h.to_radians(), s)
    }
}
