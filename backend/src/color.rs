use ordered_float::NotNan;
use palette::{FromColor, FromComponent, IntoColor, IntoComponent, RgbHue};
use serde::{de, Deserialize, Serialize};
use std::{f64::consts::PI, fmt};

type PaletteHsv = palette::Hsv<palette::encoding::Srgb, f64>;
type PaletteLinSrgb = palette::LinSrgb<f64>;

/// Floating point hue/saturation color
#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    Default,
    PartialEq,
    glib::GBoxed,
    Hash,
    Eq,
    Ord,
    PartialOrd,
)]
#[gboxed(type_name = "S76Hs")]
pub struct Hs {
    /// Hue, in radians
    pub h: NotNan<f64>,
    /// Saturation, from 0.0 to 1.0
    pub s: NotNan<f64>,
}

impl Hs {
    pub fn new(h: f64, s: f64) -> Self {
        Self {
            h: NotNan::new(h).unwrap(),
            s: NotNan::new(s).unwrap(),
        }
    }

    pub fn from_ints(h: u8, s: u8) -> Self {
        Self::new(f64::from_component(h) * (2. * PI), f64::from_component(s))
    }

    pub fn to_ints(self) -> (u8, u8) {
        let h = (self.h / (2. * PI)).rem_euclid(1.);
        (h.into_component(), self.s.into_component())
    }

    pub fn to_rgb(self) -> Rgb {
        let hue = RgbHue::from_radians(*self.h);
        let hsv = PaletteHsv::new(hue, *self.s, 1.);
        let rgb: PaletteLinSrgb = palette::Srgb::<f64>::from_color(hsv).into_linear();
        let (r, g, b) = rgb.into_components();
        Rgb::from_floats(r, g, b)
    }
}

/// Integer RGB color
#[derive(Clone, Copy, Debug, Default, glib::GBoxed)]
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
        Self {
            r: r.into_component(),
            g: g.into_component(),
            b: b.into_component(),
        }
    }

    pub fn to_floats(self) -> (f64, f64, f64) {
        (
            self.r.into_component(),
            self.g.into_component(),
            self.b.into_component(),
        )
    }

    /// Parse from hexadecimal string
    pub fn parse(s: &str) -> Option<Self> {
        if s.len() == 7 && s.starts_with('#') {
            let r = u8::from_str_radix(&s[1..3], 16).ok()?;
            let g = u8::from_str_radix(&s[3..5], 16).ok()?;
            let b = u8::from_str_radix(&s[5..7], 16).ok()?;
            Some(Self::new(r, g, b))
        } else {
            None
        }
    }

    #[allow(clippy::many_single_char_names)]
    pub fn to_hs_lossy(self) -> Hs {
        let (r, g, b) = self.to_floats();
        let rgb = PaletteLinSrgb::new(r, g, b);
        let hsv: PaletteHsv = palette::Srgb::<f64>::from_linear(rgb).into_color();
        let (h, s, _) = hsv.into_components();
        Hs::new(h.to_radians(), s)
    }
}

/// Convert to hexadecimal string
impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl Serialize for Rgb {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

struct RgbVisitor;

impl<'de> de::Visitor<'de> for RgbVisitor {
    type Value = Rgb;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a hexadecimal rgb code prefixed with #")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Rgb, E> {
        Rgb::parse(v).ok_or_else(|| E::invalid_value(de::Unexpected::Str(v), &self))
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(RgbVisitor)
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn test_hs_rgb_hs() {
        let hs1 = Hs::new(0.3, 0.4);
        let hs2 = hs1.to_rgb().to_hs_lossy();
        let hs3 = hs2.to_rgb().to_hs_lossy();
        assert!((hs1.h - hs2.h).abs() < 0.01);
        assert!((hs1.s - hs2.s).abs() < 0.01);
        assert!((hs2.h - hs3.h).abs() < 0.0001);
        assert!((hs2.s - hs3.s).abs() < 0.0001);
    }
}
